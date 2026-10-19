use super::handlers::*;
use super::params::*;
use crate::core::all::ControllerParamContract;
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};

fn decode<T: DeserializeOwned>(params: Map<String, Value>) -> Result<(), String> {
    super::helpers::deserialize_params::<T>(params).map(drop)
}

fn ignored(_params: Map<String, Value>) -> Result<(), String> {
    Ok(())
}

/// Explicit parameter fixtures; decoder types match the named handlers.
/// Domain-specific defaults, values and errors remain in schemas_tests.rs.
pub(crate) fn contracts() -> Vec<ControllerParamContract> {
    let mut contracts = vec![
        ControllerParamContract {
            method: "voice.status", required: &[], handler: handle_voice_status,
            examples: vec![json!({})], parse: ignored,
        },
        ControllerParamContract {
            method: "voice.transcribe", required: &["audio_path"], handler: handle_voice_transcribe,
            examples: vec![json!({"audio_path":"fixture.wav"}), json!({"audio_path":"fixture.wav","context":"example","skip_cleanup":true})],
            parse: decode::<TranscribeParams>,
        },
        ControllerParamContract {
            method: "voice.transcribe_bytes", required: &["audio_bytes"], handler: handle_voice_transcribe_bytes,
            examples: vec![json!({"audio_bytes":[0,1]}), json!({"audio_bytes":[0,1],"extension":"wav","context":"example","skip_cleanup":true})],
            parse: decode::<TranscribeBytesParams>,
        },
        ControllerParamContract {
            method: "voice.tts", required: &["text"], handler: handle_voice_tts,
            examples: vec![json!({"text":"Hello"}), json!({"text":"Hello","output_path":"fixture.wav"})],
            parse: decode::<TtsParams>,
        },
        ControllerParamContract {
            method: "voice.reply_synthesize", required: &["text"], handler: handle_voice_reply_synthesize,
            examples: vec![json!({"text":"Hello"}), json!({"text":"Hello","voice_id":"fixture","model_id":"fixture","output_format":"mp3_44100"})],
            parse: decode::<ReplySynthesizeParams>,
        },
        ControllerParamContract {
            method: "voice.stt_dispatch", required: &["audio_base64"], handler: handle_voice_stt_dispatch,
            examples: vec![json!({"audio_base64":"AA=="}), json!({"audio_base64":"AA==","provider":"cloud","model":"fixture","mime_type":"audio/wav","file_name":"fixture.wav","language":"en"})],
            parse: decode::<SttDispatchParams>,
        },
        ControllerParamContract {
            method: "voice.tts_dispatch", required: &["text"], handler: handle_voice_tts_dispatch,
            examples: vec![json!({"text":"Hello"}), json!({"text":"Hello","provider":"cloud","voice":"fixture"})],
            parse: decode::<TtsDispatchParams>,
        },
        ControllerParamContract {
            method: "voice.set_providers", required: &[], handler: handle_voice_set_providers,
            examples: vec![json!({}), json!({"stt_provider":"cloud","tts_provider":"cloud","stt_model":"fixture","tts_voice":"fixture"})],
            parse: decode::<SetProvidersParams>,
        },
        ControllerParamContract {
            method: "voice.update_provider_settings", required: &[], handler: handle_voice_update_provider_settings,
            examples: vec![json!({}), json!({"voice_providers":[{"slug":"fixture-provider"}],"stt_provider":"cloud","tts_provider":"cloud"})],
            parse: decode::<VoiceUpdateProviderSettingsParams>,
        },
        ControllerParamContract {
            method: "voice.list_models", required: &["provider_id"], handler: handle_voice_list_models,
            examples: vec![json!({"provider_id":"fixture"}), json!({"provider_id":"fixture","capability":"stt"})],
            parse: decode::<VoiceListModelsParams>,
        },
        ControllerParamContract {
            method: "voice.test_provider", required: &["workload","provider"], handler: handle_voice_test_provider,
            examples: vec![json!({"workload":"tts","provider":"cloud"}), json!({"workload":"stt","provider":"cloud","validate_only":true,"api_key":"fixture-key"})],
            parse: decode::<VoiceTestProviderParams>,
        },
    ];
    contracts.extend(crate::voice::live::schemas::parameter_contracts());
    contracts
}
