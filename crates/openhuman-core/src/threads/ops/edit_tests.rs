use super::*;

use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};
use crate::threads::store::{self, CreateConversationThread};
use crate::threads::{AppendConversationMessageRequest, ConversationMessageRecord};
use tinychannels_bus::{ContentPart, MediaReference};

#[tokio::test]
async fn invalid_edit_attachment_leaves_the_original_message_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = temp.path().join("workspace");
    config.action_dir = temp.path().join("acting");
    let ctx = DispatchCtx::new(CoreContext::for_test_with_config(
        DomainSet::full(),
        config.clone(),
    ));
    store::blocking::ensure_thread(
        ctx.clone(),
        config.workspace_dir.clone(),
        CreateConversationThread {
            id: "edit-attachments".into(),
            title: "Edit attachments".into(),
            created_at: "2026-10-20T00:00:00Z".into(),
            parent_thread_id: None,
            labels: None,
            personality_id: None,
            working_dir: None,
        },
    )
    .await
    .unwrap();
    crate::threads::ops::message_append(
        &ctx,
        AppendConversationMessageRequest {
            thread_id: "edit-attachments".into(),
            message: ConversationMessageRecord {
                id: "user-original".into(),
                content: "original prompt".into(),
                attachments: Vec::new(),
                message_type: "text".into(),
                extra_metadata: serde_json::json!({}),
                sender: "user".into(),
                created_at: "2026-10-20T00:00:01Z".into(),
            },
            attachments: Vec::new(),
        },
    )
    .await
    .unwrap();

    let result = edit_message(
        &ctx,
        EditMessageRequest {
            thread_id: "edit-attachments".into(),
            message_id: "user-original".into(),
            content: "replacement prompt".into(),
            attachments: vec![ContentPart::Image {
                media: MediaReference {
                    path: Some("/etc/passwd".into()),
                    ..Default::default()
                },
                caption: None,
                alt_text: None,
            }],
            client_id: Some("test-client".into()),
        },
    )
    .await;
    assert!(result.is_err());

    let rows = store::blocking::get_messages(ctx, config.workspace_dir, "edit-attachments".into())
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "user-original");
    assert_eq!(rows[0].content, "original prompt");
}
