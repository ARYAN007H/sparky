use super::*;
use base64::Engine as _;
use serde_json::json;

fn valid_image_data_uri() -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(include_bytes!(
            "../../../../../tests/fixtures/images/handoff-hero.png"
        ))
    )
}

async fn setup() -> (tempfile::TempDir, crate::config::Config) {
    let temp = tempfile::tempdir().unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = temp.path().join("internal");
    config.action_dir = temp.path().join("acting");
    conversations::blocking::ensure_thread(
        crate::core::runtime::DispatchCtx::ambient(),
        config.workspace_dir.clone(),
        conversations::CreateConversationThread {
            id: "uploads".into(),
            title: "Uploads".into(),
            created_at: "2026-10-04T00:00:00Z".into(),
            parent_thread_id: None,
            labels: None,
            personality_id: None,
            working_dir: None,
        },
    )
    .await
    .unwrap();
    (temp, config)
}

async fn append_with_test_config(
    request: AppendConversationMessageRequest,
    config: &crate::config::Config,
    origin: Option<&crate::agent::turn_origin::AgentTurnOrigin>,
) -> Result<Outcome<ApiEnvelope<ConversationMessageRecord>>, ThreadsError> {
    message_append_with_config(
        &crate::core::runtime::DispatchCtx::ambient(),
        request,
        config,
        origin,
        &config.workspace_dir,
    )
    .await
}

fn request(id: &str, content: &str, sender: &str) -> AppendConversationMessageRequest {
    AppendConversationMessageRequest {
        thread_id: "uploads".into(),
        message: ConversationMessageRecord {
            id: id.into(),
            content: content.into(),
            attachments: vec![],
            message_type: "text".into(),
            extra_metadata: json!({"attachmentNames":["image.png"], "attachmentKinds":["image"],
                "attachmentDataUris":["data:image/png;base64,iVBORw0KGgo="], "attachmentData":"bytes",
                "attachmentPosters":["data:image/png;base64,iVBORw0KGgo="],
                "attachmentPreview":"data:image/png;base64,iVBORw0KGgo=", "other":"kept"}),
            sender: sender.into(),
            created_at: "2026-10-04T00:00:00Z".into(),
        },
        attachments: vec![],
    }
}

#[tokio::test]
async fn user_append_stages_original_strips_upload_bytes_and_reuses_returned_content() {
    let (_temp, config) = setup().await;
    let data_uri = valid_image_data_uri();
    let first = append_with_test_config(
        request("one", &format!("look [IMAGE:{data_uri}]"), "user"),
        &config,
        None,
    )
    .await
    .unwrap()
    .value
    .data
    .unwrap();
    let (_, files) = crate::agent::attachments::parse(&first.content);
    assert_eq!(files.len(), 1);
    assert_eq!(
        tokio::fs::read(config.action_dir.join(&files[0].path))
            .await
            .unwrap(),
        include_bytes!("../../../../../tests/fixtures/images/handoff-hero.png")
    );
    assert!(first.extra_metadata.get("attachmentDataUris").is_none());
    assert!(first.extra_metadata.get("attachmentData").is_none());
    assert!(first.extra_metadata.get("attachmentPosters").is_none());
    assert!(first.extra_metadata.get("attachmentPreview").is_none());
    assert_eq!(
        first.extra_metadata["attachmentNames"],
        json!(["image.png"])
    );
    assert_eq!(first.extra_metadata["attachmentKinds"], json!(["image"]));
    assert_eq!(first.extra_metadata["other"], "kept");
    let second = append_with_test_config(request("two", &first.content, "user"), &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();
    assert_eq!(second.content, first.content);
    let staged = crate::agent::attachments::stage_turn(
        &second.content,
        Some(&config),
        None,
        Some("uploads"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(staged, first.content);
    let persisted = conversations::blocking::get_messages(
        crate::core::runtime::DispatchCtx::ambient(),
        config.workspace_dir.clone(),
        "uploads".into(),
    )
    .await
    .unwrap();
    assert_eq!(persisted.len(), 2);
    assert!(
        !serde_json::to_string(&persisted)
            .unwrap()
            .contains("base64,")
    );
}

#[tokio::test]
async fn rejected_upload_never_appends_a_message() {
    let (_temp, mut config) = setup().await;
    config.multimodal_files.max_files = 0;
    assert!(
        append_with_test_config(
            request(
                "disabled",
                "[IMAGE:data:image/png;base64,iVBORw0KGgo=]",
                "user"
            ),
            &config,
            None,
        )
        .await
        .is_err()
    );
    config.multimodal_files.max_files = 3;
    assert!(
        append_with_test_config(
            request("bad", "[FILE:data:application/zip;base64,!]", "user"),
            &config,
            None,
        )
        .await
        .is_err()
    );
    assert!(
        append_with_test_config(request("missing", "image.png", "user"), &config, None)
            .await
            .is_err()
    );
    assert!(
        conversations::blocking::get_messages(
            crate::core::runtime::DispatchCtx::ambient(),
            config.workspace_dir.clone(),
            "uploads".into()
        )
        .await
        .unwrap()
        .is_empty()
    );
}

#[tokio::test]
async fn external_origin_cannot_reuse_an_existing_attachment_marker() {
    let (_temp, config) = setup().await;
    let staged = crate::agent::attachments::stage_turn(
        "[IMAGE:data:image/png;base64,iVBORw0KGgo=]",
        Some(&config),
        None,
        Some("uploads"),
        None,
    )
    .await
    .unwrap();
    let origin = crate::agent::turn_origin::AgentTurnOrigin::ExternalChannel {
        sender_name: None,
        channel: "test".into(),
        sender: None,
        reply_target: "room".into(),
        message_id: "message".into(),
        history_key: None,
    };
    assert!(
        append_with_test_config(request("external", &staged, "user"), &config, Some(&origin))
            .await
            .is_err()
    );
    assert!(
        conversations::blocking::get_messages(
            crate::core::runtime::DispatchCtx::ambient(),
            config.workspace_dir,
            "uploads".into()
        )
        .await
        .unwrap()
        .is_empty()
    );
}

#[tokio::test]
async fn ordinary_user_text_and_agent_images_are_preserved() {
    let (_temp, config) = setup().await;
    let mut user = request("text", "ordinary text", "user");
    user.message.extra_metadata = json!({"other":"kept"});
    let stored = append_with_test_config(user, &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();
    assert_eq!(stored.content, "ordinary text");
    assert_eq!(stored.extra_metadata, json!({"other":"kept"}));
    let assistant = request(
        "output",
        "[IMAGE:data:image/png;base64,iVBORw0KGgo=]",
        "assistant",
    );
    let expected = serde_json::to_value(&assistant.message).unwrap();
    let stored = append_with_test_config(assistant, &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();
    assert_eq!(serde_json::to_value(stored).unwrap(), expected);
}

#[tokio::test]
async fn append_uses_host_persistence_workspace_separate_from_rpc_config() {
    let (temp, config) = setup().await;
    let persistence_dir = temp.path().join("host-thread-store");
    conversations::blocking::ensure_thread(
        crate::core::runtime::DispatchCtx::ambient(),
        persistence_dir.clone(),
        conversations::CreateConversationThread {
            id: "uploads".into(),
            title: "Host thread store".into(),
            created_at: "2026-10-04T00:00:00Z".into(),
            parent_thread_id: None,
            labels: None,
            personality_id: None,
            working_dir: None,
        },
    )
    .await
    .unwrap();

    message_append_with_config(
        &crate::core::runtime::DispatchCtx::ambient(),
        request("host-store", "saved in host workspace", "assistant"),
        &config,
        None,
        &persistence_dir,
    )
    .await
    .unwrap();

    let persisted = conversations::blocking::get_messages(
        crate::core::runtime::DispatchCtx::ambient(),
        persistence_dir,
        "uploads".into(),
    )
    .await
    .unwrap();
    assert_eq!(persisted.len(), 1);
    assert!(
        conversations::blocking::get_messages(
            crate::core::runtime::DispatchCtx::ambient(),
            config.workspace_dir,
            "uploads".into()
        )
        .await
        .unwrap()
        .is_empty()
    );
}

#[tokio::test]
async fn poster_only_user_metadata_is_rejected_before_persistence() {
    let (_temp, config) = setup().await;
    let mut upload = request("poster", "video.mp4", "user");
    upload.message.extra_metadata = json!({
        "attachmentNames": ["video.mp4"],
        "attachmentPosters": ["data:image/png;base64,iVBORw0KGgo="]
    });
    assert!(
        append_with_test_config(upload, &config, None)
            .await
            .is_err()
    );
    assert!(
        conversations::blocking::get_messages(
            crate::core::runtime::DispatchCtx::ambient(),
            config.workspace_dir.clone(),
            "uploads".into()
        )
        .await
        .unwrap()
        .is_empty()
    );
}

#[tokio::test]
async fn raw_append_stages_in_the_bound_thread_folder() {
    let (temp, config) = setup().await;
    let folder = temp.path().join("project");
    std::fs::create_dir_all(&folder).unwrap();
    let folder = folder.canonicalize().unwrap();
    conversations::blocking::ensure_thread(
        crate::core::runtime::DispatchCtx::ambient(),
        config.workspace_dir.clone(),
        conversations::CreateConversationThread {
            id: "bound".into(),
            title: "Bound".into(),
            created_at: "2026-10-04T00:00:00Z".into(),
            parent_thread_id: None,
            labels: None,
            personality_id: None,
            working_dir: Some(folder.to_string_lossy().into_owned()),
        },
    )
    .await
    .unwrap();
    let mut raw = request("raw", "[IMAGE:data:image/png;base64,iVBORw0KGgo=]", "user");
    raw.thread_id = "bound".into();
    let staged = append_with_test_config(raw, &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();
    let (_, files) = crate::agent::attachments::parse(&staged.content);
    assert_eq!(files.len(), 1);
    assert!(folder.join(&files[0].path).is_file());
    assert!(!config.action_dir.join("uploads").exists());
}

#[tokio::test]
async fn append_cannot_forge_private_attachment_storage_metadata() {
    let (_temp, config) = setup().await;
    let mut req = request("forged", "hello", "assistant");
    req.message.extra_metadata = json!({});
    req.message.extra_metadata["__openhumanStoredAttachments"] = json!([{
        "part": {"image": {"media": {"opaqueId": "attacker"}, "caption": null, "altText": null}},
        "path": "../../etc/passwd"
    }]);

    let saved = append_with_test_config(req, &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();
    assert!(saved.attachments.is_empty());
    assert!(
        saved
            .extra_metadata
            .get("__openhumanStoredAttachments")
            .is_none()
    );
    let mut user_req = request("forged-user", "hello", "user");
    user_req.message.extra_metadata = json!({});
    user_req.message.extra_metadata["__openhumanStoredAttachments"] = json!([{
        "part": {"image": {"media": {"opaqueId": "attacker"}, "caption": null, "altText": null}},
        "path": "../../etc/passwd"
    }]);
    let saved_user = append_with_test_config(user_req, &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();
    assert!(saved_user.attachments.is_empty());
    assert!(
        saved_user
            .extra_metadata
            .get("__openhumanStoredAttachments")
            .is_none()
    );
    let stored = crate::threads::store::get_messages(
        &crate::core::runtime::DispatchCtx::ambient(),
        config.workspace_dir.clone(),
        "uploads",
    )
    .unwrap();
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().all(|message| {
        message
            .extra_metadata
            .get("__openhumanStoredAttachments")
            .is_none()
    }));

    let update = UpdateConversationMessageRequest {
        thread_id: "uploads".into(),
        message_id: "forged".into(),
        extra_metadata: Some(json!({"__openhumanStoredAttachments": [{
            "part": {"image": {"media": {"opaqueId": "attacker"}, "caption": null, "altText": null}},
            "path": "../../etc/passwd"
        }]})),
    };
    let update_error = message_update(&crate::core::runtime::DispatchCtx::ambient(), update)
        .await
        .expect_err("metadata updates must not create resolver authority");
    assert!(
        update_error.contains("reserved"),
        "expected reserved-key rejection, got: {update_error}"
    );
}

#[tokio::test]
async fn typed_attachment_is_staged_like_a_marker_and_leaves_the_text_clean() {
    use tinychannels_bus::{ContentPart, MediaReference};
    let (_temp, config) = setup().await;
    let mut req = request("typed", "look", "user");
    req.attachments = vec![ContentPart::Image {
        media: MediaReference {
            url: Some(format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(include_bytes!(
                    "../../../../../tests/fixtures/images/handoff-hero.png"
                ))
            )),
            ..Default::default()
        },
        caption: None,
        alt_text: None,
    }];
    let saved = append_with_test_config(req, &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();
    assert_eq!(
        saved.content, "look",
        "modern typed image uploads must not persist a staging marker"
    );
    assert_eq!(saved.content, "look");
    assert_eq!(saved.attachments.len(), 1);
    assert!(
        saved
            .extra_metadata
            .get("__openhumanStoredAttachments")
            .is_none()
    );
    let opaque_id = match &saved.attachments[0] {
        ContentPart::Image { media, .. } => {
            assert!(media.path.is_none());
            assert!(media.url.is_none());
            assert_eq!(media.content_type.as_deref(), Some("image/png"));
            media.opaque_id.as_deref().expect("opaque attachment id")
        }
        other => panic!("expected image attachment, got {other:?}"),
    };
    let stored = crate::threads::store::get_messages(
        &crate::core::runtime::DispatchCtx::ambient(),
        config.workspace_dir.clone(),
        "uploads",
    )
    .unwrap()
    .into_iter()
    .find(|message| message.id == "typed")
    .unwrap();
    let entries = stored.extra_metadata["__openhumanStoredAttachments"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["part"]["media"]["opaqueId"], opaque_id);
    let path = entries[0]["path"].as_str().unwrap();
    assert_eq!(
        tokio::fs::read(config.action_dir.join(path)).await.unwrap(),
        include_bytes!("../../../../../tests/fixtures/images/handoff-hero.png")
    );
}

#[tokio::test]
async fn typed_attachment_with_a_local_path_is_rejected() {
    use tinychannels_bus::{ContentPart, MediaReference};
    let (_temp, config) = setup().await;
    let mut req = request("bad", "look", "user");
    req.attachments = vec![ContentPart::File {
        media: MediaReference {
            path: Some("/etc/passwd".into()),
            ..Default::default()
        },
        file_name: None,
        caption: None,
    }];
    let err = append_with_test_config(req, &config, None)
        .await
        .expect_err("local paths must be refused");
    assert!(err.to_string().contains("local paths"), "{err}");
}

#[tokio::test]
async fn public_metadata_updates_preserve_private_attachment_bindings() {
    use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};
    use tinychannels_bus::{ContentPart, MediaReference};

    let (_temp, config) = setup().await;
    let mut req = request("metadata-image", "look", "user");
    req.attachments = vec![ContentPart::Image {
        media: MediaReference {
            url: Some(format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(include_bytes!(
                    "../../../../../tests/fixtures/images/handoff-hero.png"
                ))
            )),
            ..Default::default()
        },
        caption: None,
        alt_text: None,
    }];
    let saved = append_with_test_config(req, &config, None)
        .await
        .unwrap()
        .value
        .data
        .unwrap();

    let ctx = DispatchCtx::new(CoreContext::for_test_with_config(
        DomainSet::full(),
        config.clone(),
    ));
    message_update(
        &ctx,
        UpdateConversationMessageRequest {
            thread_id: "uploads".into(),
            message_id: saved.id.clone(),
            extra_metadata: Some(json!({"pinned": true})),
        },
    )
    .await
    .expect("public metadata update");

    let (left, right) = tokio::join!(
        message_update(
            &ctx,
            UpdateConversationMessageRequest {
                thread_id: "uploads".into(),
                message_id: saved.id.clone(),
                extra_metadata: Some(json!({"seen": true})),
            },
        ),
        message_update(
            &ctx,
            UpdateConversationMessageRequest {
                thread_id: "uploads".into(),
                message_id: saved.id.clone(),
                extra_metadata: Some(json!({"reactionSummary": {"heart": 1}})),
            },
        ),
    );
    left.expect("concurrent metadata update one");
    right.expect("concurrent metadata update two");
    let current = conversations::blocking::get_messages(
        ctx.clone(),
        config.workspace_dir.clone(),
        "uploads".to_string(),
    )
    .await
    .unwrap()
    .into_iter()
    .find(|message| message.id == saved.id)
    .unwrap();
    assert_eq!(current.extra_metadata["pinned"], true);
    assert_eq!(current.extra_metadata["seen"], true);
    assert_eq!(current.extra_metadata["reactionSummary"]["heart"], 1);

    let resolved = crate::threads::attachments::resolve_persisted_parts(
        &ctx,
        "uploads",
        &saved.id,
        &saved.attachments,
    )
    .await
    .expect("attachment binding survives metadata update");
    let path = match &resolved[0] {
        ContentPart::Image { media, .. } => media.path.as_deref().expect("resolved path"),
        other => panic!("expected image attachment, got {other:?}"),
    };
    assert_eq!(
        tokio::fs::read(config.action_dir.join(path)).await.unwrap(),
        include_bytes!("../../../../../tests/fixtures/images/handoff-hero.png")
    );
}

#[tokio::test]
async fn queued_user_append_reuses_exact_request_row_and_rejects_conflicts() {
    use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};
    use base64::Engine as _;
    use tinychannels_bus::{ContentPart, MediaReference};

    let (_temp, config) = setup().await;
    let ctx = DispatchCtx::new(CoreContext::for_test_with_config(
        DomainSet::full(),
        config.clone(),
    ));
    let staged = crate::threads::attachments::stage_queued_parts(
        &ctx,
        "uploads",
        &[ContentPart::Image {
            media: MediaReference {
                url: Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(include_bytes!(
                        "../../../../../tests/fixtures/images/handoff-hero.png"
                    ))
                )),
                ..Default::default()
            },
            caption: None,
            alt_text: None,
        }],
    )
    .await
    .unwrap();
    let first = persist_queued_user_message(&ctx, "uploads", "request-1", "", &staged)
        .await
        .unwrap();
    assert_eq!(first.id, "user:request-1");
    assert_eq!(first.sender, "user");
    assert!(first.content.is_empty());
    assert_eq!(first.attachments, staged);
    let resolved = crate::threads::attachments::resolve_persisted_parts(
        &ctx,
        "uploads",
        &first.id,
        &first.attachments,
    )
    .await
    .expect("queued refs are bound to the persisted row");
    let path = match &resolved[0] {
        ContentPart::Image { media, .. } => media.path.as_deref().expect("resolved path"),
        other => panic!("expected image attachment, got {other:?}"),
    };
    assert_eq!(
        tokio::fs::read(config.action_dir.join(path)).await.unwrap(),
        include_bytes!("../../../../../tests/fixtures/images/handoff-hero.png")
    );

    let retry = persist_queued_user_message(&ctx, "uploads", "request-1", "", &staged)
        .await
        .unwrap();
    assert_eq!(retry.id, first.id);

    let conflicting_retry =
        persist_queued_user_message(&ctx, "uploads", "request-1", "different content", &staged)
            .await;
    assert!(conflicting_retry.is_err());

    let rows =
        conversations::blocking::get_messages(ctx, config.workspace_dir.clone(), "uploads".into())
            .await
            .unwrap();
    assert_eq!(rows.iter().filter(|row| row.id == first.id).count(), 1);
}

#[tokio::test]
async fn concurrent_queued_attachment_transfer_has_one_exact_owner() {
    use crate::core::runtime::{CoreContext, DispatchCtx, DomainSet};
    use base64::Engine as _;
    use tinychannels_bus::{ContentPart, MediaReference};

    let (_temp, config) = setup().await;
    let ctx = DispatchCtx::new(CoreContext::for_test_with_config(
        DomainSet::full(),
        config.clone(),
    ));
    let image = || ContentPart::Image {
        media: MediaReference {
            url: Some(format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(include_bytes!(
                    "../../../../../tests/fixtures/images/handoff-hero.png"
                ))
            )),
            ..Default::default()
        },
        caption: None,
        alt_text: None,
    };

    let staged_conflict =
        crate::threads::attachments::stage_queued_parts(&ctx, "uploads", &[image()])
            .await
            .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let left_ctx = ctx.clone();
    let left_refs = staged_conflict.clone();
    let left_barrier = barrier.clone();
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        persist_queued_user_message(&left_ctx, "uploads", "same", "left", &left_refs).await
    });
    let right_ctx = ctx.clone();
    let right_refs = staged_conflict.clone();
    let right = tokio::spawn(async move {
        barrier.wait().await;
        persist_queued_user_message(&right_ctx, "uploads", "same", "right", &right_refs).await
    });
    let results = [left.await.unwrap(), right.await.unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let rows = conversations::get_messages(&ctx, config.workspace_dir.clone(), "uploads").unwrap();
    assert_eq!(rows.iter().filter(|row| row.id == "user:same").count(), 1);

    let staged_retry = crate::threads::attachments::stage_queued_parts(&ctx, "uploads", &[image()])
        .await
        .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let left_ctx = ctx.clone();
    let left_refs = staged_retry.clone();
    let left_barrier = barrier.clone();
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        persist_queued_user_message(&left_ctx, "uploads", "same-retry", "same", &left_refs).await
    });
    let right_ctx = ctx.clone();
    let right_refs = staged_retry.clone();
    let right = tokio::spawn(async move {
        barrier.wait().await;
        persist_queued_user_message(&right_ctx, "uploads", "same-retry", "same", &right_refs).await
    });
    let results = [left.await.unwrap(), right.await.unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 2);

    let staged_single_owner =
        crate::threads::attachments::stage_queued_parts(&ctx, "uploads", &[image()])
            .await
            .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let left_ctx = ctx.clone();
    let left_refs = staged_single_owner.clone();
    let left_barrier = barrier.clone();
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        persist_queued_user_message(&left_ctx, "uploads", "owner-a", "", &left_refs).await
    });
    let right_ctx = ctx.clone();
    let right_refs = staged_single_owner.clone();
    let right = tokio::spawn(async move {
        barrier.wait().await;
        persist_queued_user_message(&right_ctx, "uploads", "owner-b", "", &right_refs).await
    });
    let results = [left.await.unwrap(), right.await.unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
}
