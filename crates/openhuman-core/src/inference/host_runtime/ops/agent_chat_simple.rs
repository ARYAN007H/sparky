//! The lightweight provider-only agent chat variant.

use crate::agent_runs::agent_chat::guards::{
    enforce_user_prompt_or_reject, normalize_model_override,
};
use crate::config::Config;
use crate::core::Outcome;
use crate::inference::provider as providers;

/// A simplified chat interface that does not update the base configuration.
pub async fn agent_chat_simple(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    message: &str,
    model_override: Option<String>,
    temperature: Option<f64>,
    thread_id: Option<String>,
) -> Result<Outcome<String>, String> {
    enforce_user_prompt_or_reject(message, "local_ai.ops.agent_chat_simple")?;

    let mut effective = config.clone();
    if let Some(model) = normalize_model_override(model_override) {
        effective.default_model = Some(model);
    }
    if let Some(temp) = temperature {
        effective.default_temperature = temp;
    }

    let (model, resolved_model): (
        std::sync::Arc<dyn tinyinference_llm::model::ChatModel<()>>,
        String,
    ) = if providers::factory::resolves_to_managed_backend("chat", &effective) {
        let resolved_context = ctx.resolved();
        let extensions = resolved_context
            .as_deref()
            .map(crate::core::runtime::CoreContext::extensions);
        let (backend, resolved_model) =
            providers::factory::make_openhuman_backend_model_for_thread_in(
                extensions,
                "chat",
                &effective,
                effective.default_model.as_deref(),
                true,
                thread_id.as_deref(),
                &crate::security::egress::EgressLedger::none(),
            )
            .map_err(|e| e.to_string())?;
        (backend, resolved_model)
    } else {
        providers::create_chat_model_with_model_id_in(
            ctx,
            "chat",
            &effective,
            effective.default_temperature,
        )
        .map_err(|e| e.to_string())?
    };
    tracing::debug!(
        requested_model = ?effective.default_model,
        resolved_model = %resolved_model,
        temperature = effective.default_temperature,
        "[inference] agent_chat_simple invoking crate-native chat model"
    );
    let run = model.invoke(
        &(),
        tinyinference_llm::model::ModelRequest::new(vec![
            tinyinference_llm::message::Message::user(message.to_string()),
        ])
        .with_model(resolved_model.clone())
        .with_temperature(effective.default_temperature),
    );
    let response = run.await.map_err(|e| e.to_string())?.text();

    Ok(Outcome::single_log(response, "agent simple chat completed"))
}
