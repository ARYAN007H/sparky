//! JSON-RPC controller surface for inference operations.

use crate::config::rpc as config_rpc;
use crate::config::Config;
use crate::core::runtime::DispatchCtx;
use crate::core::Outcome;
use crate::inference::host_runtime as local_runtime;
use crate::inference::provider as providers;
use serde_json::{json, Value};
use tinyinference_llm::message::Message;
use tinyinference_llm::model::ModelRequest;
use tinyinference_llm::sentiment::{parse_sentiment_response, SentimentResult};
use tinyinference_local::status::LocalAiStatus;
use tracing::{debug, error, warn};

const LOG_PREFIX: &str = "[inference::ops]";

/// User picked a provider id (slug) that isn't registered in the cloud
/// provider list — e.g. selecting `"ollama"` as a cloud provider when it's
/// actually a local runtime. Matches the literal phrase emitted at
/// `crates/openhuman-core/src/inference/provider/models/catalog_listing.rs:54`
/// (`"no cloud provider with id or slug '{}' found"`).
///
/// Used by [`inference_list_models`] to demote this user-config case to
/// `warn!` so it stops escalating to Sentry (TAURI-RUST-X, ~5740 events).
/// The matcher is anchored on the exact phrase so unrelated sibling
/// failures (TAURI-RUST-12 JSON parse, TAURI-RUST-2W reqwest builder,
/// TAURI-RUST-JP local ollama_admin transport) still surface as real
/// errors.
fn is_unknown_provider_user_config(err: &str) -> bool {
    err.contains("no cloud provider with id or slug")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InferenceTestChatModelResult {
    pub reply: String,
}

fn expected_test_provider_model_error_kind(
    err: &str,
) -> Option<crate::core::observability::ExpectedErrorKind> {
    crate::core::observability::expected_error_kind(err)
}

pub async fn inference_status(config: &Config) -> Result<Outcome<LocalAiStatus>, String> {
    debug!("{LOG_PREFIX} status:start");
    let result = local_runtime::ops::local_ai_status(config).await;
    match &result {
        Ok(outcome) => debug!(state = %outcome.value.state, "{LOG_PREFIX} status:ok"),
        Err(err) => warn!(error = %err, "{LOG_PREFIX} status:error"),
    }
    result
}

pub async fn inference_summarize(
    config: &Config,
    text: &str,
    max_tokens: Option<u32>,
) -> Result<Outcome<String>, String> {
    debug!(
        text_len = text.len(),
        ?max_tokens,
        "{LOG_PREFIX} summarize:start"
    );
    let result = local_runtime::ops::local_ai_summarize(config, text, max_tokens).await;
    match &result {
        Ok(outcome) => debug!(
            output_len = outcome.value.len(),
            "{LOG_PREFIX} summarize:ok"
        ),
        Err(err) => warn!(error = %err, "{LOG_PREFIX} summarize:error"),
    }
    result
}

pub async fn inference_prompt(
    config: &Config,
    prompt: &str,
    max_tokens: Option<u32>,
    no_think: Option<bool>,
) -> Result<Outcome<String>, String> {
    debug!(
        prompt_len = prompt.len(),
        ?max_tokens,
        ?no_think,
        "{LOG_PREFIX} prompt:start"
    );
    let result = local_runtime::ops::local_ai_prompt(config, prompt, max_tokens, no_think).await;
    match &result {
        Ok(outcome) => debug!(output_len = outcome.value.len(), "{LOG_PREFIX} prompt:ok"),
        Err(err) => warn!(error = %err, "{LOG_PREFIX} prompt:error"),
    }
    result
}

pub async fn inference_vision_prompt(
    config: &Config,
    prompt: &str,
    image_refs: &[String],
    max_tokens: Option<u32>,
) -> Result<Outcome<String>, String> {
    debug!(
        prompt_len = prompt.len(),
        image_count = image_refs.len(),
        ?max_tokens,
        "{LOG_PREFIX} vision_prompt:start"
    );
    let result =
        local_runtime::ops::local_ai_vision_prompt(config, prompt, image_refs, max_tokens).await;
    match &result {
        Ok(outcome) => debug!(
            output_len = outcome.value.len(),
            "{LOG_PREFIX} vision_prompt:ok"
        ),
        Err(err) => warn!(error = %err, "{LOG_PREFIX} vision_prompt:error"),
    }
    result
}

pub async fn inference_test_provider_model(
    config: &Config,
    workload: &str,
    provider: &str,
    prompt: &str,
) -> Result<Outcome<InferenceTestChatModelResult>, String> {
    debug!(
        workload,
        provider,
        prompt_len = prompt.len(),
        "{LOG_PREFIX} test_provider_model:start"
    );
    let local = tinyinference_local::profile::is_local_provider_string(provider);
    let (chat_model, model) = if local {
        debug!(
            provider,
            "{LOG_PREFIX} test_provider_model:build_local_model"
        );
        crate::inference::provider::factory::create_local_chat_model_from_string(provider, config)
    } else {
        debug!(provider, "{LOG_PREFIX} test_provider_model:build_model");
        crate::inference::provider::create_chat_model_from_string_with_model_id(
            workload,
            provider,
            config,
            config.default_temperature,
        )
    }
    .map_err(|e| e.to_string())?;
    debug!(%model, local, "{LOG_PREFIX} test_provider_model:invoke");
    let result = chat_model
        .invoke(
            &(),
            ModelRequest::new(vec![Message::user(prompt)])
                .with_model(model)
                .with_temperature(config.default_temperature),
        )
        .await
        .map_err(|e| e.to_string())
        .map(|response| {
            Outcome::single_log(
                InferenceTestChatModelResult {
                    reply: response.text(),
                },
                "provider model test completed",
            )
        });
    match &result {
        Ok(outcome) => debug!(
            output_len = outcome.value.reply.len(),
            "{LOG_PREFIX} test_provider_model:ok"
        ),
        Err(err) => {
            if let Some(kind) = expected_test_provider_model_error_kind(err) {
                warn!(
                    workload,
                    provider,
                    expected_error_kind = ?kind,
                    error = %err,
                    "{LOG_PREFIX} test_provider_model:expected_error"
                );
            } else {
                error!(
                    workload,
                    provider,
                    error = %err,
                    "{LOG_PREFIX} test_provider_model:error"
                );
            }
        }
    }
    result
}

pub async fn inference_diagnostics(config: &Config) -> Result<Outcome<Value>, String> {
    debug!("{LOG_PREFIX} diagnostics:start");
    let service = local_runtime::global(config);
    // Return the diagnostics payload directly (no `{result, logs}` wrap) so
    // callers (UI + json_rpc_e2e tests) can read `provider`, `lm_studio_running`,
    // etc. straight off the response — mirrors the legacy
    // `local_ai_diagnostics` shape that the test asserts against.
    let runtime = crate::inference::local_runtime_config(config);
    let result = service
        .diagnostics(&runtime)
        .await
        .map(|value| Outcome::new(value, Vec::new()));
    match &result {
        Ok(_) => debug!("{LOG_PREFIX} diagnostics:ok"),
        Err(err) => warn!(error = %err, "{LOG_PREFIX} diagnostics:error"),
    }
    result
}

#[cfg(test)]
#[path = "ops_tests.rs"]
