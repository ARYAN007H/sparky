//! Test support: a fake managed chat model registered under the managed
//! provider id, standing in for the hosted provider the core has no copy of.
//!
//! Registration is process-global, so callers hold
//! [`inference_test_guard`](crate::inference::inference_test_guard) (or its
//! async twin) for as long as the [`FakeManaged`] guard lives.

use std::sync::Arc;

use async_trait::async_trait;
use tinyinference_llm::model::{ChatModel, ModelProfile, ModelRequest, ModelResponse};

use super::registry::{
    register_chat_model, unregister_chat_model, ChatModelEntry, ChatModelRequest,
};
use crate::inference::provider::factory::PROVIDER_OPENHUMAN;

/// A chat model whose profile mirrors the managed provider's (`managed`, with
/// tool capabilities following `native_tool_calling`).
pub(crate) struct FakeManagedModel {
    profile: ModelProfile,
    /// The thread the request that built this model carried.
    pub(crate) thread_id: Option<String>,
}

#[async_trait]
impl ChatModel<()> for FakeManagedModel {
    fn profile(&self) -> Option<&ModelProfile> {
        Some(&self.profile)
    }

    async fn invoke(
        &self,
        _state: &(),
        _request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        Ok(ModelResponse::assistant("from the fake managed model"))
    }
}

fn build(request: &ChatModelRequest<'_>) -> anyhow::Result<(Arc<dyn ChatModel<()>>, String)> {
    let native = request.native_tool_calling;
    let model = FakeManagedModel {
        profile: ModelProfile {
            provider: Some("managed".to_string()),
            tool_calling: native,
            parallel_tool_calls: native,
            streaming_tool_chunks: native,
            streaming: true,
            ..ModelProfile::default()
        },
        thread_id: request.thread_id.map(str::to_string),
    };
    let resolved = request
        .model_override
        .unwrap_or(crate::config::MODEL_MANAGED_DEFAULT)
        .to_string();
    Ok((Arc::new(model), resolved))
}

/// Registers the fake for as long as the guard lives.
pub(crate) struct FakeManaged;

impl FakeManaged {
    pub(crate) fn register() -> Self {
        register_chat_model(
            PROVIDER_OPENHUMAN,
            ChatModelEntry {
                build: Arc::new(build),
                probe: None,
            },
        );
        Self
    }
}

impl Drop for FakeManaged {
    fn drop(&mut self) {
        unregister_chat_model(PROVIDER_OPENHUMAN);
    }
}
