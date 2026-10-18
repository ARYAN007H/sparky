//! Calling the `tinyvoice` module: the voice primitives, over the bus.
//!
//! Each function here is the host half of one method on
//! `ai.tinyhumans.tinyvoice.Voice`. They exist so the voice domain does not
//! have to know about proxies, base64 framing, or wire error names — a caller
//! asks a question about a transcript or a buffer and gets an answer.
//!
//! # A call costs about 15 microseconds
//!
//! Measured on the real loaded module (`bench_call` in the `tinyvoice` repo):
//! ~15 µs per round trip, against a 20 ms audio frame. A TinyBus module shares
//! this address space — a call is a channel send and a JSON hop, not IPC.
//!
//! So there is no per-call budget to protect and nothing here is too hot to go
//! over the bus, including the hotkey-driven recording session state machine.
//!
//! **What stays on this side is decided by the audio callback, not by cost.**
//! `cpal` delivers on a realtime thread where blocking is a dropout, so the
//! callback converts the sample format and forwards raw interleaved samples;
//! every transform happens in an async worker that calls this module. That is
//! *less* work on the audio thread than the in-process version did, not more.
//!
//! # Failure is not fatal here
//!
//! Every function returns a [`VoiceCallError`] the caller can fall back from,
//! and the callers do. A module that will not load must degrade voice to its
//! pre-module behaviour — deferring to the agent, or skipping a filter — rather
//! than taking dictation down with it. The one thing none of them may do is
//! guess: see [`is_hallucinated`].

use tinyvoice_bus::names::methods;

use super::{ops, registry};
use crate::config::Config;

/// Registry id of the module these calls go to.
const MODULE_ID: &str = "tinyvoice";

/// Why a voice call did not produce an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceCallError {
    /// The module is not loaded and cannot be: unsupported host, downloads off,
    /// disabled in config, or a load that already failed in this process.
    Unavailable(String),
    /// The call itself failed — a malformed payload, or a refused argument.
    Failed(String),
}

impl std::fmt::Display for VoiceCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) | Self::Failed(message) => f.write_str(message),
        }
    }
}

/// Which hallucination list applies.
///
/// The contract's own type under the name the voice domain has always used for
/// it. It was redeclared here — with a comment saying it had to be, "because
/// this crate does not depend on `tinyvoice`" — and that is no longer true:
/// `tinyvoice-bus` is exactly that dependency, and it costs `serde` and
/// nothing else.
pub use tinyvoice_bus::transcript::Mode as HallucinationMode;

/// The wire value for a screening mode.
///
/// The interface takes the mode as a plain string argument rather than a JSON
/// value, so this reaches the same spelling the contract's `rename_all =
/// "snake_case"` derive produces without a `serde_json` round trip. The match
/// is exhaustive, so a variant added upstream is a compile error here rather
/// than a mode that silently screens as something else.
fn hallucination_mode_wire(mode: HallucinationMode) -> &'static str {
    match mode {
        HallucinationMode::Dictation => "dictation",
        HallucinationMode::Conversation => "conversation",
    }
}

/// Recording-session vocabulary shared with the loaded TinyVoice module.
pub use tinyvoice_bus::{
    GateVerdict, RecordingGates, SessionAction, SessionAdvance, SessionInput, SessionPhase,
    SessionStatus,
};

/// Native capture lease types shared with the loaded TinyVoice module.
pub use tinyvoice_bus::capture::{
    CaptureError, CaptureHandle, CaptureResult, MicrophonePermission, RecordingFinishRequest,
    RecordingStartRequest,
};

/// Hotkey lifecycle types shared with the loaded TinyVoice module.
pub use tinyvoice_bus::hotkey::{
    ActivationMode, HostKeyFact, HotkeyBatch, HotkeyError, HotkeyEvent, HotkeyFeedRequest,
    HotkeyHandle, HotkeyHandleRequest, HotkeyReadRequest, HotkeyReply, HotkeyRequest,
    HotkeyReserveRequest, HotkeySource, HotkeyStartRequest, HotkeyState, SequencedHostFact,
};

/// Apply the pre-transcription recording gates (minimum duration, silence).
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
pub async fn gate_recording(
    config: &Config,
    duration_secs: f32,
    peak_rms: f32,
    gates: RecordingGates,
) -> Result<GateVerdict, VoiceCallError> {
    let json: String = call(
        config,
        methods::GATE_RECORDING,
        (
            duration_secs,
            peak_rms,
            gates.min_duration_secs,
            gates.silence_threshold,
        ),
    )
    .await?;
    serde_json::from_str(&json)
        .map_err(|e| VoiceCallError::Failed(format!("could not decode verdict: {e}")))
}

/// Whether an STT transcript looks like a hallucination rather than speech.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
///
/// **A caller that cannot reach the module must not guess.** Treating an error
/// as "hallucinated" silently deletes real speech; treating it as "clean" lets
/// `[BLANK_AUDIO]` reach the agent as an instruction. Of the two, passing the
/// text through is recoverable and losing it is not, so callers here fall open
/// — and say so at the call site rather than burying it in a default.
pub async fn is_hallucinated(
    config: &Config,
    text: &str,
    mode: HallucinationMode,
) -> Result<bool, VoiceCallError> {
    call(
        config,
        methods::IS_HALLUCINATED,
        (text, hallucination_mode_wire(mode)),
    )
    .await
}

/// Downmix, resample to 16 kHz, optionally silence-gate, and frame as WAV.
///
/// This is the whole capture-side pipeline in one call. Three separate calls
/// would ship the same audio across the bus three times to do work that is
/// microseconds of arithmetic.
///
/// `samples` are interleaved `f32`; `gate_threshold` of zero disables the
/// silence gate.
///
/// # Errors
///
/// [`VoiceCallError`], including a `Failed` when `samples` is not a whole
/// number of frames for `channels`.
pub async fn prepare_capture(
    config: &Config,
    samples: &[f32],
    source_rate: u32,
    channels: u16,
    gate_threshold: f32,
) -> Result<Vec<u8>, VoiceCallError> {
    let encoded = encode_samples(samples);
    let wav: String = call(
        config,
        methods::PREPARE_CAPTURE,
        (encoded, source_rate, channels, gate_threshold),
    )
    .await?;
    decode_audio(&wav)
}

/// A hotkey-driven recording session held by the module.
///
/// The module owns the hotkey -> record -> finish state machine (phase,
/// generation guard, setup / stop-intent buffering). The host reports facts as
/// [`SessionInput`]s and performs the one [`SessionAction`] each answer names.
///
/// Not `Drop`-based: call
/// [`close`](Self::close) when the server stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecorderSession {
    id: u64,
}

impl RecorderSession {
    /// A session handle for an id no module issued, so host tests can drive
    /// code that must survive the module being absent.
    #[cfg(test)]
    pub(crate) const fn unopened_for_test() -> Self {
        Self { id: u64::MAX }
    }

    /// Open a stopped session.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] when the module is unavailable or at its session cap.
    pub async fn open(config: &Config) -> Result<Self, VoiceCallError> {
        let id: u64 = call(config, methods::SESSION_OPEN, ()).await?;
        Ok(Self { id })
    }

    /// Apply `input` and return the module's verdict and the action to take.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] when the module is unavailable or the session is not
    /// open (it was closed, or the process restarted the module).
    pub async fn advance(
        &self,
        config: &Config,
        input: SessionInput,
    ) -> Result<SessionAdvance, VoiceCallError> {
        let json = serde_json::to_string(&input)
            .map_err(|e| VoiceCallError::Failed(format!("could not encode session input: {e}")))?;
        let reply: String = call(config, methods::SESSION_ADVANCE, (self.id, json)).await?;
        serde_json::from_str(&reply)
            .map_err(|e| VoiceCallError::Failed(format!("could not decode session advance: {e}")))
    }

    /// The session's phase and latest generation.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] when the module is unavailable or the session is not open.
    pub async fn status(&self, config: &Config) -> Result<SessionStatus, VoiceCallError> {
        let reply: String = call(config, methods::SESSION_STATUS, (self.id,)).await?;
        serde_json::from_str(&reply)
            .map_err(|e| VoiceCallError::Failed(format!("could not decode session status: {e}")))
    }

    /// Release the session. Closing one that is already gone is not an error.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] only when the module itself is unreachable.
    pub async fn close(&self, config: &Config) -> Result<(), VoiceCallError> {
        call(config, methods::SESSION_CLOSE, (self.id,)).await
    }
}

/// Frame 16-bit PCM samples as a WAV file, without touching the samples.
///
/// A caller holding `i16` should not have
/// to widen to `f32` and let the module narrow back: that round trip is lossy
/// by one LSB for no reason. This path is exact.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
pub async fn encode_wav_pcm16(
    config: &Config,
    samples: &[i16],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, VoiceCallError> {
    use base64::Engine as _;
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let wav: String = call(
        config,
        methods::ENCODE_WAV_PCM16,
        (encoded, sample_rate, channels),
    )
    .await?;
    decode_audio(&wav)
}

/// Load the voice module if it is not already serving.
///
/// Callers do not have to invoke this — every operation above does it — but a
/// caller that wraps its work in a deadline should, *outside* that deadline. A
/// first use may download and verify an artifact, and charging that against a
/// dictation timeout means the first utterance a user ever speaks is the one
/// that fails.
///
/// # Errors
///
/// The same [`VoiceCallError::Unavailable`] the operations return.
pub async fn ensure_ready(config: &Config) -> Result<(), VoiceCallError> {
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(VoiceCallError::Unavailable)
}

/// Reserve an opaque handle before native microphone startup.
pub async fn reserve_capture(
    config: &Config,
    permission: MicrophonePermission,
) -> Result<CaptureResult<CaptureHandle>, VoiceCallError> {
    call(
        config,
        methods::RESERVE_CAPTURE,
        (RecordingStartRequest {
            permission,
            handle: None,
        },),
    )
    .await
}

/// Reserve a native or host-fed hotkey listener before startup.
pub async fn reserve_hotkey(
    config: &Config,
    request: HotkeyRequest,
) -> Result<Result<HotkeyHandle, HotkeyError>, VoiceCallError> {
    call(
        config,
        methods::HOTKEY_RESERVE,
        (HotkeyReserveRequest { request },),
    )
    .await
}

/// Start a reserved listener. Repeated starts recover a lost reply.
pub async fn start_hotkey(
    config: &Config,
    handle: HotkeyHandle,
) -> Result<Result<HotkeyReply, HotkeyError>, VoiceCallError> {
    call(
        config,
        methods::HOTKEY_START,
        (HotkeyStartRequest { handle },),
    )
    .await
}

/// Read an acknowledged/replayable batch of activation events.
pub async fn read_hotkey(
    config: &Config,
    request: HotkeyReadRequest,
) -> Result<Result<HotkeyBatch, HotkeyError>, VoiceCallError> {
    call(config, methods::HOTKEY_READ, (request,)).await
}

/// Feed a bounded batch of host-owned generic key facts.
pub async fn feed_hotkey(
    config: &Config,
    request: HotkeyFeedRequest,
) -> Result<Result<HotkeyReply, HotkeyError>, VoiceCallError> {
    call(config, methods::HOTKEY_FEED, (request,)).await
}

/// Stop a listener after its native hook has been removed and joined.
pub async fn stop_hotkey(
    config: &Config,
    handle: HotkeyHandle,
) -> Result<Result<HotkeyReply, HotkeyError>, VoiceCallError> {
    call(
        config,
        methods::HOTKEY_STOP,
        (HotkeyHandleRequest { handle },),
    )
    .await
}

/// Shut down every listener and wait for native cleanup.
pub async fn shutdown_hotkeys(
    config: &Config,
) -> Result<Result<HotkeyReply, HotkeyError>, VoiceCallError> {
    call(config, methods::HOTKEY_SHUTDOWN, ()).await
}

/// Base64 little-endian `f32`, which is how the interface carries samples.
fn encode_samples(samples: &[f32]) -> String {
    use base64::Engine as _;
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Decode base64 little-endian `f32` samples the module produced.
fn decode_samples(encoded: &str) -> Result<Vec<f32>, VoiceCallError> {
    let bytes = decode_audio(encoded)?;
    if !bytes.len().is_multiple_of(4) {
        return Err(VoiceCallError::Failed(format!(
            "module returned {} bytes, not a whole number of f32 samples",
            bytes.len()
        )));
    }
    let (samples, _) = bytes.as_chunks::<4>();
    Ok(samples.iter().copied().map(f32::from_le_bytes).collect())
}

/// Decode a base64 audio payload the module produced.
fn decode_audio(encoded: &str) -> Result<Vec<u8>, VoiceCallError> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| VoiceCallError::Failed(format!("module returned invalid base64: {e}")))
}

/// Ensure the module is serving, then make one call on it.
async fn call<A, R>(config: &Config, method: &str, args: A) -> Result<R, VoiceCallError>
where
    A: serde::Serialize + Send,
    R: serde::de::DeserializeOwned,
{
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(VoiceCallError::Unavailable)?;
    let record = registry::find(MODULE_ID)
        .ok_or_else(|| VoiceCallError::Unavailable(format!("unknown module '{MODULE_ID}'")))?;
    let runtime = super::runtime::runtime()
        .await
        .map_err(|_| VoiceCallError::Unavailable("the module bus is not running".to_string()))?;
    let proxy = runtime
        .proxy(record.bus_name, record.object_path)
        .map_err(|error| VoiceCallError::Failed(error.to_string()))?;

    proxy
        .call(method, args)
        .await
        .map_err(|error| classify(&error))
}

/// Map a wire error onto the two outcomes a caller distinguishes.
fn classify(error: &tinybus::Error) -> VoiceCallError {
    let message = error.to_string();
    match error.wire_name() {
        // Loaded but not answering: refused, faulted, or gone.
        name if name.contains("ModuleUnavailable") => VoiceCallError::Unavailable(message),
        _ => VoiceCallError::Failed(message),
    }
}

#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
