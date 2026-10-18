//! Durable audio for in-app voice notes.
//!
//! A voice note is an audio file the thread row points at. The user's
//! recording and the assistant's spoken reply are both stored under
//! `<workspace>/voice_notes/<note_id>.<ext>`; the row only carries
//! `{ noteId, mime }` in its metadata, so a reload (or a second window) can
//! fetch the bytes again through `channel.web_voice_note_read`.
//!
//! Assistant replies use [`reply_note_id`], derived from the turn's
//! `request_id`, which is what lets reply persistence find the audio the turn
//! synthesized without threading it through the turn result.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

const LOG_PREFIX: &str = "[web_channel][voice_note]";
const DIR: &str = "voice_notes";
/// Largest note the store accepts (decoded bytes).
pub(crate) const MAX_NOTE_BYTES: usize = 25 * 1024 * 1024;

/// `(extension, mime)` pairs the store writes and serves.
const KNOWN: &[(&str, &str)] = &[
    ("webm", "audio/webm"),
    ("ogg", "audio/ogg"),
    ("m4a", "audio/mp4"),
    ("mp3", "audio/mpeg"),
    ("wav", "audio/wav"),
];

/// Id of the audio a turn's spoken reply is stored under.
pub(crate) fn reply_note_id(request_id: &str) -> String {
    let safe: String = request_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    format!("reply-{safe}")
}

/// Ids become file names, so only a conservative alphabet is accepted.
pub(crate) fn is_valid_note_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn extension_for_mime(mime: &str) -> &'static str {
    let base = mime.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    match base.as_str() {
        "audio/ogg" => "ogg",
        "audio/mp4" | "audio/x-m4a" | "audio/aac" => "m4a",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
        _ => "webm",
    }
}

fn dir(workspace: &Path) -> PathBuf {
    workspace.join(DIR)
}

/// Writes `bytes` as note `id`, replacing any earlier file for that id.
/// Returns the stored mime type.
pub(crate) fn store_note(
    workspace: &Path,
    id: &str,
    mime: &str,
    bytes: &[u8],
) -> Result<&'static str, String> {
    if !is_valid_note_id(id) {
        return Err("invalid voice note id".to_string());
    }
    if bytes.is_empty() {
        return Err("voice note audio is empty".to_string());
    }
    if bytes.len() > MAX_NOTE_BYTES {
        return Err(format!("voice note exceeds {MAX_NOTE_BYTES} bytes"));
    }
    let ext = extension_for_mime(mime);
    let root = dir(workspace);
    std::fs::create_dir_all(&root).map_err(|e| format!("create voice_notes dir: {e}"))?;
    for (other, _) in KNOWN.iter().filter(|(e, _)| *e != ext) {
        let _ = std::fs::remove_file(root.join(format!("{id}.{other}")));
    }
    std::fs::write(root.join(format!("{id}.{ext}")), bytes)
        .map_err(|e| format!("write voice note: {e}"))?;
    log::debug!("{LOG_PREFIX} stored id={id} ext={ext} bytes={}", bytes.len());
    Ok(KNOWN.iter().find(|(e, _)| *e == ext).map(|(_, m)| *m).unwrap_or("audio/webm"))
}

/// Reads note `id`; `Ok(None)` when no file exists.
pub(crate) fn read_note(
    workspace: &Path,
    id: &str,
) -> Result<Option<(Vec<u8>, &'static str)>, String> {
    if !is_valid_note_id(id) {
        return Err("invalid voice note id".to_string());
    }
    let root = dir(workspace);
    for (ext, mime) in KNOWN {
        let path = root.join(format!("{id}.{ext}"));
        if path.is_file() {
            let bytes = std::fs::read(&path).map_err(|e| format!("read voice note: {e}"))?;
            return Ok(Some((bytes, mime)));
        }
    }
    Ok(None)
}

/// The `voiceNote` metadata for a reply whose audio was stored, if any.
pub(crate) fn reply_note_metadata(workspace: &Path, request_id: &str) -> Option<Value> {
    let id = reply_note_id(request_id);
    let root = dir(workspace);
    KNOWN.iter().find_map(|(ext, mime)| {
        root.join(format!("{id}.{ext}"))
            .is_file()
            .then(|| json!({ "noteId": id, "mime": mime }))
    })
}

#[cfg(test)]
#[path = "voice_notes_tests.rs"]
mod tests;
