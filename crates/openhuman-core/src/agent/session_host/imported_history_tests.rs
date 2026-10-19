use super::*;

use std::sync::Arc;
use tinyagents_session::transcript::TranscriptMessage;
use tinyinference_llm::message::Message;
use tinyinference_llm::model::ChatModel;

#[test]
fn imported_history_gets_the_real_prefix_and_freezes_it_after_first_turn() {
    std::thread::Builder::new()
        .stack_size(crate::core::runtime::AGENT_WORKER_STACK_BYTES)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("test runtime")
                .block_on(run_imported_history_prefix_regression());
        })
        .expect("test thread")
        .join()
        .expect("test thread panicked");
}

async fn run_imported_history_prefix_regression() {
    let root = tempfile::tempdir().expect("workspace");
    let locator = crate::agent::session_store::splice_fixture::memory_locator();
    let tool = || {
        Box::new(tinyagents_harness::testkit::FakeTool::returning(
            "relay_history_probe",
            "ok",
        )) as Box<dyn tinytools::Tool>
    };
    let first_model = Arc::new(
        tinyagents_harness::testkit::ScriptedModel::replies(vec!["first reply"]).with_profile(
            tinyinference_llm::model::ModelProfile {
                tool_calling: true,
                ..Default::default()
            },
        ),
    );
    let first_chat_model: Arc<dyn ChatModel<()>> = first_model.clone();
    let mut first = SessionHostBuilder::new()
        .with_session_history_locator(Arc::clone(&locator))
        .chat_model(first_chat_model)
        .tools(vec![tool()])
        .prompt_builder(crate::agent::prompts::SystemPromptBuilder::verbatim(
            "You are the relay assistant. Preserve this exact system prefix.".to_owned(),
        ))
        .workspace_dir(root.path().join("workspace"))
        .action_dir(root.path().to_path_buf())
        .tool_dispatcher(Box::new(tinytools_agent::dialect::NativeDialect))
        .build()
        .expect("first session");
    first.set_thread_id(Some("relay-import-thread"));
    let imported = [
        TranscriptMessage::user("legacy relay question"),
        TranscriptMessage::assistant("legacy relay answer"),
    ];
    assert!(first.adopt_imported_history(&imported).unwrap());
    assert!(!first.adopt_imported_history(&imported).unwrap());
    assert_eq!(
        first.turn("new relay question").await.unwrap(),
        "first reply"
    );
    let actual_agent_id = first.agent_definition_id.clone();

    let first_request = first_model
        .requests()
        .into_iter()
        .next()
        .expect("first model request");
    let first_messages = &first_request.messages;
    let prefix: Vec<_> = first_messages
        .iter()
        .filter(|message| matches!(message, Message::System(_)))
        .map(Message::text)
        .collect();
    assert!(!prefix.is_empty(), "the configured system prefix is sent");
    assert!(first_request
        .tools
        .iter()
        .any(|tool| tool.name == "relay_history_probe"));
    let legacy_question = first_messages
        .iter()
        .position(|message| matches!(message, Message::User(_) if message.text() == "legacy relay question"))
        .expect("imported user message reaches first model request");
    let legacy_answer = first_messages
        .iter()
        .position(|message| matches!(message, Message::Assistant(_) if message.text() == "legacy relay answer"))
        .expect("imported assistant message reaches first model request");
    let current_question = first_messages
        .iter()
        .position(|message| matches!(message, Message::User(_) if message.text().contains("new relay question")))
        .expect("current user message reaches first model request");
    assert!(
        first_messages
            .iter()
            .position(|message| matches!(message, Message::System(_)))
            .expect("system prefix")
            < legacy_question
            && legacy_question < legacy_answer
            && legacy_answer < current_question,
        "the live prefix precedes imported rows, followed by this turn"
    );
    let transcript = locator
        .read_session_transcript(&tinyagents_session::transcript::SessionRef::scoped(
            "relay-import-thread",
            &actual_agent_id,
        ))
        .expect("adopted session transcript")
        .read_session()
        .unwrap()
        .unwrap();
    assert!(transcript.meta.prefix_message_count.unwrap_or_default() > 0);
    assert!(transcript
        .tools
        .as_ref()
        .is_some_and(|tools| tools.to_string().contains("relay_history_probe")));
    drop(first);

    let second_model = Arc::new(
        tinyagents_harness::testkit::ScriptedModel::replies(vec!["second reply"]).with_profile(
            tinyinference_llm::model::ModelProfile {
                tool_calling: true,
                ..Default::default()
            },
        ),
    );
    let second_chat_model: Arc<dyn ChatModel<()>> = second_model.clone();
    let mut second = SessionHostBuilder::new()
        .with_session_history_locator(locator)
        .chat_model(second_chat_model)
        .tools(vec![tool()])
        .prompt_builder(crate::agent::prompts::SystemPromptBuilder::verbatim(
            "This replacement prompt must not change an existing session.".to_owned(),
        ))
        .workspace_dir(root.path().join("workspace"))
        .action_dir(root.path().to_path_buf())
        .tool_dispatcher(Box::new(tinytools_agent::dialect::NativeDialect))
        .build()
        .expect("resumed session");
    second.set_thread_id(Some("relay-import-thread"));
    assert_eq!(second.turn("after restart").await.unwrap(), "second reply");
    let second_request = second_model
        .requests()
        .into_iter()
        .next()
        .expect("resumed model request");
    let second_prefix: Vec<_> = second_request
        .messages
        .iter()
        .filter(|message| matches!(message, Message::System(_)))
        .map(Message::text)
        .collect();
    assert_eq!(second_prefix, prefix, "resume freezes the committed prompt");
    assert!(second_prefix
        .iter()
        .any(|message| message.contains("Preserve this exact system prefix")));
    assert!(!second_prefix
        .iter()
        .any(|message| message.contains("replacement prompt")));
    assert_eq!(
        second_request.tools, first_request.tools,
        "resume restores the committed tool snapshot"
    );
    assert!(second_request
        .tools
        .iter()
        .any(|tool| tool.name == "relay_history_probe"));
    let resumed_history = serde_json::to_string(&second_request.messages).unwrap();
    assert!(resumed_history.contains("legacy relay question"));
    assert!(resumed_history.contains("legacy relay answer"));
    assert!(resumed_history.contains("new relay question"));
    assert!(resumed_history.contains("first reply"));
}
