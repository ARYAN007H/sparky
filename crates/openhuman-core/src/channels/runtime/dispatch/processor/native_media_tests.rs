use super::*;
use tinychannels_bus::{ChannelInboundEnvelope, MediaKind};

fn reference() -> MediaReference {
    MediaReference {
        opaque_id: Some("opaque-media".into()),
        content_type: Some("image/png".into()),
        size_bytes: Some(32),
        kind: MediaKind::Image,
        ..Default::default()
    }
}

#[test]
fn native_media_requires_an_opaque_reference_and_rejects_provider_paths() {
    let mut media = reference();
    media.opaque_id = None;
    assert!(validate_reference(&media, true, 1024, &[]).is_err());

    let mut media = reference();
    media.path = Some("/etc/passwd".into());
    assert!(validate_reference(&media, true, 1024, &[]).is_err());

    let mut media = reference();
    media.url = Some("https://attacker.invalid/media".into());
    assert!(validate_reference(&media, true, 1024, &[]).is_err());

    let mut media = reference();
    media.local_cache_path = Some("/tmp/private".into());
    assert!(validate_reference(&media, true, 1024, &[]).is_err());
}

#[test]
fn native_media_rejects_declared_oversize_and_unsupported_mime() {
    let mut media = reference();
    media.size_bytes = Some(1025);
    assert!(validate_reference(&media, true, 1024, &[]).is_err());

    let mut media = reference();
    media.content_type = Some("image/svg+xml".into());
    assert!(validate_reference(&media, true, 1024, &[]).is_err());

    let mut file = reference();
    file.content_type = Some("application/x-executable".into());
    assert!(validate_reference(&file, false, 1024, &["text/plain".into()]).is_err());
}

#[test]
fn typed_native_media_is_selected_only_from_envelope_parts() {
    let envelope = ChannelInboundEnvelope {
        parts: vec![ContentPart::Image {
            media: reference(),
            caption: None,
            alt_text: None,
        }],
        ..Default::default()
    };
    assert!(requires_typed_preparation(&envelope));

    let text_only = ChannelInboundEnvelope {
        parts: vec![ContentPart::Text {
            text: "ordinary text".into(),
            markdown_dialect: None,
        }],
        ..Default::default()
    };
    assert!(!requires_typed_preparation(&text_only));
}
