//! Resolve typed native-channel media through TinyChannels' scoped media port.

use tinybus::module::ModuleClient;
use tinychannels_bus::{
    ChannelInboundEnvelope, ContentPart, HostVoiceScope, MediaKind, MediaReference,
    module_media::{MediaReadBegin, MediaReadChunk, MediaReadFinish},
};
use tinyinference_llm::message::Message;

use crate::{
    channels::context::{ChannelRuntimeContext, module_client},
    core::runtime::DispatchCtx,
};

const MAX_MEDIA_BYTES: usize = tinychannels_bus::module_media::MEDIA_UPLOAD_MAX_BYTES;

pub(super) fn requires_typed_preparation(envelope: &ChannelInboundEnvelope) -> bool {
    envelope.parts.iter().any(|part| {
        matches!(
            part,
            ContentPart::Image { .. }
                | ContentPart::File { .. }
                | ContentPart::Audio { .. }
                | ContentPart::Video { .. }
                | ContentPart::VoiceNote { .. }
        )
    })
}

/// Resolve only opaque references carried by the trusted module envelope.
/// Provider URLs and paths are never used as a filesystem or network source.
pub(super) async fn resolve_typed_message(
    ctx: &ChannelRuntimeContext,
    config: &crate::config::Config,
    envelope: &ChannelInboundEnvelope,
    thread_id: &str,
) -> Result<Option<ResolvedNativeMessage>, String> {
    if !requires_typed_preparation(envelope) {
        return Ok(None);
    }
    validate_native_parts(&envelope.parts)?;
    let scope = media_scope(envelope)?;
    let mut parts = envelope.parts.clone();
    let mut staged_parts = Vec::new();
    let mut image_count = 0usize;
    let mut file_count = 0usize;
    let (max_images, image_mb) = config.multimodal.effective_limits();
    let (configured_files, file_mb, _) = config.multimodal_files.effective_limits();
    let max_files = if config.multimodal_files.max_files == 0 {
        0
    } else {
        configured_files
    };
    let image_limit = image_mb.saturating_mul(1024 * 1024).min(MAX_MEDIA_BYTES);
    let file_limit = file_mb.saturating_mul(1024 * 1024).min(MAX_MEDIA_BYTES);
    for part in &parts {
        match part {
            ContentPart::Image { media, .. } => {
                image_count += 1;
                validate_reference(
                    media,
                    true,
                    image_limit,
                    &config.multimodal_files.allowed_mime_types,
                )?;
            }
            ContentPart::File { media, .. } => {
                file_count += 1;
                validate_reference(
                    media,
                    false,
                    file_limit,
                    &config.multimodal_files.allowed_mime_types,
                )?;
            }
            _ => {}
        }
    }
    if image_count > max_images {
        return Err("native image attachments exceed their count limit".into());
    }
    if file_count > max_files {
        return Err("native file attachments are disabled or exceed their count limit".into());
    }
    let client = module_client(ctx).await;
    let dispatch = ctx.dispatch_ctx();
    for part in &mut parts {
        let (media, image, name) = match part {
            ContentPart::Image { media, .. } => (
                media,
                true,
                media.file_name.clone().unwrap_or_else(|| "image".into()),
            ),
            ContentPart::File {
                media, file_name, ..
            } => (
                media,
                false,
                file_name
                    .clone()
                    .or_else(|| media.file_name.clone())
                    .unwrap_or_else(|| "attachment".into()),
            ),
            ContentPart::Audio { .. } | ContentPart::Video { .. } => {
                return Err("native audio/video attachments are not supported".into());
            }
            ContentPart::VoiceNote { voice_note } => {
                let transcript = voice_note
                    .transcript
                    .as_ref()
                    .expect("validated transcript");
                let replacement = ContentPart::Text {
                    text: transcript.text.trim().to_owned(),
                    markdown_dialect: None,
                };
                *part = replacement;
                continue;
            }
            ContentPart::Text { .. } => continue,
            _ => return Err("native channel content kind is not supported".into()),
        };
        let byte_limit = if image { image_limit } else { file_limit };
        let bytes = read_scoped_media(
            &client,
            &dispatch,
            envelope.channel.id.as_str(),
            &scope,
            media,
            byte_limit,
        )
        .await?;
        let mime = media
            .content_type
            .as_deref()
            .ok_or_else(|| "native media has no content type".to_owned())?;
        let staged = crate::threads::attachments::stage_native_channel_part(
            &dispatch,
            thread_id,
            config,
            part,
            &name,
            mime,
            &bytes,
        )
        .await?;
        *part = staged.clone();
        staged_parts.push(staged);
    }

    let message = crate::agent::message_convert::user_message_from_parts("", &parts)?;
    Ok(Some(ResolvedNativeMessage {
        message,
        staged_parts,
    }))
}

pub(super) struct ResolvedNativeMessage {
    pub(super) message: Message,
    pub(super) staged_parts: Vec<ContentPart>,
}

fn media_scope(envelope: &ChannelInboundEnvelope) -> Result<HostVoiceScope, String> {
    let configured_instance = envelope
        .channel
        .account_id
        .as_deref()
        .filter(|account| !account.trim().is_empty())
        .ok_or_else(|| "native media has no configured account scope".to_owned())?;
    if envelope.channel.id.trim().is_empty()
        || envelope.conversation.id.trim().is_empty()
        || envelope.message_id.trim().is_empty()
    {
        return Err("native media has an incomplete conversation scope".into());
    }
    Ok(HostVoiceScope {
        configured_instance: configured_instance.to_owned(),
        channel: envelope.channel.clone(),
        conversation: envelope.conversation.clone(),
        turn_id: envelope.message_id.clone(),
    })
}

fn validate_native_parts(parts: &[ContentPart]) -> Result<(), String> {
    for part in parts {
        match part {
            ContentPart::Text { .. } | ContentPart::Image { .. } | ContentPart::File { .. } => {}
            ContentPart::VoiceNote { voice_note } => {
                let transcript = voice_note
                    .transcript
                    .as_ref()
                    .filter(|transcript| {
                        transcript.state == tinychannels_bus::VoiceTranscriptState::Complete
                    })
                    .map(|transcript| transcript.text.trim())
                    .filter(|text| !text.is_empty());
                if transcript.is_none() {
                    return Err("native voice transcript is unavailable".into());
                }
            }
            ContentPart::Audio { .. } | ContentPart::Video { .. } => {
                return Err("native audio/video attachments are not supported".into());
            }
            _ => return Err("native channel content kind is not supported".into()),
        }
    }
    Ok(())
}

fn validate_reference(
    media: &MediaReference,
    image: bool,
    byte_limit: usize,
    allowed_file_mimes: &[String],
) -> Result<(), String> {
    let opaque_id = media
        .opaque_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| "native media reference is missing its opaque id".to_owned())?;
    if opaque_id.len() > tinychannels_bus::module_media::MEDIA_UPLOAD_BEGIN_MAX_BYTES
        || media.path.is_some()
        || media.url.is_some()
        || media.local_cache_path.is_some()
    {
        return Err("native media reference is not an opaque module reference".into());
    }
    if media
        .size_bytes
        .is_some_and(|size| size == 0 || size > byte_limit as u64)
    {
        return Err("native media exceeds its declared byte limit".into());
    }
    let mime = media
        .content_type
        .as_deref()
        .filter(|mime| !mime.trim().is_empty())
        .ok_or_else(|| "native media has no content type".to_owned())?;
    if image {
        if media.kind != MediaKind::Image {
            return Err("native media kind does not match its image part".into());
        }
        if !matches!(
            mime,
            "image/png" | "image/jpeg" | "image/webp" | "image/gif"
        ) {
            return Err("native image MIME type is unsupported".into());
        }
    } else {
        if !matches!(media.kind, MediaKind::Document | MediaKind::Unknown) {
            return Err("native media kind does not match its file part".into());
        }
        if !allowed_file_mimes.iter().any(|allowed| allowed == mime) {
            return Err("native file MIME type is unsupported".into());
        }
    }
    Ok(())
}

fn validate_actual_media_size(actual: usize, declared: usize, maximum: usize) -> Result<(), String> {
    if actual == 0 || actual != declared || actual > maximum || actual > MAX_MEDIA_BYTES {
        return Err("native media bytes exceed or disagree with their declared limit".into());
    }
    Ok(())
}

async fn read_scoped_media(
    client: &ModuleClient,
    dispatch: &DispatchCtx,
    instance: &str,
    scope: &HostVoiceScope,
    media: &MediaReference,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let begin = client
        .call_confidential::<tinychannels_bus::MediaReadInfo>(
            "tinychannels",
            tinychannels_bus::methods::BEGIN_MEDIA_READ,
            (MediaReadBegin {
                instance: instance.to_owned(),
                scope: scope.clone(),
                media: media.clone(),
                max_bytes: max_bytes as u64,
            },),
        )
        .await
        .map_err(|_| "native media could not be resolved in this turn".to_owned())?;
    let mut lease = MediaReadLease::new(client.clone(), dispatch.clone(), begin.handle.clone());
    if begin.byte_count == 0
        || begin.byte_count > max_bytes
        || begin.byte_count > MAX_MEDIA_BYTES
        || begin.media.opaque_id != media.opaque_id
        || begin.media.kind != media.kind
        || begin.media.path.is_some()
        || begin.media.url.is_some()
        || begin.media.local_cache_path.is_some()
        || begin.content_type != media.content_type.as_deref().unwrap_or_default()
        || media
            .size_bytes
            .is_some_and(|size| size != begin.byte_count as u64)
    {
        return Err("native media read metadata does not match its scoped reference".into());
    }
    let expected_chunks = begin
        .byte_count
        .div_ceil(tinychannels_bus::module_media::MEDIA_READ_CHUNK_BYTES);
    if begin.chunk_count as usize != expected_chunks {
        return Err("native media read chunk count is invalid".into());
    }
    let mut bytes = Vec::with_capacity(begin.byte_count);
    for sequence in 0..begin.chunk_count {
        let chunk = client
            .call_confidential::<tinychannels_bus::MediaReadChunkResult>(
                "tinychannels",
                tinychannels_bus::methods::READ_MEDIA_CHUNK,
                (MediaReadChunk {
                    handle: begin.handle.clone(),
                    sequence,
                },),
            )
            .await
            .map_err(|_| "native media read was interrupted".to_owned())?;
        let expected_done = sequence + 1 == begin.chunk_count;
        if chunk.sequence != sequence || chunk.done != expected_done {
            return Err("native media read returned an invalid chunk sequence".into());
        }
        if chunk.bytes.len() > tinychannels_bus::module_media::MEDIA_READ_CHUNK_BYTES {
            return Err("native media read returned an oversized chunk".into());
        }
        if bytes.len().saturating_add(chunk.bytes.len()) > begin.byte_count
            || bytes.len().saturating_add(chunk.bytes.len()) > max_bytes
        {
            return Err("native media bytes exceed their declared limit".into());
        }
        bytes.extend(chunk.bytes);
    }
    validate_actual_media_size(bytes.len(), begin.byte_count, max_bytes)?;
    lease.abort().await?;
    Ok(bytes)
}

struct MediaReadLease {
    client: ModuleClient,
    dispatch: DispatchCtx,
    handle: Option<tinychannels_bus::MediaReadHandle>,
}

impl MediaReadLease {
    fn new(
        client: ModuleClient,
        dispatch: DispatchCtx,
        handle: tinychannels_bus::MediaReadHandle,
    ) -> Self {
        Self {
            client,
            dispatch,
            handle: Some(handle),
        }
    }

    async fn abort(&mut self) -> Result<(), String> {
        let Some(handle) = self.handle.as_ref().cloned() else {
            return Ok(());
        };
        self.client
            .call_confidential::<()>(
                "tinychannels",
                tinychannels_bus::methods::ABORT_MEDIA_READ,
                (MediaReadFinish(handle),),
            )
            .await
            .map_err(|_| "native media read cleanup failed".to_owned())?;
        self.handle = None;
        Ok(())
    }
}

impl Drop for MediaReadLease {
    fn drop(&mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };
        let client = self.client.clone();
        self.dispatch.spawn(async move {
            let _ = client
                .call_confidential::<()>(
                    "tinychannels",
                    tinychannels_bus::methods::ABORT_MEDIA_READ,
                    (MediaReadFinish(handle),),
                )
                .await;
        });
    }
}

#[cfg(test)]
#[path = "native_media_tests.rs"]
mod tests;
