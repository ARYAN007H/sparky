//! The lightweight provider-only agent chat variant.

use crate::agent_runs::agent_chat::guards::{
    enforce_user_prompt_or_reject, normalize_model_override,
};
use crate::config::Config;
use crate::core::Outcome;
use crate::inference::provider as providers;
use tinyinference_llm::model::{
    ChatModel, ModelRequestDefaults, ModelResponse, ProfileOverrideModel,
};

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
    let response = simple_chat_with_model(
        model,
        resolved_model,
        effective.default_temperature,
        message,
    )
    .await?
    .text();

    Ok(Outcome::single_log(response, "agent simple chat completed"))
}

async fn simple_chat_with_model(
    model: std::sync::Arc<dyn ChatModel<()>>,
    resolved_model: String,
    temperature: f64,
    prompt: &str,
) -> Result<ModelResponse, String> {
    let model: std::sync::Arc<dyn ChatModel<()>> = std::sync::Arc::new(
        ProfileOverrideModel::new_preserving_profile(model).with_defaults(ModelRequestDefaults {
            model: Some(resolved_model),
            temperature: Some(temperature),
        }),
    );
    tinyagents_harness::oneshot::simple_chat(model, Vec::new(), prompt.to_string()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tinyinference_llm::message::Message;
    use tinyinference_llm::model::ModelRequest;

    struct RequestProbe {
        request: Mutex<Option<ModelRequest>>,
    }

    #[async_trait::async_trait]
    impl ChatModel<()> for RequestProbe {
        async fn invoke(
            &self,
            _state: &(),
            request: ModelRequest,
        ) -> tinyinference_llm::Result<ModelResponse> {
            *self.request.lock().expect("probe lock") = Some(request);
            Ok(ModelResponse::assistant("reply"))
        }
    }

    #[tokio::test]
    async fn simple_chat_uses_resolved_model_and_temperature_defaults() {
        let probe = Arc::new(RequestProbe {
            request: Mutex::new(None),
        });
        let model: Arc<dyn ChatModel<()>> = probe.clone();

        let response =
            simple_chat_with_model(model, "resolved/provider-model".to_string(), 0.37, "hello")
                .await
                .expect("simple chat response");

        assert_eq!(response.text(), "reply");
        let request = probe
            .request
            .lock()
            .expect("probe lock")
            .take()
            .expect("model invocation");
        assert_eq!(request.model.as_deref(), Some("resolved/provider-model"));
        assert_eq!(request.temperature, Some(0.37));
        assert_eq!(request.messages, vec![Message::user("hello")]);
    }
}
