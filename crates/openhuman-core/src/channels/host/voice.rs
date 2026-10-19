//! Voice host ports for the TinyChannels module: scoped speech-to-text,
//! text-to-speech and the per-turn spoken-reply policy.
//!
//! The module calls these over the `ChannelsHost` callback object. Everything
//! here logs request ids, sizes and durations; transcripts, reply text and
//! audio are never logged.

use crate::config::Config;
use crate::config::schema::{ChannelVoiceConfig, VoiceReplyMode};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use tinychannels_bus::{
    ChannelInboundEnvelope, ContentPart, HostSpeechSynthesisRequest, HostSpeechSynthesisResult,
    HostTranscriptionRequest, HostTranscriptionResult, HostVoiceReplyMode, HostVoiceReplyPolicy,
    HostVoiceScope, HostVoiceTranscriptionRequest,
};

const LOG_PREFIX: &str = "[channels_voice]";
/// Conservative speaking rate used to bound synthesis before the backend call.
const SPOKEN_CHARS_PER_SEC: usize = 15;
const MAX_TRACKED_TURNS: usize = 1024;

/// Verified facts about an inbound turn, recorded by the dispatcher.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct VoiceTurnFacts {
    pub inbound_has_voice: bool,
    pub explicitly_requested: bool,
}

static TURNS: Mutex<VecDeque<(String, VoiceTurnFacts)>> = Mutex::new(VecDeque::new());

fn turn_key(conversation_id: &str, turn_id: &str) -> String {
    format!("{conversation_id}\u{1f}{turn_id}")
}

/// Remember the facts of an inbound turn so the reply policy can read them.
pub(crate) fn record_turn(conversation_id: &str, turn_id: &str, facts: VoiceTurnFacts) {
    let key = turn_key(conversation_id, turn_id);
    let mut turns = TURNS.lock();
    turns.retain(|(existing, _)| existing != &key);
    if turns.len() >= MAX_TRACKED_TURNS {
        turns.pop_front();
    }
    turns.push_back((key, facts));
}

fn turn_facts(conversation_id: &str, turn_id: &str) -> VoiceTurnFacts {
    let key = turn_key(conversation_id, turn_id);
    TURNS
        .lock()
        .iter()
        .find(|(existing, _)| existing == &key)
        .map(|(_, facts)| *facts)
        .unwrap_or_default()
}

/// Record an inbound envelope's voice facts. The inbound message id is the
/// turn id the module echoes back in its voice scope.
pub(crate) fn record_inbound_envelope(envelope: &ChannelInboundEnvelope) {
    let inbound_has_voice = envelope
        .parts
        .iter()
        .any(|part| matches!(part, ContentPart::VoiceNote { .. }));
    record_turn(
        &envelope.conversation.id,
        &envelope.message_id,
        VoiceTurnFacts {
            inbound_has_voice,
            explicitly_requested: false,
        },
    );
}

fn thread_key(scope: &HostVoiceScope) -> &str {
    scope
        .conversation
        .thread_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .unwrap_or(&scope.conversation.id)
}

fn wire_mode(mode: VoiceReplyMode) -> HostVoiceReplyMode {
    match mode {
        VoiceReplyMode::Mirror => HostVoiceReplyMode::Mirror,
        VoiceReplyMode::Always => HostVoiceReplyMode::Always,
        VoiceReplyMode::Never => HostVoiceReplyMode::Never,
        VoiceReplyMode::SpeakReply => HostVoiceReplyMode::SpeakReply,
    }
}

/// Resolve the spoken-reply policy for one scoped turn.
pub(crate) fn resolve_policy(
    voice: &ChannelVoiceConfig,
    scope: &HostVoiceScope,
) -> HostVoiceReplyPolicy {
    let mode = voice.resolve_mode(&scope.configured_instance, thread_key(scope));
    let facts = turn_facts(&scope.conversation.id, &scope.turn_id);
    HostVoiceReplyPolicy {
        mode: wire_mode(mode),
        inbound_has_voice: facts.inbound_has_voice,
        explicitly_requested: facts.explicitly_requested,
    }
}

fn validate_scope(scope: &HostVoiceScope) -> anyhow::Result<()> {
    if scope.configured_instance.trim().is_empty()
        || scope.conversation.id.trim().is_empty()
        || scope.turn_id.trim().is_empty()
    {
        anyhow::bail!("voice scope is incomplete");
    }
    Ok(())
}

/// Reject replies that cannot fit the channel's outbound duration cap.
pub(crate) fn check_outbound_length(
    voice: &ChannelVoiceConfig,
    channel: &str,
    text: &str,
) -> anyhow::Result<()> {
    let max_secs = voice.resolve_max_outbound_secs(channel) as usize;
    if text.chars().count() > max_secs.saturating_mul(SPOKEN_CHARS_PER_SEC) {
        anyhow::bail!("reply exceeds the maximum spoken duration ({max_secs}s)");
    }
    Ok(())
}

/// Speech-to-text and text-to-speech over OpenHuman's voice domain.
pub(crate) struct ChannelVoiceHost {
    pub config: Arc<Config>,
}

impl ChannelVoiceHost {
    /// Unscoped transcription, honouring the user's local/cloud STT setting.
    pub(crate) async fn transcribe_host(
        &self,
        request: HostTranscriptionRequest,
    ) -> anyhow::Result<HostTranscriptionResult> {
        let request_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        let provider = crate::voice::effective_stt_provider(&self.config);
        tracing::debug!(
            "{LOG_PREFIX} stt start request_id={request_id} provider={provider} bytes_b64={}",
            request.audio_base64.len()
        );
        let stt = crate::voice::create_stt_provider(&provider, "", &self.config)?;
        let outcome = stt
            .transcribe(
                &self.config,
                &request.audio_base64,
                request.mime_type.as_deref(),
                request.file_name.as_deref(),
                request.language.as_deref(),
            )
            .await
            .map_err(|e| {
                tracing::warn!(
                    "{LOG_PREFIX} stt failed request_id={request_id} elapsed_ms={}",
                    started.elapsed().as_millis()
                );
                anyhow::anyhow!(e)
            })?;
        tracing::debug!(
            "{LOG_PREFIX} stt done request_id={request_id} elapsed_ms={} chars={}",
            started.elapsed().as_millis(),
            outcome.value.text.chars().count()
        );
        Ok(HostTranscriptionResult {
            text: outcome.value.text,
            language: request.language,
            duration_secs: None,
        })
    }

    /// Scoped transcription used by the shared voice pipeline.
    pub(crate) async fn transcribe_voice(
        &self,
        request: HostVoiceTranscriptionRequest,
    ) -> anyhow::Result<HostTranscriptionResult> {
        validate_scope(&request.scope)?;
        self.transcribe_host(HostTranscriptionRequest {
            audio_base64: request.audio_base64,
            mime_type: request.mime_type,
            file_name: request.file_name,
            language: request.language,
        })
        .await
    }

    /// Scoped synthesis through the hosted reply-speech endpoint.
    pub(crate) async fn synthesize_speech(
        &self,
        request: HostSpeechSynthesisRequest,
    ) -> anyhow::Result<HostSpeechSynthesisResult> {
        validate_scope(&request.scope)?;
        let channel = request.scope.configured_instance.as_str();
        let voice_cfg = &self.config.channel_voice;
        check_outbound_length(voice_cfg, channel, &request.text)?;
        let request_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        let voice = request
            .voice
            .clone()
            .or_else(|| voice_cfg.resolve_voice(channel).map(str::to_owned));
        tracing::debug!(
            "{LOG_PREFIX} tts start request_id={request_id} channel={channel} chars={}",
            request.text.chars().count()
        );
        let opts = crate::voice::reply_speech::ReplySpeechOptions {
            voice_id: voice,
            model_id: None,
            output_format: request.format,
            voice_settings: None,
        };
        let outcome =
            crate::voice::reply_speech::synthesize_reply(&self.config, &request.text, &opts)
                .await
                .map_err(|e| {
                    tracing::warn!(
                        "{LOG_PREFIX} tts failed request_id={request_id} elapsed_ms={}",
                        started.elapsed().as_millis()
                    );
                    anyhow::anyhow!(e)
                })?;
        let speech = outcome.value;
        if speech.audio_base64.is_empty() {
            anyhow::bail!("speech synthesis returned no audio");
        }
        tracing::debug!(
            "{LOG_PREFIX} tts done request_id={request_id} elapsed_ms={} bytes_b64={}",
            started.elapsed().as_millis(),
            speech.audio_base64.len()
        );
        Ok(HostSpeechSynthesisResult {
            audio_base64: speech.audio_base64,
            mime_type: speech.audio_mime,
            duration_ms: None,
        })
    }

    /// Host-authoritative reply policy for the scoped turn.
    pub(crate) fn reply_policy(&self, scope: &HostVoiceScope) -> HostVoiceReplyPolicy {
        let policy = resolve_policy(&self.config.channel_voice, scope);
        tracing::debug!(
            "{LOG_PREFIX} policy channel={} mode={:?} inbound_voice={}",
            scope.configured_instance,
            policy.mode,
            policy.inbound_has_voice
        );
        policy
    }
}

#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
