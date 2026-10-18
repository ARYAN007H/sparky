//! Bind OpenHuman's prebuilt turn models to the harness-owned resolver.

use std::sync::Arc;

use crate::inference::provider::model::TurnChatModel;
use crate::inference::turn_models::TurnModels;
use tinyagents_harness::host::PrebuiltModelResolver;
use tinyinference_llm::model::ErrorSlotModel;

/// Keep the lead's typed provider error in the turn slot while the owner
/// resolver selects child routes from the same prebuilt bundle.
pub(crate) fn from_turn_models(models: &TurnModels) -> PrebuiltModelResolver<()> {
    PrebuiltModelResolver::new(
        models.primary.clone(),
        models.routes.iter().cloned().collect(),
    )
    .with_lead_model(Arc::new(ErrorSlotModel::new(
        models.primary.clone(),
        models.error_slot.clone(),
    )) as TurnChatModel)
}
