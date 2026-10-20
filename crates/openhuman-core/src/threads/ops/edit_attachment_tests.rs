use super::*;
use base64::Engine as _;

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

#[tokio::test]
async fn editing_an_image_row_restages_only_its_exact_opaque_reference() {
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
            id: "edit-image-source".into(),
            title: "Edit image source".into(),
            created_at: "2026-10-20T00:00:00Z".into(),
            parent_thread_id: None,
            labels: None,
            personality_id: None,
            working_dir: None,
        },
    )
    .await
    .unwrap();
    let source = crate::threads::ops::message_append(
        &ctx,
        AppendConversationMessageRequest {
            thread_id: "edit-image-source".into(),
            message: ConversationMessageRecord {
                id: "user-image-source".into(),
                content: "describe this".into(),
                attachments: Vec::new(),
                message_type: "text".into(),
                extra_metadata: serde_json::json!({}),
                sender: "user".into(),
                created_at: "2026-10-20T00:00:01Z".into(),
            },
            attachments: vec![ContentPart::Image {
                media: MediaReference {
                    url: Some(format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(include_bytes!(
                            "../../../../../tests/fixtures/images/handoff-hero.png"
                        ))
                    )),
                    content_type: Some("image/png".into()),
                    kind: tinychannels_bus::MediaKind::Image,
                    ..Default::default()
                },
                caption: None,
                alt_text: None,
            }],
        },
    )
    .await
    .unwrap()
    .value
    .data
    .unwrap();
    assert_eq!(source.attachments.len(), 1);
    let original_ref = source.attachments[0].clone();

    let replacement_refs = crate::threads::attachments::restage_persisted_parts(
        &ctx,
        "edit-image-source",
        &source.id,
        &[original_ref.clone()],
    )
    .await
    .unwrap();
    assert_eq!(replacement_refs.len(), 1);
    assert_ne!(replacement_refs[0], original_ref);
    crate::threads::attachments::validate_opaque_parts(&replacement_refs).unwrap();

    let other_row = crate::threads::attachments::restage_persisted_parts(
        &ctx,
        "edit-image-source",
        "user-someone-elses-row",
        &[original_ref],
    )
    .await;
    assert!(other_row.is_err());
    crate::threads::attachments::discard_staged_parts(
        &ctx,
        "edit-image-source",
        &replacement_refs,
    )
    .await
    .unwrap();
}
