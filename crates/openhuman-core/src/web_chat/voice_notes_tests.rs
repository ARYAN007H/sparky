use super::*;

#[test]
fn stored_note_reads_back_with_mime() {
    let tmp = tempfile::tempdir().unwrap();
    let mime = store_note(tmp.path(), "note-1", "audio/ogg;codecs=opus", b"abc").unwrap();
    assert_eq!(mime, "audio/ogg");
    let (bytes, mime) = read_note(tmp.path(), "note-1").unwrap().unwrap();
    assert_eq!(bytes, b"abc");
    assert_eq!(mime, "audio/ogg");
}

#[test]
fn restoring_under_another_mime_drops_the_old_file() {
    let tmp = tempfile::tempdir().unwrap();
    store_note(tmp.path(), "n", "audio/webm", b"one").unwrap();
    store_note(tmp.path(), "n", "audio/mpeg", b"two").unwrap();
    let (bytes, mime) = read_note(tmp.path(), "n").unwrap().unwrap();
    assert_eq!((bytes.as_slice(), mime), (b"two".as_slice(), "audio/mpeg"));
    assert!(!tmp.path().join("voice_notes/n.webm").exists());
}

#[test]
fn missing_note_is_none() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(read_note(tmp.path(), "nope").unwrap().is_none());
}

#[test]
fn path_traversal_ids_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(store_note(tmp.path(), "../evil", "audio/webm", b"x").is_err());
    assert!(read_note(tmp.path(), "a/b").is_err());
    assert!(!is_valid_note_id(""));
    assert!(!is_valid_note_id(&"a".repeat(129)));
}

#[test]
fn empty_and_oversized_audio_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(store_note(tmp.path(), "e", "audio/webm", b"").is_err());
    let big = vec![0u8; MAX_NOTE_BYTES + 1];
    assert!(store_note(tmp.path(), "big", "audio/webm", &big).is_err());
}

#[test]
fn reply_metadata_exists_only_after_the_reply_audio_is_stored() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(reply_note_metadata(tmp.path(), "req/1").is_none());
    store_note(tmp.path(), &reply_note_id("req/1"), "audio/mpeg", b"mp3").unwrap();
    let meta = reply_note_metadata(tmp.path(), "req/1").unwrap();
    assert_eq!(meta["noteId"], "reply-req_1");
    assert_eq!(meta["mime"], "audio/mpeg");
}
