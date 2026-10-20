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
