//! Durable typed attachment references for conversation messages.
//!
//! The public message stores opaque TinyChannels media parts separately from
//! its text. Workspace-relative file paths remain private to OpenHuman's
//! persisted metadata and are resolved only under the trusted dispatch scope
//! when a turn is about to execute.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tinychannels_bus::{ContentPart, MediaReference};

use crate::agent::attachments::Attachment;

const STORAGE_KEY: &str = "__openhumanStoredAttachments";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAttachment {
    part: ContentPart,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StagedAttachment {
    tenant: crate::core::runtime::Tenant,
    thread_id: String,
    attachment: StoredAttachment,
    expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct StagedAttachmentClaim {
    tenant: crate::core::runtime::Tenant,
    thread_id: String,
    message_id: String,
    part: ContentPart,
    #[serde(default)]
    canceled: bool,
}

const STAGED_DIR: &str = "openhuman-staged-attachments";
const STAGED_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// Persist sanitized typed parts and their private staged paths in message
/// metadata. The public row projection strips the private path records.
pub(crate) fn save(metadata: &mut Value, entries: Vec<(ContentPart, String)>) -> Vec<ContentPart> {
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

/// Pair typed media input with the staged files created from it, storing only
/// opaque refs publicly and keeping each validated relative path privately.
pub(crate) fn from_staged_parts(
    parts: &[ContentPart],
    files: &[Attachment],
) -> Result<Vec<(ContentPart, String)>, String> {
    let media_count = parts
        .iter()
        .filter(|part| {
            matches!(
                part,
                ContentPart::Image { .. }
                    | ContentPart::Audio { .. }
                    | ContentPart::Video { .. }
                    | ContentPart::File { .. }
            )
        })
        .count();
    if media_count != files.len() {
        return Err("typed attachment staging did not preserve media ordering".into());
    }

    let mut files = files.iter();
    let mut output = Vec::with_capacity(media_count);
    for part in parts {
        match part {
            ContentPart::Image {
                media,
                caption,
                alt_text,
            } => {
                let file = files.next().expect("media count was checked");
                output.push((
                    ContentPart::Image {
                        media: stored_media(media, file),
                        caption: caption.clone(),
                        alt_text: alt_text.clone(),
                    },
                    file.path.clone(),
                ));
            }
            ContentPart::Audio {
                media,
                codec,
                duration_ms,
                title,
                transcript,
            } => {
                let file = files.next().expect("media count was checked");
                output.push((
                    ContentPart::Audio {
                        media: stored_media(media, file),
                        codec: codec.clone(),
                        duration_ms: *duration_ms,
                        title: title.clone(),
                        transcript: transcript.clone(),
                    },
                    file.path.clone(),
                ));
            }
            ContentPart::Video {
                media,
                caption,
                duration_ms,
                thumbnail,
            } => {
                let file = files.next().expect("media count was checked");
                output.push((
                    ContentPart::Video {
                        media: stored_media(media, file),
                        caption: caption.clone(),
                        duration_ms: *duration_ms,
                        thumbnail: thumbnail.as_ref().map(|thumb| {
                            let mut sanitized = thumb.clone();
                            sanitized.path = None;
                            sanitized.url = None;
                            sanitized.local_cache_path = None;
                            sanitized.opaque_id = None;
                            sanitized
                        }),
                    },
                    file.path.clone(),
                ));
            }
            ContentPart::File {
                media,
                file_name,
                caption,
            } => {
                let file = files.next().expect("media count was checked");
                let mut stored = stored_media(media, file);
                if let Some(name) = file_name {
                    stored.file_name = Some(name.clone());
                }
                output.push((
                    ContentPart::File {
                        media: stored,
                        file_name: file_name.clone(),
                        caption: caption.clone(),
                    },
                    file.path.clone(),
                ));
            }
            ContentPart::Text { .. } | ContentPart::VoiceNote { .. } => {}
            _ => return Err("typed attachment has an unsupported kind".into()),
        }
    }
    Ok(output)
}

fn stored_media(input: &MediaReference, file: &Attachment) -> MediaReference {
    let mut media = input.clone();
    media.opaque_id = Some(uuid::Uuid::new_v4().simple().to_string());
    media.path = None;
    media.url = None;
    media.local_cache_path = None;
    media.content_type = Some(file.mime.clone());
    media.file_name = Some(file.name.clone());
    media.size_bytes = Some(file.size_bytes as u64);
    media
}

fn stored(metadata: &Value) -> Result<Vec<StoredAttachment>, String> {
    let Some(value) = metadata.get(STORAGE_KEY) else {
        return Ok(Vec::new());
    };
    serde_json::from_value(value.clone())
        .map_err(|error| format!("stored attachment metadata is invalid: {error}"))
}

fn tenant_for(
    ctx: &crate::core::runtime::DispatchCtx,
) -> Result<crate::core::runtime::Tenant, String> {
    crate::core::runtime::tenant::tenant_in(
        crate::core::runtime::is_saas(),
        ctx.resolved().as_deref(),
    )
    .map_err(|error| error.to_string())
}

fn staged_manifest_path(
    workspace: &std::path::Path,
    opaque_id: &str,
) -> Result<std::path::PathBuf, String> {
    let uuid = uuid::Uuid::parse_str(opaque_id)
        .map_err(|_| "staged attachment reference is invalid".to_string())?;
    Ok(workspace
        .join(STAGED_DIR)
        .join(format!("{}.json", uuid.simple())))
}

async fn read_staged_entries(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    requested: &[ContentPart],
    config: &crate::config::Config,
) -> Result<Vec<StoredAttachment>, String> {
    validate_opaque_parts(requested)?;
    let tenant = tenant_for(ctx)?;
    let mut entries = Vec::with_capacity(requested.len());
    for part in requested {
        let opaque_id = opaque_media(part)
            .and_then(|media| media.opaque_id.as_deref())
            .ok_or_else(|| "staged attachment reference is missing".to_string())?;
        let path = staged_manifest_path(&config.workspace_dir, opaque_id)?;
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|_| "staged attachment is missing or expired".to_string())?;
        let manifest: StagedAttachment = serde_json::from_slice(&bytes)
            .map_err(|error| format!("staged attachment record is invalid: {error}"))?;
        if manifest.tenant != tenant || manifest.thread_id != thread_id {
            return Err("staged attachment belongs to a different scope".into());
        }
        if manifest.expires_at < chrono::Utc::now().timestamp() {
            return Err("staged attachment is missing or expired".into());
        }
        if &manifest.attachment.part != part {
            return Err("staged attachment reference does not match its stored record".into());
        }
        entries.push(manifest.attachment);
    }
    Ok(entries)
}

/// Bind staged refs to a new user row's private path map without exposing
/// paths in the returned row projection.
pub(crate) async fn bind_staged_refs(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    config: &crate::config::Config,
    requested: &[ContentPart],
    message_id: &str,
    metadata: &mut Value,
) -> Result<Vec<ContentPart>, String> {
    let entries = read_staged_entries(ctx, thread_id, requested, config).await?;
    validate_staged_files(ctx, thread_id, config, &entries).await?;
    claim_staged_refs(ctx, thread_id, config, message_id, &entries).await?;
    Ok(save(
        metadata,
        entries
            .into_iter()
            .map(|entry| (entry.part, entry.path))
            .collect(),
    ))
}

async fn claim_staged_refs(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    config: &crate::config::Config,
    message_id: &str,
    entries: &[StoredAttachment],
) -> Result<(), String> {
    if message_id.is_empty() {
        return Err("staged attachment target message ID is empty".into());
    }
    let tenant = tenant_for(ctx)?;
    let mut created = Vec::new();
    for entry in entries {
        let opaque_id = opaque_media(&entry.part)
            .and_then(|media| media.opaque_id.as_deref())
            .ok_or_else(|| "staged attachment reference is missing".to_string())?;
        let manifest_path = staged_manifest_path(&config.workspace_dir, opaque_id)?;
        let claim_path = manifest_path.with_extension("claim.json");
        let claim = StagedAttachmentClaim {
            tenant: tenant.clone(),
            thread_id: thread_id.to_string(),
            message_id: message_id.to_string(),
            part: entry.part.clone(),
            canceled: false,
        };
        let bytes = serde_json::to_vec(&claim)
            .map_err(|error| format!("serialize staged attachment claim: {error}"))?;
        let temp_path = manifest_path.with_extension(format!("{}.claim-tmp", uuid::Uuid::new_v4()));
        let result = async {
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)
                .await
                .map_err(|error| format!("create staged attachment claim: {error}"))?;
            file.write_all(&bytes)
                .await
                .map_err(|error| format!("write staged attachment claim: {error}"))?;
            file.sync_all()
                .await
                .map_err(|error| format!("sync staged attachment claim: {error}"))?;
            match tokio::fs::hard_link(&temp_path, &claim_path).await {
                Ok(()) => Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let stored: StagedAttachmentClaim = serde_json::from_slice(
                        &tokio::fs::read(&claim_path)
                            .await
                            .map_err(|read| format!("read staged attachment claim: {read}"))?,
                    )
                    .map_err(|read| format!("decode staged attachment claim: {read}"))?;
                    if stored == claim {
                        Ok(false)
                    } else {
                        Err("staged attachment is already bound to another message".into())
                    }
                }
                Err(error) => Err(format!("publish staged attachment claim: {error}")),
            }
        }
        .await;
        let _ = tokio::fs::remove_file(&temp_path).await;
        match result {
            Ok(_) => {}
            Ok(false) => {}
            Err(error) => {
                return Err(error);
            }
        }
    }
    Ok(())
}

/// Stage media supplied with a queued follow-up, returning only opaque refs.
/// The manifest is bound to this dispatch tenant and thread, and remains
/// separate from visible message history until the runner admits the turn.
pub(crate) async fn stage_queued_parts(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    input: &[ContentPart],
) -> Result<Vec<ContentPart>, String> {
    if thread_id.is_empty() || input.is_empty() {
        return Err("queued attachment scope and parts must be non-empty".into());
    }
    for part in input {
        if let Some(media) = opaque_media(part) {
            if media.opaque_id.is_some() {
                return Err("queued attachment input must not contain existing opaque refs".into());
            }
        }
    }
    let config = ctx
        .load_config()
        .await
        .map_err(|error| format!("queued attachment configuration unavailable: {error}"))?;
    let workspace = crate::agent::attachments::thread_action_dir(
        ctx,
        &config,
        &config.workspace_dir,
        Some(thread_id),
    )
    .await
    .map_err(|error| format!("queued attachment workspace unavailable: {error}"))?;
    let origin = ctx.turn_origin_or_scoped();
    let folded = crate::agent::attachments::parts::fold_into_message("", input)
        .map_err(|error| error.to_string())?;
    let staged = crate::agent::attachments::stage_turn(
        &folded,
        Some(&config),
        Some(&tinytools::WorkspaceDescriptor::new(workspace)),
        Some(thread_id),
        origin.as_ref(),
    )
    .await
    .map_err(|error| format!("stage queued attachments: {error}"))?;
    let (_, files) = crate::agent::attachments::parse(&staged);
    let entries = from_staged_parts(input, &files)?;
    let tenant = tenant_for(ctx)?;
    let manifest_dir = config.workspace_dir.join(STAGED_DIR);
    tokio::fs::create_dir_all(&manifest_dir)
        .await
        .map_err(|error| format!("create queued attachment store: {error}"))?;
    let expires_at = chrono::Utc::now().timestamp() + STAGED_TTL_SECS;
    let mut public = Vec::with_capacity(entries.len());
    for attachment in entries {
        let opaque_id = opaque_media(&attachment.0)
            .and_then(|media| media.opaque_id.as_deref())
            .ok_or_else(|| "staged attachment has no opaque reference".to_string())?;
        let path = staged_manifest_path(&config.workspace_dir, opaque_id)?;
        let manifest = StagedAttachment {
            tenant: tenant.clone(),
            thread_id: thread_id.to_string(),
            attachment: StoredAttachment {
                part: attachment.0.clone(),
                path: attachment.1,
            },
            expires_at,
        };
        let bytes = serde_json::to_vec(&manifest)
            .map_err(|error| format!("serialize queued attachment record: {error}"))?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .map_err(|error| format!("create queued attachment record: {error}"))?;
        use tokio::io::AsyncWriteExt;
        file.write_all(&bytes)
            .await
            .map_err(|error| format!("write queued attachment record: {error}"))?;
        public.push(manifest.attachment.part);
    }
    Ok(public)
}

/// Resolve staged queue refs immediately before execution. The caller gets
/// trusted relative paths only after exact tenant, thread, token, and limit
/// checks pass.
pub(crate) async fn resolve_staged_parts(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    requested: &[ContentPart],
) -> Result<Vec<ContentPart>, String> {
    if thread_id.is_empty() || requested.is_empty() {
        return Err("staged attachment scope and refs must be non-empty".into());
    }
    let config = ctx
        .load_config()
        .await
        .map_err(|error| format!("staged attachment configuration unavailable: {error}"))?;
    let entries = read_staged_entries(ctx, thread_id, requested, &config).await?;
    validate_staged_files(ctx, thread_id, &config, &entries).await?;
    let mut resolved = Vec::with_capacity(entries.len());
    for mut entry in entries {
        let media = opaque_media_mut(&mut entry.part)
            .ok_or_else(|| "staged attachment has an unsupported kind".to_string())?;
        media.path = Some(entry.path);
        resolved.push(entry.part);
    }
    Ok(resolved)
}

async fn validate_staged_files(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    config: &crate::config::Config,
    entries: &[StoredAttachment],
) -> Result<(), String> {
    let workspace = crate::agent::attachments::thread_action_dir(
        ctx,
        config,
        &config.workspace_dir,
        Some(thread_id),
    )
    .await
    .map_err(|error| format!("staged attachment workspace unavailable: {error}"))?;
    let image_count = entries
        .iter()
        .filter(|entry| matches!(entry.part, ContentPart::Image { .. }))
        .count();
    let file_count = entries.len() - image_count;
    let (max_images, image_mb) = config.multimodal.effective_limits();
    let (max_files, file_mb, _) = config.multimodal_files.effective_limits();
    if (image_count > 0 && (config.multimodal.max_images == 0 || image_count > max_images))
        || (!entries.is_empty() && config.multimodal_files.max_files == 0)
        || file_count > max_files
    {
        return Err("staged attachment count exceeds configured limit".into());
    }
    let tenant = tenant_for(ctx)?;
    for entry in entries {
        let Some(id) = opaque_media(&entry.part).and_then(|media| media.opaque_id.as_deref())
        else {
            continue;
        };
        let manifest_path = staged_manifest_path(&config.workspace_dir, id)?;
        let claim_path = manifest_path.with_extension("claim.json");
        let canceled_claim = StagedAttachmentClaim {
            tenant: tenant.clone(),
            thread_id: thread_id.to_string(),
            message_id: String::new(),
            part: entry.part.clone(),
            canceled: true,
        };
        let bytes = serde_json::to_vec(&canceled_claim)
            .map_err(|error| format!("serialize staged attachment cancellation: {error}"))?;
        let temp_path = manifest_path.with_extension(format!("{}.claim-tmp", uuid::Uuid::new_v4()));
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .await
            .map_err(|error| format!("create staged attachment cancellation: {error}"))?;
        use tokio::io::AsyncWriteExt;
        file.write_all(&bytes)
            .await
            .map_err(|error| format!("write staged attachment cancellation: {error}"))?;
        file.sync_all()
            .await
            .map_err(|error| format!("sync staged attachment cancellation: {error}"))?;
        drop(file);
        let owns_cancellation = match tokio::fs::hard_link(&temp_path, &claim_path).await {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let claim: StagedAttachmentClaim = serde_json::from_slice(
                    &tokio::fs::read(&claim_path)
                        .await
                        .map_err(|read| format!("read staged attachment claim: {read}"))?,
                )
                .map_err(|read| format!("decode staged attachment claim: {read}"))?;
                claim.canceled
                    && claim.tenant == tenant
                    && claim.thread_id == thread_id
                    && claim.part == entry.part
            }
            Err(error) => return Err(format!("publish staged attachment cancellation: {error}")),
        };
        let _ = tokio::fs::remove_file(&temp_path).await;
        if !owns_cancellation {
            continue;
        }
        let scope = crate::agent::attachments::AttachmentAccessScope {
            external_channel: false,
            workspace: Some(workspace.clone()),
        };
        let path = crate::agent::attachments::resolve_path(config, &entry.path, &scope)
            .await
            .map_err(|error| format!("staged attachment is unavailable: {error}"))?;
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|error| format!("staged attachment is unavailable: {error}"))?;
        let media = opaque_media(&entry.part)
            .ok_or_else(|| "staged attachment has an unsupported kind".to_string())?;
        let is_image = matches!(entry.part, ContentPart::Image { .. });
        let mime = media
            .content_type
            .as_deref()
            .filter(|mime| !mime.is_empty())
            .ok_or_else(|| "staged attachment MIME type is missing".to_string())?;
        if (is_image && !mime.starts_with("image/"))
            || !metadata.is_file()
            || metadata.len() > (if is_image { image_mb } else { file_mb } * 1024 * 1024) as u64
        {
            return Err("staged attachment exceeds configured type or size limit".into());
        }
    }
    Ok(())
}

/// Remove staged media for a queue entry that was canceled before execution.
pub(crate) async fn discard_staged_parts(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    requested: &[ContentPart],
) -> Result<(), String> {
    let config = ctx
        .load_config()
        .await
        .map_err(|error| format!("staged attachment configuration unavailable: {error}"))?;
    let entries = read_staged_entries(ctx, thread_id, requested, &config).await?;
    let workspace = crate::agent::attachments::thread_action_dir(
        ctx,
        &config,
        &config.workspace_dir,
        Some(thread_id),
    )
    .await
    .map_err(|error| format!("staged attachment workspace unavailable: {error}"))?;
    for entry in entries {
        let scope = crate::agent::attachments::AttachmentAccessScope {
            external_channel: false,
            workspace: Some(workspace.clone()),
        };
        let path = crate::agent::attachments::resolve_path(&config, &entry.path, &scope)
            .await
            .map_err(|error| format!("staged attachment cleanup refused: {error}"))?;
        tokio::fs::remove_file(path)
            .await
            .map_err(|error| format!("remove staged attachment: {error}"))?;
        let _ = tokio::fs::remove_file(manifest_path).await;
    }
    Ok(())
}

/// Retire manifests after the staged refs have been durably bound to a
/// message row. Files remain at their validated paths and are now owned by
/// that row's private attachment metadata.
pub(crate) async fn consume_staged_parts(
    ctx: &crate::core::runtime::DispatchCtx,
    thread_id: &str,
    requested: &[ContentPart],
    message_id: &str,
) -> Result<(), String> {
    if requested.is_empty() {
        return Ok(());
    }
    let config = ctx
        .load_config()
        .await
        .map_err(|error| format!("staged attachment configuration unavailable: {error}"))?;
    let tenant = tenant_for(ctx)?;
    for part in requested {
        let id = opaque_media(part)
            .and_then(|media| media.opaque_id.as_deref())
            .ok_or_else(|| "staged attachment reference is missing".to_string())?;
        let path = staged_manifest_path(&config.workspace_dir, id)?;
        let claim_path = path.with_extension("claim.json");
        let claim: StagedAttachmentClaim =
            serde_json::from_slice(&tokio::fs::read(&claim_path).await.map_err(|_| {
                "staged attachment is not bound to a persisted message".to_string()
            })?)
            .map_err(|error| format!("staged attachment claim is invalid: {error}"))?;
        if claim.tenant != tenant
            || claim.thread_id != thread_id
            || claim.message_id != message_id
            || claim.part != *part
            || claim.canceled
        {
            return Err("staged attachment is bound to a different message or scope".into());
        }
        match tokio::fs::read(&path).await {
            Ok(bytes) => {
                let manifest: StagedAttachment = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("staged attachment record is invalid: {error}"))?;
                if manifest.tenant != tenant
                    || manifest.thread_id != thread_id
                    || manifest.attachment.part != *part
                {
                    return Err("staged attachment belongs to a different scope".into());
                }
                match tokio::fs::remove_file(path).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(format!("remove staged attachment record: {error}")),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("read staged attachment record: {error}")),
        }
    }
    Ok(())
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

pub(crate) fn validate_opaque_parts(parts: &[ContentPart]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::with_capacity(parts.len());
    for part in parts {
        let media = opaque_media(part)
            .ok_or_else(|| "stored attachment has an unsupported kind".to_string())?;
        let opaque_id = media
            .opaque_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| "stored attachment must include an opaque reference".to_string())?;
        if media.path.is_some() || media.url.is_some() || media.local_cache_path.is_some() {
            return Err("stored attachment must not include a path or URL".into());
        }
        if !ids.insert(opaque_id) {
            return Err("stored attachment refs must be unique".into());
        }
        if let ContentPart::Video {
            thumbnail: Some(thumb),
            ..
        } = part
        {
            if thumb.path.is_some() || thumb.url.is_some() || thumb.local_cache_path.is_some() {
                return Err("stored attachment thumbnail must not include a path or URL".into());
            }
        }
    }
    Ok(())
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
    validate_opaque_parts(requested)?;

    let config = ctx
        .load_config()
        .await
        .map_err(|error| format!("stored attachment configuration unavailable: {error}"))?;
    let messages =
        crate::threads::store::get_messages(ctx, config.workspace_dir.clone(), thread_id)?;
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

    validate_staged_files(ctx, thread_id, &config, &entries)
        .await
        .map_err(|error| error.replace("staged", "stored"))?;

    let mut resolved = Vec::with_capacity(entries.len());
    for mut entry in entries {
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

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod tests;
