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
        config: config.clone(),
        core: ctx.downgrade(),
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
    let extractor = Arc::new(OwnedDocumentsExtractor { ctx: ctx.downgrade() });
    let model = MediaRequestModel::new(
        inner,
        request_config,
        |_, request| {
            let mime_hints = typed_mime_hints(request);
            let mut scope = super::super::take_request_scope(&mut request.metadata);
            scope.mime_hints = mime_hints;
            Ok(scope)
        },
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
        let Some((resolved_modality, path, mut mime, bytes, recovered)) =
            self.resolve_block(&block, scope).await.map_err(|error| error.to_string())?
        else {
            return Err("unsupported media source".into());
        };
        if bytes.len() > max_bytes || resolved_modality != modality {
            return Err("attachment exceeds configured file limit".into());
        }
        if modality == InputModality::Image {
            mime = tinyagents_harness::multimodal::mime::image_mime_from_magic(&bytes)
                .map(str::to_owned)
                .or_else(|| tinyagents_harness::multimodal::mime::detect_image_mime(
                    Some(std::path::Path::new(&path)),
                    &bytes,
                    scope.mime_hints.get(source).map(String::as_str),
                ))
                .ok_or_else(|| "image attachment type cannot be determined".to_string())?;
        } else if mime == "application/octet-stream" {
            mime = scope
                .mime_hints
                .get(source)
                .cloned()
                .or_else(|| tinyagents_harness::multimodal::mime::detect_file_mime(
                    Some(std::path::Path::new(&path)),
                    &bytes,
                    None,
                ))
                .unwrap_or(mime);
        }
        let mut resolved = ResolvedMedia::new(ResolvedAttachment {
            size_bytes: bytes.len(),
            bytes,
            name: path,
            mime,
        });
        if recovered {
            let annotation = format!(
                "[Recovered attachment: {}; {} bytes; workspace path: {}]",
                resolved.attachment.mime,
                resolved.attachment.size_bytes,
                resolved.attachment.name
            );
            resolved = resolved.with_annotation(annotation);
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
            let key = self.fallback_key(&attachment.name, &attachment.mime, &attachment.bytes);
            if let Some(text) = self.cached_fallback(&attachment.name, &key, _scope).await {
                return Ok(Some(text));
            }
            let text = format!(
                "Image readout (untrusted):\n{}",
                self.image_readout(&attachment.mime, &attachment.bytes)
                    .await
                    .map_err(|error| error.to_string())?
            );
            let text = self.limit_context(text);
            self.save_fallback(&attachment.name, &key, &text, _scope).await;
            return Ok(Some(text));
        }
        #[cfg(feature = "documents")]
        if matches!(attachment.mime.as_str(), "application/pdf"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        {
            let key = self.fallback_key(&attachment.name, &attachment.mime, &attachment.bytes);
            if let Some(text) = self.cached_fallback(&attachment.name, &key, _scope).await {
                return Ok(Some(text));
            }
            let (text, cacheable) = self.fallback(
                modality,
                &attachment.name,
                &attachment.mime,
                &attachment.bytes,
            ).await.map_err(|error| error.to_string())?;
            let body = text.split_once('\n').map(|(_, body)| body.to_owned()).unwrap_or(text);
            if cacheable {
                self.save_fallback(&attachment.name, &key, &body, _scope).await;
            }
            return Ok(Some(body));
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
        if let Some(text) = self.fallback_cache.lock().ok()?.get(&key).cloned() {
            return Some(text);
        }
        let text = self.cached_fallback(&attachment.name, &key, scope).await?;
        if let Ok(mut cache) = self.fallback_cache.lock() {
            if cache.len() < 16 {
                cache.insert(key, text.clone());
            }
        }
        Some(text)
    }

    async fn put(
        &self,
        scope: &AttachmentAccessScope,
        _modality: InputModality,
        attachment: &ResolvedAttachment,
        text: &str,
    ) {
        let key = self.fallback_key(&attachment.name, &attachment.mime, &attachment.bytes);
        if let Ok(mut cache) = self.fallback_cache.lock() {
            if cache.len() < 16 {
                cache.insert(key.clone(), text.to_owned());
            }
        }
        self.save_fallback(&attachment.name, &key, text, scope).await;
    }
}

impl MediaHistoryFallback<AttachmentAccessScope> for AttachmentModel {
    fn omitted(&self, scope: &AttachmentAccessScope, _modality: InputModality, source: &str) -> String {
        let block = ContentBlock::Document(MediaRef::Path { path: source.to_owned(), media_type: None });
        let hint = super::historical_source_hint(&block, scope);
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
        if !error.contains(super::super::SOURCE_MISSING_MARKER)
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
        let hint = super::historical_source_hint(&block, scope);
        Some(format!("[Earlier attachment unavailable or omitted at workspace path: {hint}]") )
    }
}

fn typed_mime_hints(request: &ModelRequest) -> std::collections::HashMap<String, String> {
    let mut hints = std::collections::HashMap::new();
    for message in &request.messages {
        let Message::User(user) = message else { continue };
        for block in &user.content {
            match block {
                ContentBlock::Image(image) => {
                    if let Some(mime) = &image.mime_type {
                        hints.insert(image.url.clone(), mime.clone());
                    }
                }
                ContentBlock::Audio(media)
                | ContentBlock::Video(media)
                | ContentBlock::Document(media) => {
                    let (source, mime) = match media {
                        MediaRef::Path { path, media_type } => (path.clone(), media_type.clone()),
                        MediaRef::Url { url, media_type } => (url.clone(), media_type.clone()),
                        MediaRef::Base64 { data, media_type } => (
                            format!("data:{media_type};base64,{data}"),
                            Some(media_type.clone()),
                        ),
                    };
                    if let Some(mime) = mime { hints.insert(source, mime); }
                }
                _ => ()
            }
        }
    }
    hints
}

struct OwnedDocumentsExtractor {
    ctx: Option<std::sync::Weak<crate::core::runtime::CoreContext>>,
}

#[async_trait]
impl TextExtractor for OwnedDocumentsExtractor {
    fn handles(&self, mime: &str) -> bool { mime == "application/pdf" }

    async fn extract(&self, mime: &str, bytes: &[u8]) -> Result<String, String> {
        let ctx = crate::core::runtime::DispatchCtx::from_weak(self.ctx.as_ref());
        crate::agent::multimodal::DocumentsTextExtractor { ctx: &ctx }
            .extract(mime, bytes).await
    }
}
