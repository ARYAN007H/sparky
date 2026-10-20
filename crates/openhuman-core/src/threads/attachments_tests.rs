use super::*;

fn image(media: MediaReference) -> ContentPart {
    ContentPart::Image {
        media,
        caption: None,
        alt_text: None,
    }
}

#[test]
fn persisted_refs_require_unique_opaque_ids_without_source_authority() {
    let valid = image(MediaReference {
        opaque_id: Some("opaque-1".into()),
        content_type: Some("image/png".into()),
        ..Default::default()
    });
    assert!(validate_opaque_parts(std::slice::from_ref(&valid)).is_ok());

    for media in [
        MediaReference {
            opaque_id: Some("opaque-1".into()),
            path: Some("../../etc/passwd".into()),
            ..Default::default()
        },
        MediaReference {
            opaque_id: Some("opaque-1".into()),
            url: Some("data:image/png;base64,AAAA".into()),
            ..Default::default()
        },
        MediaReference::default(),
    ] {
        assert!(validate_opaque_parts(&[image(media)]).is_err());
    }
    assert!(validate_opaque_parts(&[valid.clone(), valid]).is_err());
}

#[test]
fn public_projection_hides_paths_but_keeps_opaque_attachment_refs() {
    let part = image(MediaReference {
        opaque_id: Some("opaque-2".into()),
        content_type: Some("image/png".into()),
        ..Default::default()
    });
    let mut metadata = json!({"keep": true});
    let public = save(
        &mut metadata,
        vec![(part.clone(), "uploads/thread/a.png".into())],
    );
    assert_eq!(public, vec![part]);
    assert!(metadata[STORAGE_KEY][0]["path"]
        .as_str()
        .unwrap()
        .contains("uploads/"));

    let projected = public_refs(&metadata);
    assert_eq!(projected, public);
    strip_private_metadata(&mut metadata);
    assert_eq!(metadata, json!({"keep": true}));
}

fn claim(message_id: &str, part: ContentPart, canceled: bool) -> StagedAttachmentClaim {
    StagedAttachmentClaim {
        tenant: crate::core::runtime::Tenant::default(),
        thread_id: "thread".into(),
        message_id: message_id.into(),
        part,
        canceled,
    }
}

#[tokio::test]
async fn partial_claim_failure_does_not_revoke_same_owner_retry() {
    let dir = tempfile::tempdir().unwrap();
    let first_path = dir.path().join("first.claim.json");
    let second_path = dir.path().join("second.claim.json");
    let first = claim(
        "user:req",
        image(MediaReference {
            opaque_id: Some("first".into()),
            ..Default::default()
        }),
        false,
    );
    let other_owner = claim(
        "user:other",
        image(MediaReference {
            opaque_id: Some("second".into()),
            ..Default::default()
        }),
        false,
    );
    let cancel_second = claim("", other_owner.part.clone(), true);

    publish_staged_claim(&first_path, &first).await.unwrap();
    publish_staged_claim(&second_path, &cancel_second)
        .await
        .unwrap();
    assert!(publish_staged_claim(&second_path, &other_owner)
        .await
        .is_err());
    assert!(!publish_staged_claim(&first_path, &first).await.unwrap());
    let stored: StagedAttachmentClaim =
        serde_json::from_slice(&tokio::fs::read(&first_path).await.unwrap()).unwrap();
    assert_eq!(stored, first);
}

#[tokio::test]
async fn staged_claim_and_cancellation_have_one_atomic_winner() {
    let dir = tempfile::tempdir().unwrap();
    let claim_path = dir.path().join("race.claim.json");
    let part = image(MediaReference {
        opaque_id: Some("race".into()),
        ..Default::default()
    });
    let bound = claim("user:req", part.clone(), false);
    let canceled = claim("", part, true);
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let claim_task = {
        let barrier = barrier.clone();
        let path = claim_path.clone();
        let bound = bound.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            publish_staged_claim(&path, &bound).await
        })
    };
    let cancel_task = {
        let barrier = barrier.clone();
        let path = claim_path.clone();
        let canceled = canceled.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            publish_staged_claim(&path, &canceled).await
        })
    };
    barrier.wait().await;
    let claim_result = claim_task.await.unwrap();
    let cancel_result = cancel_task.await.unwrap();
    assert_ne!(claim_result.is_ok(), cancel_result.is_ok());

    let stored: StagedAttachmentClaim =
        serde_json::from_slice(&tokio::fs::read(&claim_path).await.unwrap()).unwrap();
    if stored.canceled {
        assert!(publish_staged_claim(&claim_path, &bound).await.is_err());
    } else {
        assert!(publish_staged_claim(&claim_path, &canceled).await.is_err());
        assert!(!publish_staged_claim(&claim_path, &bound).await.unwrap());
    }
}

#[tokio::test]
async fn multipart_claim_failure_keeps_prior_claim_owned_by_retry() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = dir.path().join("workspace");
    config.action_dir = dir.path().join("actions");
    let claim_dir = config.workspace_dir.join(STAGED_DIR);
    tokio::fs::create_dir_all(&claim_dir).await.unwrap();
    let first = image(MediaReference {
        opaque_id: Some("multipart-first".into()),
        ..Default::default()
    });
    let second = image(MediaReference {
        opaque_id: Some("multipart-second".into()),
        ..Default::default()
    });
    let first_path = staged_manifest_path(&config.workspace_dir, "multipart-first").unwrap();
    let second_path = staged_manifest_path(&config.workspace_dir, "multipart-second").unwrap();
    publish_staged_claim(
        &second_path.with_extension("claim.json"),
        &claim("", second.clone(), true),
    )
    .await
    .unwrap();
    let entries = vec![
        StoredAttachment {
            part: first.clone(),
            path: "first.png".into(),
        },
        StoredAttachment {
            part: second.clone(),
            path: "second.png".into(),
        },
    ];
    assert!(claim_staged_refs(
        &crate::core::runtime::DispatchCtx::ambient(),
        "thread",
        &config,
        "user:req",
        &entries,
    )
    .await
    .is_err());

    let first_claim_path = first_path.with_extension("claim.json");
    let owner = claim("user:req", first.clone(), false);
    assert!(!publish_staged_claim(&first_claim_path, &owner)
        .await
        .unwrap());
    assert!(
        publish_staged_claim(&first_claim_path, &claim("user:other", first, false))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn discarding_claimed_attachment_preserves_row_owned_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = dir.path().join("workspace");
    config.action_dir = dir.path().join("actions");
    let part = image(MediaReference {
        opaque_id: Some("committed".into()),
        content_type: Some("image/png".into()),
        ..Default::default()
    });
    let claim_dir = config.workspace_dir.join(STAGED_DIR);
    tokio::fs::create_dir_all(&claim_dir).await.unwrap();
    let manifest_path = staged_manifest_path(&config.workspace_dir, "committed").unwrap();
    publish_staged_claim(
        &manifest_path.with_extension("claim.json"),
        &claim("user:req", part.clone(), false),
    )
    .await
    .unwrap();
    let workspace = crate::agent::attachments::thread_action_dir(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        &config.workspace_dir,
        Some("thread"),
    )
    .await
    .unwrap();
    tokio::fs::create_dir_all(&workspace).await.unwrap();
    let relative_path = "committed.png";
    let file_path = workspace.join(relative_path);
    tokio::fs::write(&file_path, b"row-owned bytes")
        .await
        .unwrap();
    let metadata = serde_json::json!({"__openhumanStoredAttachments": [{
        "part": part,
        "path": relative_path
    }]});
    let entries = stored(&metadata).unwrap();

    discard_staged_entries(
        &crate::core::runtime::DispatchCtx::ambient(),
        "thread",
        &config,
        entries,
    )
    .await
    .unwrap();
    assert_eq!(
        tokio::fs::read(file_path).await.unwrap(),
        b"row-owned bytes"
    );
    assert!(manifest_path.with_extension("claim.json").exists());
}
