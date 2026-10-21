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
fn is_unknown_provider_user_config(err: &str) -> bool {
    err.contains("no cloud provider with id or slug")
}

#[derive(Debug, Clone, serde::Serialize)]
pub async fn inference_get_client_config(ctx: &DispatchCtx) -> Result<Outcome<Value>, String> {
    debug!("{LOG_PREFIX} get_client_config:start");
    let result = config_rpc::load_and_get_client_config_snapshot(ctx).await;
    match &result {
        Ok(_) => debug!("{LOG_PREFIX} get_client_config:ok"),
        Err(err) => warn!(error = %err, "{LOG_PREFIX} get_client_config:error"),
    }
    result
}

pub async fn inference_update_model_settings(
    ctx: &DispatchCtx,
    update: config_rpc::ModelSettingsPatch,
) -> Result<Outcome<Value>, String> {
    debug!("{LOG_PREFIX} update_model_settings:start");
    let result = config_rpc::load_and_apply_model_settings(ctx, update).await;
    match &result {
        Ok(_) => debug!("{LOG_PREFIX} update_model_settings:ok"),
        Err(err) => warn!(error = %err, "{LOG_PREFIX} update_model_settings:error"),
    }
    result
}

pub async fn inference_update_local_settings(
    ctx: &DispatchCtx,
    update: config_rpc::LocalAiSettingsPatch,
) -> Result<Outcome<Value>, String> {
    debug!("{LOG_PREFIX} update_local_settings:start");
    let result = config_rpc::load_and_apply_local_ai_settings(ctx, update).await;
    match &result {
        Ok(_) => {
            debug!("{LOG_PREFIX} update_local_settings:ok");
            // The endpoint, provider or models may have changed: drop the
            // cached probe verdict so the next status poll re-probes the
            // user's runtime instead of reporting the old endpoint's state.
            match ctx.load_config().await {
                Ok(config) => {
                    let runtime = crate::inference::local_runtime_config(&config);
                    local_runtime::global(&config).reset_to_idle(&runtime);
                    debug!("{LOG_PREFIX} update_local_settings:probe_reset");
                }
                Err(err) => warn!(
                    error = %err,
                    "{LOG_PREFIX} update_local_settings:probe_reset_skipped (config reload failed)"
                ),
            }
        }
        Err(err) => warn!(error = %err, "{LOG_PREFIX} update_local_settings:error"),
    }
    result
}

pub async fn inference_list_models(provider_id: &str) -> Result<Outcome<Value>, String> {
    debug!(provider_id, "{LOG_PREFIX} list_models:start");
    let result = providers::models::catalog_listing::list_configured_models(provider_id).await;
    match &result {
        Ok(_) => debug!("{LOG_PREFIX} list_models:ok"),
        Err(err) => {
            if is_unknown_provider_user_config(err) {
                // User selected a provider id that isn't a registered
                // cloud provider (e.g. picking "ollama", a local runtime).
                // Demote to `warn!` so it stays in local logs but doesn't
                // escalate to Sentry. Targets TAURI-RUST-X (~5740 events).
                warn!(
                    provider_id,
                    error = %err,
                    "{LOG_PREFIX} list_models:unknown-provider (user-config)"
                );
            } else if let Some(kind) = crate::core::observability::expected_error_kind(err) {
                // Classify at the TYPED SOURCE — run the raw provider error
                // through the central classifier BEFORE the
                // `[inference::ops] list_models:error: …` prefix is applied,
                // then `warn!` so the Sentry tracing layer records at most a
                // breadcrumb instead of a hard error event.
                //
                // TAURI-RUST-8X3: a user pointed a custom OpenAI-compatible
                // provider at a base URL with no `/models` route, so the
                // probe returns `provider returned 404: 404 page not found`.
                // That is a preventable user-state condition (wrong base URL;
                // the dropdown already surfaces an actionable hint inline) —
                // not a code bug. The 404 arm of
                // `is_provider_user_state_message` matched the *raw* error
                // string fine, but the previous `error!` path captured the
                // PREFIXED log line, so the demotion never reached Sentry's
                // classifier. Classifying the raw `err` here removes that
                // dependency on the log-string shape entirely; any
                // `ExpectedErrorKind` the central classifier recognizes is
                // demoted at the source.
                warn!(
                    provider_id,
                    error = %err,
                    expected_kind = ?kind,
                    "{LOG_PREFIX} list_models:expected (user-config): {err}"
                );
            } else {
                // Real error — embed `{err}` in the format string so
                // Sentry's event title carries the actionable cause
                // instead of the opaque `list_models:error` shape that
                // made TAURI-RUST-X untriageable.
                error!(
                    provider_id,
                    error = %err,
                    "{LOG_PREFIX} list_models:error: {err}"
                );
            }
        }
    }
    result
}

/// Snapshot of BYO provider auth failures (invalid / revoked key, 401 / 403)
/// recorded this process. Backs the AI-settings provider-error notice so a
/// key that breaks at runtime — most often in a silent background loop like
/// memory summarization (TAURI-RUST-4RC) — is surfaced inline next to the key
/// editor, not only in the notification center. Cleared when the user updates
/// or removes the offending key.
