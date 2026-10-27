use super::*;
use serde_json::json;
use tinytools::Tool;

#[tokio::test]
async fn missing_hash_is_error() {
    let tool = retrieve_tool_output_tool();
    let res2 = tool.execute(json!({})).await.unwrap();
    assert!(res2.is_error);
}

#[tokio::test]
async fn an_unreachable_store_is_reported_as_a_tool_error() {
    let tool = retrieve_tool_output_tool();
    let res = tool
        .execute(json!({ "hash": "deadbeefcafe" }))
        .await
        .unwrap();
    assert!(res.is_error);
}

fn echoing_tool() -> RetrieveToolOutputTool {
    RetrieveToolOutputTool::new(Arc::new(|key| {
        Box::pin(async move { Ok(Some(format!("original for {key}"))) })
    }))
}

#[tokio::test]
async fn the_advertised_token_key_retrieves_the_original() {
    let schema = echoing_tool().parameters_schema();
    assert_eq!(schema["required"], json!(["token"]));
    let res = echoing_tool()
        .execute(json!({ "token": "abc123" }))
        .await
        .unwrap();
    assert!(!res.is_error);
    assert_eq!(res.output(), "original for abc123");
}

#[tokio::test]
async fn the_legacy_hash_key_still_retrieves_the_original() {
    let res = echoing_tool()
        .execute(json!({ "hash": "abc123" }))
        .await
        .unwrap();
    assert!(!res.is_error);
    assert_eq!(res.output(), "original for abc123");
}
