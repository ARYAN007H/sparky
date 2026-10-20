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
