//! Handlers for voice dictation and voice-server settings.

use crate::core::runtime::DispatchCtx;
use serde_json::{Map, Value};

use crate::config::rpc as config_rpc;
use crate::core::all::ControllerFuture;

use super::super::helpers::{
    deserialize_params, to_json, DictationSettingsUpdate, VoiceServerSettingsUpdate,
};

pub(super) fn handle_get_dictation_settings(
    ctx: DispatchCtx,
    _params: Map<String, Value>,
) -> ControllerFuture {
    Box::pin(async move { to_json(config_rpc::get_dictation_settings(&ctx).await?) })
}

pub(super) fn handle_update_dictation_settings(
    ctx: DispatchCtx,
    params: Map<String, Value>,
) -> ControllerFuture {
    Box::pin(async move {
        let update = deserialize_params::<DictationSettingsUpdate>(params)?;
        let patch = config_rpc::DictationSettingsPatch {
            enabled: update.enabled,
            hotkey: update.hotkey,
            activation_mode: update.activation_mode,
            llm_refinement: update.llm_refinement,
            streaming: update.streaming,
            streaming_interval_ms: update.streaming_interval_ms,
        };
        to_json(config_rpc::load_and_apply_dictation_settings(&ctx, patch).await?)
    })
}

pub(super) fn handle_get_voice_server_settings(
    ctx: DispatchCtx,
    _params: Map<String, Value>,
) -> ControllerFuture {
    Box::pin(async move { to_json(config_rpc::get_voice_server_settings(&ctx).await?) })
}

pub(super) fn handle_update_voice_server_settings(
    ctx: DispatchCtx,
    params: Map<String, Value>,
) -> ControllerFuture {
    Box::pin(async move {
        let update = deserialize_params::<VoiceServerSettingsUpdate>(params)?;
        let patch = config_rpc::VoiceServerSettingsPatch {
            auto_start: update.auto_start,
            hotkey: update.hotkey,
            activation_mode: update.activation_mode,
            skip_cleanup: update.skip_cleanup,
            min_duration_secs: update.min_duration_secs,
            silence_threshold: update.silence_threshold,
            custom_dictionary: update.custom_dictionary,
            stt_engine: update.stt_engine,
        };
        let result = config_rpc::load_and_apply_voice_server_settings(&ctx, patch).await?;
        to_json(result)
    })
}
