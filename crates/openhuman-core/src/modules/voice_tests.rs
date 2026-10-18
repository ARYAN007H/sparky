//! Tests for the voice call client.
//!
//! Nothing here loads a module. What is testable without one is the part
//! that decides what a caller does next: the intent wire contract, the
//! mode spellings, and that the unavailable path is reached without a
//! broker. The round trips themselves are covered where they can be
//! honest — `tinyvoice`'s own loader E2E, which drives a real module over
//! a real broker against the published artifact.

use super::{encode_samples, hallucination_mode_wire, HallucinationMode, VoiceCallError};
use crate::config::Config;
#[test]
fn hallucination_modes_use_the_wire_spelling() {
    // The module rejects an unknown mode rather than defaulting, so a typo
    // here is a hard failure at runtime rather than a silent mode swap.
    assert_eq!(
        hallucination_mode_wire(HallucinationMode::Dictation),
        "dictation"
    );
    assert_eq!(
        hallucination_mode_wire(HallucinationMode::Conversation),
        "conversation"
    );
}

#[test]
fn samples_encode_as_little_endian_f32() {
    use base64::Engine as _;
    let encoded = encode_samples(&[1.0f32, -1.0f32]);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .expect("valid base64");
    assert_eq!(bytes.len(), 8);
    assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
    assert_eq!(&bytes[4..8], &(-1.0f32).to_le_bytes());
}

#[test]
fn errors_render_as_their_message() {
    assert_eq!(
        VoiceCallError::Unavailable("downloads are off".to_string()).to_string(),
        "downloads are off"
    );
    assert_eq!(
        VoiceCallError::Failed("bad payload".to_string()).to_string(),
        "bad payload"
    );
}

/// A config with modules enabled but nothing fetchable.
fn offline_config() -> Config {
    let mut config = Config::default();
    config.modules.enabled = true;
    config.modules.allow_download = false;
    config
}

#[tokio::test]
async fn a_disabled_host_reports_unavailable_without_starting_a_broker() {
    // Every entry point has to reach the unavailable path on its own: they
    // each call `ensure_loaded` separately, so one of them forgetting to
    // would only show up as a hang or a panic in the field.
    let mut config = offline_config();
    config.modules.enabled = false;

    assert!(matches!(
        super::route(&config, "pause").await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::sequence_utterance(&config, "hey tiny pause", "Hey Tiny").await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::gate_recording(
            &config,
            1.0,
            0.5,
            super::RecordingGates {
                min_duration_secs: 0.3,
                silence_threshold: 0.01
            }
        )
        .await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::is_hallucinated(&config, "okay", HallucinationMode::Dictation).await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::encode_wav(&config, &[0.1, 0.2], 16_000).await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::prepare_capture(&config, &[0.1, 0.2], 32_000, 2, 0.0).await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::ensure_ready(&config).await,
        Err(VoiceCallError::Unavailable(_))
    ));
}

#[test]
fn the_registry_entry_matches_the_interface_this_client_calls() {
    // The registry is a plain `const` table and cannot name a gated crate, so
    // the bus name and object path are still written out there by hand. This
    // is what checks them against the contract's own constants — a mismatch is
    // not a compile error, it is a `NameHasNoOwner` at first use, in the field,
    // on whichever platform nobody tested.
    let record = crate::modules::registry::find("tinyvoice").expect("tinyvoice is registered");
    assert_eq!(record.bus_name, tinyvoice_bus::names::BUS_NAME);
    assert_eq!(record.object_path, tinyvoice_bus::names::OBJECT_PATH);
    assert!(
        record.object_path.starts_with('/') && !record.object_path.contains('.'),
        "an object path with a dot in it is rejected by the loader, not by the compiler"
    );
}

#[tokio::test]
async fn every_entry_point_degrades_rather_than_hanging_when_the_module_is_gone() {
    // The fallback directions are asymmetric on purpose, and the asymmetry is
    // the point: whichever way a caller falls, it must first get a plain error
    // back rather than a panic or a stall. `ops.rs` falls OPEN on this error
    // (failing closed would silently delete real speech) and `always_on`'s
    // wake-word gate falls CLOSED (failing open would hand an unaddressed
    // utterance to the agent). Both depend on seeing `Unavailable`.
    let mut config = offline_config();
    config.modules.enabled = false;

    assert!(matches!(
        super::is_hallucinated(
            &config,
            "thank you for watching",
            HallucinationMode::Conversation
        )
        .await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::sequence_utterance(&config, "hey tiny pause", "Hey Tiny").await,
        Err(VoiceCallError::Unavailable(_))
    ));

    // The session surface has to reach the same verdict, because always_on now
    // retries `open` on a cooldown instead of tearing the loop down: a variant
    // other than `Unavailable` here would change that path's behaviour.
    assert!(matches!(
        super::VadSession::open(
            &config,
            super::vad_config_from_server_config(&crate::config::VoiceServerConfig::default())
        )
        .await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::prepare_frames(&config, &[0.1, 0.2], 32_000, 2).await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::frame_energies(&config, &[0.1, 0.2], 1).await,
        Err(VoiceCallError::Unavailable(_))
    ));
    assert!(matches!(
        super::encode_wav_pcm16(&config, &[1i16, 2], 16_000, 1).await,
        Err(VoiceCallError::Unavailable(_))
    ));
}

#[test]
fn every_member_this_client_calls_is_one_the_contract_declares() {
    // Host adapters use contract constants for call sites. Keep the curated
    // surface, including capture leases and hotkeys, aligned with the module.
    use tinyvoice_bus::names::methods;
    let called = [
        methods::ROUTE,
        methods::EXTRACT_COMMAND,
        methods::WAKE_WORD_PRESENT,
        methods::IS_HALLUCINATED,
        methods::VAD_OPEN,
        methods::VAD_PUSH,
        methods::VAD_IS_SPEAKING,
        methods::VAD_RESET,
        methods::VAD_CLOSE,
        methods::PREPARE_FRAMES,
        methods::FRAME_ENERGIES,
        methods::ENCODE_WAV,
        methods::ENCODE_WAV_PCM16,
        methods::PREPARE_CAPTURE,
        methods::CAPTURE_POLL,
        methods::CAPTURE_SHUTDOWN,
        methods::CAPTURE_START,
        methods::CAPTURE_STOP,
        methods::RESERVE_CAPTURE,
        methods::LIST_INPUT_DEVICES,
        methods::RECORDING_START,
        methods::RECORDING_FINISH,
        methods::RECORDING_SUMMARY,
        methods::RECORDING_CANCEL,
        methods::READ_AUDIO_OUTPUT,
        methods::RELEASE_AUDIO_OUTPUT,
        methods::HOTKEY_RESERVE,
        methods::HOTKEY_START,
        methods::HOTKEY_READ,
        methods::HOTKEY_FEED,
        methods::HOTKEY_STOP,
        methods::HOTKEY_SHUTDOWN,
    ];
    for member in called {
        assert!(
            tinyvoice_bus::names::METHODS.contains(&member),
            "the host calls `{member}`, which the contract does not dispatch"
        );
    }
}

/// The session input and action tags are a wire contract with the module: a
/// rename on either side makes a hotkey press a no-op, which looks like a dead
/// key rather than an error, so the spellings the host sends and expects are
/// pinned.
#[test]
fn recording_session_wire_spellings_match_the_contract() {
    use super::{SessionAction, SessionAdvance, SessionInput, SessionPhase};
    assert_eq!(
        serde_json::to_string(&SessionInput::SetupReady {
            stop_buffered: false
        })
        .unwrap(),
        r#"{"input":"setup_ready","stop_buffered":false}"#
    );
    assert_eq!(
        serde_json::to_string(&SessionInput::Finished { generation: 4 }).unwrap(),
        r#"{"input":"finished","generation":4}"#
    );
    let reply: SessionAdvance = serde_json::from_str(
        r#"{"accepted":true,"phase":"recording","generation":2,"action":{"action":"defer_stop","generation":2,"after_ms":1500}}"#,
    )
    .unwrap();
    assert_eq!(reply.phase, SessionPhase::Recording);
    assert_eq!(
        reply.action,
        SessionAction::DeferStop {
            generation: 2,
            after_ms: 1500
        }
    );
}
