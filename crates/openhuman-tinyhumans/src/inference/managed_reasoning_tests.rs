use super::*;
use tinyinference_llm::message::Message;

#[test]
fn without_reasoning_marks_the_request_without_rewriting_other_options() {
    let options = serde_json::json!({ "transport": { "trace": true } });
    let mut request =
        ModelRequest::new(vec![Message::user("hi")]).with_provider_options(options.clone());
    request.metadata = serde_json::json!({ "caller": "kept" });

    let updated = without_reasoning(request);

    assert_eq!(updated.metadata["caller"], "kept");
    assert_eq!(updated.metadata[REASONING_OFF_METADATA_KEY], true);
    assert_eq!(updated.provider_options, options);
}
