use super::*;

use crate::config::Config;
use crate::inference::provider::factory::PROVIDER_OPENHUMAN;
use crate::inference::registry::{ChatModelEntry, ChatModelRequest, ProviderModelHooks};
use std::sync::Arc;
use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};

struct SessionFixtureModel;

#[async_trait::async_trait]
impl ChatModel<()> for SessionFixtureModel {
    fn profile(&self) -> Option<&ModelProfile> {
        None
    }

    async fn invoke(
        &self,
        _state: &(),
        _request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        Ok(ModelResponse::assistant("session fixture"))
    }
}

/// Installs an explicit generic model fixture for tests that construct a
/// session through the managed-provider factory. HookSlot test registrations
/// are thread-local, so each test opts in on the thread that builds its host.
fn model_fixture_entry() -> ChatModelEntry {
    ChatModelEntry {
            build: Arc::new(|request: ChatModelRequest<'_>| {
                let model = request
                    .model_override
                    .or(request.default_model)
                    .or(request.config.default_model.as_deref())
                    .unwrap_or("test/session-fixture");
                Ok((Arc::new(SessionFixtureModel), model.to_string()))
            }),
            probe: None,
            models: Some(ProviderModelHooks {
                default_model: Arc::new(|| Some("test/session-fixture".into())),
                role_for_model: Arc::new(|model| {
                    if model == "fixture-reasoning" {
                        Some("reasoning".to_string())
                    } else {
                        model
                            .strip_prefix("hint:")
                            .filter(|role| crate::config::WORKLOAD_ROLES.contains(role))
                            .map(str::to_string)
                    }
                }),
                is_alias: Arc::new(|model| {
                    model == "fixture-reasoning"
                        || model
                        .strip_prefix("hint:")
                        .is_some_and(|role| crate::config::WORKLOAD_ROLES.contains(&role))
                }),
                is_raw_model: Arc::new(|model| {
                    !model.starts_with("hint:") && model != "fixture-reasoning"
                }),
                supports_vision: Arc::new(|_| false),
                resolve_model: Arc::new(|_, config, model| match model {
                    Some("fixture-reasoning") => Some("test/selected-reasoning".to_string()),
                    Some(model) if !model.starts_with("hint:") => Some(model.to_string()),
                    Some(model) => model
                        .strip_prefix("hint:")
                        .filter(|role| crate::config::WORKLOAD_ROLES.contains(role))
                        .map(|_| "test/session-fixture".to_string()),
                    None => config
                        .default_model
                        .clone()
                        .or_else(|| Some("test/session-fixture".into())),
                }),
                context_window: Arc::new(|_| None),
                default_pricing: Arc::new(|| None),
                without_reasoning: Arc::new(|request| request),
            }),
    }
}

pub(crate) fn install_model_fixture() {
    crate::inference::registry::register_chat_model(PROVIDER_OPENHUMAN, model_fixture_entry());
}

pub(crate) fn install_model_fixture_in(extensions: &crate::core::slots::Extensions) {
    crate::inference::registry::register_chat_model_in(
        extensions,
        PROVIDER_OPENHUMAN,
        model_fixture_entry(),
    );
}

pub(crate) fn model_fixture_context() -> Arc<crate::core::runtime::CoreContext> {
    let context = crate::core::runtime::CoreContext::for_test(
        crate::core::runtime::DomainSet::full(),
        None,
    );
    install_model_fixture_in(context.extensions());
    context
}

pub(crate) fn config_with_model_fixture() -> Config {
    install_model_fixture();
    let mut config = Config::default();
    config.default_model = Some("test/session-fixture".into());
    config
}
