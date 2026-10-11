//! Recording native inference for policy refusal before an agent turn.

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tinyinference_llm::model::{ChatModel, ModelRequest, ModelResponse};

pub(super) struct RecordingModel(pub Arc<AtomicUsize>);

#[async_trait]
impl ChatModel<()> for RecordingModel {
    async fn invoke(
        &self,
        _state: &(),
        _request: ModelRequest,
    ) -> tinyinference_llm::Result<ModelResponse> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ModelResponse::assistant("policy fixture completed"))
    }
}
