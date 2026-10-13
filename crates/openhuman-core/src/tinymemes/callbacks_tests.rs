use super::*;
use serde_json::json;

fn config_in(dir: &std::path::Path) -> Config {
    let mut config = Config::default();
    config.config_path = dir.join("config.toml");
    config.workspace_dir = dir.join("workspace");
    config.secrets.encrypt = false;
    config
}

fn decode<T: DeserializeOwned>(value: Value) -> HostResult<T> {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn an_index_saved_through_the_host_loads_back() {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::new(&config_in(dir.path()));

    let missing = dispatch(&session, host_methods::LOAD_INDEX, json!([{ "kind": "slang" }]))
        .await
        .unwrap();
    assert_eq!(decode::<Option<String>>(missing), Ok(None));

    let saved = dispatch(
        &session,
        host_methods::SAVE_INDEX,
        json!([{ "kind": "slang", "json": "{\"terms\":1}" }]),
    )
    .await
    .unwrap();
    assert_eq!(decode::<()>(saved), Ok(()));

    let loaded = dispatch(&session, host_methods::LOAD_INDEX, json!([{ "kind": "slang" }]))
        .await
        .unwrap();
    assert_eq!(decode::<Option<String>>(loaded), Ok(Some("{\"terms\":1}".into())));
    // The other index is a different document.
    let memes = dispatch(&session, host_methods::LOAD_INDEX, json!([{ "kind": "memes" }]))
        .await
        .unwrap();
    assert_eq!(decode::<Option<String>>(memes), Ok(None));
    assert!(dir
        .path()
        .join("workspace/tinymemes/slang-index.json")
        .is_file());
}

#[tokio::test]
async fn without_managed_jev_evaluate_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::new(&config_in(dir.path()));
    let request = json!([{ "body": { "questions": {}, "messages": [] } }]);
    let result = dispatch(&session, host_methods::EVALUATE, request).await.unwrap();
    assert_eq!(
        decode::<EvaluateEnvelope>(result),
        Err(HostFailure::Unsupported)
    );
}

#[tokio::test]
async fn search_switched_off_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = config_in(dir.path());
    config.search.enabled = Some(false);
    let session = Session::new(&config);
    let result = dispatch(
        &session,
        host_methods::SEARCH,
        json!([{ "query": "q", "max_results": 3 }]),
    )
    .await
    .unwrap();
    assert_eq!(
        decode::<Vec<tinymemes_bus::SearchResult>>(result),
        Err(HostFailure::Unsupported)
    );
}

#[tokio::test]
async fn deliver_is_acknowledged_and_unknown_members_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::new(&config_in(dir.path()));
    let delivered = dispatch(
        &session,
        host_methods::DELIVER,
        json!([{ "reply": "hi", "remixed": true }]),
    )
    .await
    .unwrap();
    assert_eq!(decode::<()>(delivered), Ok(()));
    assert!(dispatch(&session, "Nope", json!([])).await.is_err());
    assert!(dispatch(&session, host_methods::COMPLETE, json!("bad"))
        .await
        .is_err());
}

#[tokio::test]
async fn callbacks_answer_only_while_a_remix_runs() {
    let dir = tempfile::tempdir().unwrap();
    let config = config_in(dir.path());
    assert!(active().is_none());
    let seen = run_with(&config, async { active().is_some() }).await;
    assert!(seen);
    assert!(active().is_none(), "the session is cleared when the remix ends");
}
