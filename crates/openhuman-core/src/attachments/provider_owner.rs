//! OpenHuman adapters for TinyAgents' generic media request decorator.
use super::*;
use async_trait::async_trait;
use tinyagents_harness::multimodal::{
    config::{FileLimits, ImageLimits}, MediaFallbackCache, MediaFallbackRenderer,
    MediaHistoryFallback, MediaRequestConfig, MediaRequestModel, MediaSourceResolver,
    ResolvedAttachment, ResolvedMedia, TextExtractor,
};

pub(super) fn wrap(
    ctx: &crate::core::runtime::DispatchCtx,
    inner: Arc<dyn ChatModel<()>>,
    config: Arc<Config>,
    profile: ModelProfile,
) -> Arc<dyn ChatModel<()>> {
    let host = Arc::new(AttachmentModel {
        inner: inner.clone(),
        config: config.clone(),
        core: ctx.downgrade(),
        profile: profile.clone(),
        fallback_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
    });
    let (max_images, image_mb) = config.multimodal.effective_limits();
    let (max_files, file_mb, text_limit) = config.multimodal_files.effective_limits();
    let request_config = MediaRequestConfig {
        images: ImageLimits {
            max_images,
            max_image_size_mb: image_mb,
            allow_remote_fetch: config.multimodal.allow_remote_fetch,
        },
        files: FileLimits {
            max_files: if config.multimodal_files.max_files == 0 { 0 } else { max_files },
            max_file_size_mb: file_mb,
            max_extracted_text_chars: text_limit,
            allow_remote_fetch: config.multimodal_files.allow_remote_fetch,
            allowed_mime_types: config.multimodal_files.allowed_mime_types.clone(),
        },
        max_historical_media_items: MAX_HISTORICAL_MEDIA_BLOCKS,
    };
    let extractor = Arc::new(OwnedDocumentsExtractor { ctx: ctx.clone() });
    let model = MediaRequestModel::new(
        inner,
        request_config,
        |_, request| Ok(super::take_request_scope(&mut request.metadata)),
        host.clone(),
        extractor,
    )
    .with_profile(profile)
    .with_fallback_renderer(host.clone())
    .with_fallback_cache(host.clone())
    .with_history_fallback(host.clone());
    Arc::new(model)
}

#[async_trait]
impl MediaSourceResolver<AttachmentAccessScope> for AttachmentModel {
    async fn resolve(
        &self,
        scope: &AttachmentAccessScope,
        modality: InputModality,
        source: &str,
        max_bytes: usize,
    ) -> Result<ResolvedMedia, String> {
        let block = match modality {
            InputModality::Image => ContentBlock::Image(ImageRef {
                url: source.to_owned(),
                mime_type: None,
            }),
            InputModality::Audio | InputModality::Video | InputModality::Document => {
                let media = if source.starts_with("data:")
                    || source.starts_with("http://")
                    || source.starts_with("https://")
                {
                    MediaRef::Url { url: source.to_owned(), media_type: None }
                } else {
                    MediaRef::Path { path: source.to_owned(), media_type: None }
                };
                match modality {
                    InputModality::Audio => ContentBlock::Audio(media),
                    InputModality::Video => ContentBlock::Video(media),
                    _ => ContentBlock::Document(media),
                }
            }
        };
        let Some((resolved_modality, path, mime, bytes, recovered)) =
            self.resolve_block(&block, scope).await.map_err(|error| error.to_string())?
        else {
            return Err("unsupported media source".into());
        };
        if bytes.len() > max_bytes || resolved_modality != modality {
            return Err("attachment exceeds configured file limit".into());
        }
        let mut resolved = ResolvedMedia::new(ResolvedAttachment {
            size_bytes: bytes.len(),
            bytes,
            name: path,
            mime,
        });
        if recovered {
            resolved = resolved.with_annotation(format!(
                "[Recovered attachment: {}; {} bytes; workspace path: {}]",
                resolved.attachment.mime,
                resolved.attachment.size_bytes,
                resolved.attachment.name
            ));
        }
        Ok(resolved)
    }
}

#[async_trait]
impl MediaFallbackRenderer<AttachmentAccessScope> for AttachmentModel {
    async fn render(
        &self,
        _scope: &AttachmentAccessScope,
        modality: InputModality,
        attachment: &ResolvedAttachment,
    ) -> Result<Option<String>, String> {
        if modality == InputModality::Image {
            let text = format!(
                "[Attachment: {}; {} bytes; workspace path: {}]\nImage readout:\n{}",
                attachment.mime,
                attachment.size_bytes,
                attachment.name,
                self.image_readout(&attachment.mime, &attachment.bytes)
                    .await
                    .map_err(|error| error.to_string())?
            );
            return Ok(Some(self.limit_context(text)));
        }
        #[cfg(feature = "documents")]
        if matches!(attachment.mime.as_str(), "application/pdf"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        {
            let (text, cacheable) = self.document_fallback(attachment).await?;
            return Ok(cacheable.then_some(text));
        }
        Ok(None)
    }
}

#[async_trait]
impl MediaFallbackCache<AttachmentAccessScope> for AttachmentModel {
    async fn get(
        &self,
        scope: &AttachmentAccessScope,
        _modality: InputModality,
        attachment: &ResolvedAttachment,
    ) -> Option<String> {
        let key = self.fallback_key(&attachment.name, &attachment.mime, &attachment.bytes);
        self.cached_fallback(&attachment.name, &key, scope).await
    }

    async fn put(
        &self,
        scope: &AttachmentAccessScope,
        _modality: InputModality,
        attachment: &ResolvedAttachment,
        text: &str,
    ) {
        let key = self.fallback_key(&attachment.name, &attachment.mime, &attachment.bytes);
        self.save_fallback(&attachment.name, &key, text, scope).await;
    }
}

impl MediaHistoryFallback<AttachmentAccessScope> for AttachmentModel {
    fn omitted(&self, scope: &AttachmentAccessScope, _modality: InputModality, source: &str) -> String {
        let block = ContentBlock::Document(MediaRef::Path { path: source.to_owned(), media_type: None });
        let hint = super::provider::historical_source_hint(&block, scope);
        format!("[Earlier attachment omitted by history budget; workspace path: {hint}]")
    }

    fn unavailable(
        &self,
        scope: &AttachmentAccessScope,
        modality: InputModality,
        source: &str,
        error: &str,
    ) -> Option<String> {
        let error = error.to_owned();
        if !error.contains(super::SOURCE_MISSING_MARKER)
            && !error.contains("attachments are disabled for this origin")
            && !error.contains("remote attachment fetch is disabled")
        {
            return None;
        }
        let block = match modality {
            InputModality::Image => ContentBlock::Image(ImageRef { url: source.to_owned(), mime_type: None }),
            InputModality::Audio => ContentBlock::Audio(MediaRef::Path { path: source.to_owned(), media_type: None }),
            InputModality::Video => ContentBlock::Video(MediaRef::Path { path: source.to_owned(), media_type: None }),
            InputModality::Document => ContentBlock::Document(MediaRef::Path { path: source.to_owned(), media_type: None }),
        };
        let hint = super::provider::historical_source_hint(&block, scope);
        Some(format!("[Earlier attachment unavailable or omitted at workspace path: {hint}]") )
    }
}

struct OwnedDocumentsExtractor { ctx: crate::core::runtime::DispatchCtx }

#[async_trait]
impl TextExtractor for OwnedDocumentsExtractor {
    fn handles(&self, mime: &str) -> bool { mime == "application/pdf" }

    async fn extract(&self, mime: &str, bytes: &[u8]) -> Result<String, String> {
        crate::agent::multimodal::DocumentsTextExtractor { ctx: &self.ctx }
            .extract(mime, bytes).await
    }
}
