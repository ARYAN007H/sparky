//! Durable typed attachment references for conversation messages.
//!
//! The public message stores opaque TinyChannels media parts separately from
//! its text. Workspace-relative file paths remain private to OpenHuman's
//! persisted metadata and are resolved only under the trusted dispatch scope
//! when a turn is about to execute.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tinychannels_bus::{ContentPart, MediaReference};

const STORAGE_KEY: &str = "__openhumanStoredAttachments";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAttachment {
    part: ContentPart,
    path: String,
}

/// Persist sanitized typed parts and their private staged paths in message
/// metadata. The public row projection strips the private path records.
pub(crate) fn save(
    metadata: &mut Value,
    entries: Vec<(ContentPart, String)>,
) -> Vec<ContentPart> {
    let stored = entries
        .into_iter()
        .map(|(part, path)| StoredAttachment { part, path })
        .collect::<Vec<_>>();
    let public = stored.iter().map(|entry| entry.part.clone()).collect();
    if !metadata.is_object() {
        *metadata = json!({});
    }
    metadata[STORAGE_KEY] = serde_json::to_value(stored).expect("stored attachments serialize");
    public
}

/// Public opaque refs recorded on a message row, with no local path or URL.
pub(crate) fn public_refs(metadata: &Value) -> Vec<ContentPart> {
    stored(metadata)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| entry.part)
        .collect()
}

/// Remove the private path map from metadata returned over RPC.
pub(crate) fn strip_private_metadata(metadata: &mut Value) {
    if let Some(object) = metadata.as_object_mut() {
        object.remove(STORAGE_KEY);
    }
}

fn stored(metadata: &Value) -> Result<Vec<StoredAttachment>, String> {
    let Some(value) = metadata.get(STORAGE_KEY) else {
        return Ok(Vec::new());
    };
    serde_json::from_value(value.clone())
        .map_err(|error| format!("stored attachment metadata is invalid: {error}"))
}

fn opaque_media(part: &ContentPart) -> Option<&MediaReference> {
    match part {
        ContentPart::Image { media, .. }
        | ContentPart::Audio { media, .. }
        | ContentPart::Video { media, .. }
        | ContentPart::File { media, .. } => Some(media),
        _ => None,
    }
}

/// Resolve request refs against one persisted user row. The request may carry
/// opaque IDs and descriptive metadata only; storage paths come exclusively
/// from the exact row's private metadata.
pub(crate) async fn resolve_persisted_parts(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    persisted_message_id: &str,
    requested: &[ContentPart],
) -> Result<Vec<ContentPart>, String> {
    if thread_id.is_empty() || persisted_message_id.is_empty() || requested.is_empty() {
        return Err("stored attachment scope and refs must be non-empty".into());
    }
    for part in requested {
        let media = opaque_media(part)
            .ok_or_else(|| "stored attachment has an unsupported kind".to_string())?;
        if media.opaque_id.as_deref().is_none_or(str::is_empty)
            || media.path.is_some()
            || media.url.is_some()
            || media.local_cache_path.is_some()
        {
            return Err("stored attachment must use an opaque reference only".into());
        }
    }

    let config = ctx
        .load_config()
        .await
        .map_err(|error| format!("stored attachment configuration unavailable: {error}"))?;
    let workspace = crate::agent::attachments::thread_action_dir(
        ctx,
        &config,
        &config.workspace_dir,
        Some(thread_id),
    )
    .await
    .map_err(|error| format!("stored attachment workspace unavailable: {error}"))?;
    let messages = crate::threads::store::get_messages(ctx, config.workspace_dir, thread_id)?;
    let row = messages
        .iter()
        .find(|message| message.id == persisted_message_id && message.sender == "user")
        .ok_or_else(|| "stored attachment message is missing or is not a user row".to_string())?;
    let entries = stored(&row.extra_metadata)?;
    let stored_parts = entries
        .iter()
        .map(|entry| entry.part.clone())
        .collect::<Vec<_>>();
    if requested != stored_parts {
        return Err("stored attachment refs do not match the persisted user row".into());
    }

    let mut resolved = Vec::with_capacity(entries.len());
    for mut entry in entries {
        let scope = crate::agent::attachments::AttachmentAccessScope {
            external_channel: false,
            workspace: Some(workspace.clone()),
        };
        let path = crate::agent::attachments::resolve_path(&config, &entry.path, &scope)
            .await
            .map_err(|error| format!("stored attachment is unavailable: {error}"))?;
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|error| format!("stored attachment is unavailable: {error}"))?;
        if !metadata.is_file() {
            return Err("stored attachment is not a regular file".into());
        }
        let media = opaque_media_mut(&mut entry.part)
            .ok_or_else(|| "stored attachment has an unsupported kind".to_string())?;
        media.path = Some(entry.path);
        resolved.push(entry.part);
    }
    Ok(resolved)
}

fn opaque_media_mut(part: &mut ContentPart) -> Option<&mut MediaReference> {
    match part {
        ContentPart::Image { media, .. }
        | ContentPart::Audio { media, .. }
        | ContentPart::Video { media, .. }
        | ContentPart::File { media, .. } => Some(media),
        _ => None,
    }
}
