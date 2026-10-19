use super::*;
use crate::core::all::ControllerParamContract;
use serde_json::json;

fn decode<T: serde::de::DeserializeOwned>(params: Map<String, Value>) -> Result<(), String> {
    parse_params::<T>(params).map(drop)
}

fn ignored(_params: Map<String, Value>) -> Result<(), String> {
    Ok(())
}

pub(crate) fn contracts() -> Vec<ControllerParamContract> {
    vec![
        ControllerParamContract {
            method: "voice.live_providers", required: &[], handler: handle_live_providers,
            examples: vec![json!({})], parse: ignored,
        },
        ControllerParamContract {
            method: "voice.live_settings_get", required: &[], handler: handle_live_settings_get,
            examples: vec![json!({})], parse: ignored,
        },
        ControllerParamContract {
            method: "voice.live_settings_set", required: &[], handler: handle_live_settings_set,
            examples: vec![json!({}),json!({"default_provider":"gemini","gemini":{"model":"fixture","voice":null},"sarvam":{"language":"en"},"elevenlabs":{"voice_id":"fixture"}})],
            parse: decode::<LiveSettingsPatch>,
        },
        ControllerParamContract {
            method: "voice.live_test_provider", required: &["provider"], handler: handle_live_test_provider,
            examples: vec![json!({"provider":"gemini"})], parse: decode::<TestParams>,
        },
    ]
}
