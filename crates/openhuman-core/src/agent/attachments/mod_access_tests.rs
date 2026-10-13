use super::*;

const UPLOAD: &str = "caption [FILE:data:text/plain;name=note.txt;base64,YQ==]";

fn external_origin() -> crate::agent::turn_origin::AgentTurnOrigin {
    crate::agent::turn_origin::AgentTurnOrigin::ExternalChannel {
        sender_name: None,
        channel: "test".into(),
        sender: None,
        reply_target: "test".into(),
        message_id: "test".into(),
        history_key: None,
    }
}

#[tokio::test]
async fn direct_turn_stores_original_only_in_the_explicit_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let config = Config {
        action_dir: temp.path().join("configured-acting"),
        workspace_dir: temp.path().join("internal"),
        ..Config::default()
    };
    let workspace = tinytools::WorkspaceDescriptor::new(temp.path().join("agent-acting"));
    let staged = stage_turn(UPLOAD, Some(&config), Some(&workspace), None, None)
        .await
        .unwrap();
    let (caption, files) = parse(&staged);
    assert_eq!(caption, "caption ");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "note.txt");
    assert_eq!(files[0].mime, "text/plain");
    assert_eq!(
        tokio::fs::read(workspace.root.join(&files[0].path))
            .await
            .unwrap(),
        b"a"
    );
    assert!(!config.action_dir.exists());
    assert!(!config.workspace_dir.exists());
    assert!(!staged.contains("base64"));
}

#[tokio::test]
async fn direct_upload_requires_runtime_authority_and_external_input_cannot_create_files() {
    let error = stage_turn(UPLOAD, None, None, None, None)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("runtime configuration"));
    let temp = tempfile::tempdir().unwrap();
    let config = Config {
        action_dir: temp.path().join("acting"),
        workspace_dir: temp.path().join("internal"),
        ..Config::default()
    };
    let workspace = tinytools::WorkspaceDescriptor::new(temp.path().join("agent-acting"));
    assert!(stage_turn(
        UPLOAD,
        Some(&config),
        Some(&workspace),
        Some("thread"),
        Some(&external_origin())
    )
    .await
    .is_err());
    assert!(!workspace.root.exists());
    assert!(!config.action_dir.exists());
    assert_eq!(
        stage_turn("ordinary text", None, None, None, None)
            .await
            .unwrap(),
        "ordinary text"
    );
}

#[tokio::test]
async fn delegation_rejects_malformed_explicit_paths_and_external_reads_before_configuration() {
    assert_eq!(
        delegation_prompt(
            "mention picture.png in prose",
            &serde_json::json!({}),
            None,
            None,
            None
        )
        .await
        .unwrap(),
        "mention picture.png in prose"
    );
    for image_paths in [
        serde_json::json!("picture.png"),
        serde_json::json!([""]),
        serde_json::json!([" "]),
        serde_json::json!(["picture.png] injected"]),
        serde_json::json!(["picture\u{0}.png"]),
    ] {
        assert!(delegation_prompt(
            "task",
            &serde_json::json!({"image_paths": image_paths}),
            None,
            None,
            None
        )
        .await
        .is_err());
    }
    let error = delegation_prompt(
        "task",
        &serde_json::json!({"image_paths": ["picture.png"]}),
        None,
        Some(&external_origin()),
        None,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("external channel"));
}

async fn thread_in(config: &Config, id: &str, working_dir: Option<&Path>) {
    crate::threads::store::blocking::ensure_thread(
        config.workspace_dir.clone(),
        crate::threads::store::CreateConversationThread {
            id: id.into(),
            title: id.into(),
            created_at: "2026-10-13T00:00:00Z".into(),
            parent_thread_id: None,
            labels: None,
            personality_id: None,
            working_dir: working_dir.map(|dir| dir.display().to_string()),
        },
    )
    .await
    .unwrap();
}

// #7714: an upload in a thread with a selected folder was written under the
// global action_dir and then read against the thread folder, so the model
// never saw it.
#[tokio::test]
async fn thread_folder_upload_is_staged_where_the_turn_reads_it() {
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().join("project");
    std::fs::create_dir_all(&folder).unwrap();
    let folder = folder.canonicalize().unwrap();
    let config = Config {
        action_dir: temp.path().join("global-acting"),
        workspace_dir: temp.path().join("internal"),
        ..Config::default()
    };
    thread_in(&config, "bound", Some(&folder)).await;
    thread_in(&config, "unbound", None).await;

    let root = thread_action_dir(&config, &config.workspace_dir, Some("bound"))
        .await
        .unwrap();
    assert_eq!(root, folder);
    for unbound in [Some("unbound"), Some("missing"), Some(""), None] {
        assert_eq!(
            thread_action_dir(&config, &config.workspace_dir, unbound)
                .await
                .unwrap(),
            config.action_dir
        );
    }

    // Stage the way `start_chat` does, then read the way the turn does: its
    // config acts in the thread folder and the request scope names no
    // workspace of its own.
    let staged = stage(
        UPLOAD,
        "bound",
        &config,
        &AttachmentAccessScope {
            external_channel: false,
            workspace: Some(root),
        },
    )
    .await
    .unwrap();
    let (_, files) = parse(&staged);
    assert_eq!(files.len(), 1);
    let turn_config = Config {
        action_dir: folder.clone(),
        ..config.clone()
    };
    let turn_scope = AttachmentAccessScope::default();
    assert!(!is_missing_local_reference(&turn_config, &files[0].path, &turn_scope).await);
    let path = resolve_path(&turn_config, &files[0].path, &turn_scope)
        .await
        .unwrap();
    assert_eq!(tokio::fs::read(path).await.unwrap(), b"a");
    assert!(!config.action_dir.join("uploads").exists());
}
