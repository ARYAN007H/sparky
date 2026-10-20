//! Product-specific overrides for TinyInference's provider-neutral failure copy.
//!
//! OpenHuman owns its wire keys and branded messages. All neutral classes use
//! TinyInference's defaults through `FailureCopyProvider`.

use tinyinference_llm::failure::copy::{
    failure_copy_with, FailureCopy, FailureCopyClass, FailureCopyProvider,
};

/// A failure class the chat surface renders copy for. Several classifier arms
/// share a class (the "temporarily unavailable" copy is reached from four).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureClass {
    CodexSessionExpired,
    SessionExpired,
    LocalSessionManagedUnavailable,
    ActionBudget,
    MaxIterations,
    TurnTimeout,
    EmptyResponse,
    ChatTemplateRejected,
    RateLimited,
    RateLimitedBilling,
    ManagedRateLimited,
    Timeout,
    AuthError,
    BudgetExhausted,
    ManagedBudgetExhausted,
    ProviderUnavailable,
    ManagedUnavailable,
    PayloadTooLarge,
    ContextOverflow,
    ModelConfigRejected,
    ModelUnavailable,
    CapabilityUnsupported,
    MalformedHistory,
    RequestRejected,
    ManagedRequestRejected,
    ManagedMalformedRequest,
    Network,
    Inference,
    ManagedInternal,
}

struct OpenHumanFailureCopy;

impl FailureCopyProvider for OpenHumanFailureCopy {
    fn copy(&self, class: FailureCopyClass) -> Option<FailureCopy> {
        use FailureCopyClass::*;
        let copy = match class {
            ChatTemplateRejected => Some("This model's chat template rejected the request — it isn't the model, the temperature, or your API key. Local models without native tool calling are driven through their own chat template, and some templates refuse the message shape of a tool step. Start a new chat to reset the history, or pick a model with native tool support in Connections → API keys → LLM."),
            RateLimited => Some("Your AI provider is rate-limiting requests. This is a transient upstream limit, not a thread-level block — you can retry in this thread."),
            RateLimitedBilling => Some("Your AI provider is rejecting requests for billing or plan reasons (out of credits, plan limit, or unavailable model). Retrying won't help — open Settings to top up, upgrade your plan, or pick a different model."),
            Authentication => Some("There's an authentication issue with the AI provider. Please check your API key in settings."),
            EmptyResponse => Some("The model returned an empty response. Please retry. If it keeps happening, try a different model or check its setup in Connections → API keys → LLM."),
            ModelConfigRejected => Some("Your AI provider rejected the request's model or temperature setting. Check your model and routing in Settings → LLM."),
            CapabilityUnsupported => Some("This model can't process images. Remove the attachment or switch to a vision-capable model in Connections → API keys → LLM."),
            RequestRejected => Some("The AI provider rejected the request — this is usually a model or parameter incompatibility. Try a different model in Connections → API keys → LLM."),
            _ => None,
        }?;
        let mut row = tinyinference_llm::failure::copy::default_failure_copy(class)?;
        row.copy = copy;
        Some(row)
    }
}

/// Looks up OpenHuman's branded row or falls back to TinyInference's neutral
/// default for provider-owned failure classes.
pub(crate) fn failure_copy(class: FailureClass) -> FailureCopy {
    use FailureClass::*;
    let owner_class = match class {
        ChatTemplateRejected => Some(FailureCopyClass::ChatTemplateRejected),
        EmptyResponse => Some(FailureCopyClass::EmptyResponse),
        RateLimited => Some(FailureCopyClass::RateLimited),
        RateLimitedBilling => Some(FailureCopyClass::RateLimitedBilling),
        Timeout => Some(FailureCopyClass::Timeout),
        AuthError => Some(FailureCopyClass::Authentication),
        PayloadTooLarge => Some(FailureCopyClass::PayloadTooLarge),
        ContextOverflow => Some(FailureCopyClass::ContextOverflow),
        ModelConfigRejected => Some(FailureCopyClass::ModelConfigRejected),
        ModelUnavailable => Some(FailureCopyClass::ModelUnavailable),
        CapabilityUnsupported => Some(FailureCopyClass::CapabilityUnsupported),
        ProviderUnavailable => Some(FailureCopyClass::ProviderUnavailable),
        RequestRejected => Some(FailureCopyClass::RequestRejected),
        Network => Some(FailureCopyClass::Network),
        Inference => Some(FailureCopyClass::Inference),
        _ => None,
    };
    if let Some(owner_class) = owner_class {
        return failure_copy_with(&OpenHumanFailureCopy, owner_class)
            .expect("known TinyInference failure copy class");
    }

    macro_rules! row {
        ($key:literal, $error_type:literal, $source:literal, $retryable:literal, $copy:literal) => {
            FailureCopy {
                key: $key,
                error_type: $error_type,
                source: $source,
                retryable: $retryable,
                copy: $copy,
            }
        };
    }

    match class {
        CodexSessionExpired => row!("chat_error.codex_session_expired", "provider_error", "auth", false, "Your Codex session has expired. Please reconnect it in Settings → Integrations."),
        SessionExpired => row!("chat_error.session_expired", "session_expired", "auth", false, "Your OpenHuman session has expired. Please sign in again to continue."),
        LocalSessionManagedUnavailable => row!("chat_error.local_session_managed_unavailable", "auth_error", "config", false, "You're on the local offline profile, which has no OpenHuman account behind it, so the managed (cloud) model can't run. Sign in to use managed models, or switch routing to \"Use Your Own Models\" in Connections → API keys → LLM and add your own provider."),
        ActionBudget => row!("chat_error.action_budget", "action_budget_exceeded", "openhuman_budget", true, "You've hit OpenHuman's per-hour action budget — this is a local safety cap, not your AI provider. The window decays gradually; you can keep chatting in this thread and tool-heavy steps will resume as the budget refills."),
        MaxIterations => row!("chat_error.max_iterations", "max_iterations", "agent_loop", true, "The agent ran the maximum number of tool steps for one turn without finishing. This usually means a tool kept failing (often a rate limit on a web fetch). You can retry the same question in this thread once the underlying limit clears."),
        TurnTimeout => row!("chat_error.turn_timeout", "turn_timeout", "agent_loop", true, "This turn ran past its time budget without finishing and was stopped so it wouldn't hang. This usually means a tool call or a delegated sub-agent stalled. You can retry your question in this thread."),
        ManagedRateLimited => row!("chat_error.managed_rate_limited", "rate_limited", "provider", true, "Your AI provider is rate-limiting requests. You can retry in this thread."),
        BudgetExhausted => row!("chat_error.budget_exhausted", "budget_exhausted", "openhuman_billing", false, "You're out of credits, so I can't run the managed (cloud) model right now. You can top up your credits or pick a plan to continue — or, if you've enabled a local model like Ollama, switch routing to \"Use Your Own Models\" in Connections → API keys → LLM."),
        ManagedBudgetExhausted => row!("chat_error.managed_budget_exhausted", "budget_exhausted", "openhuman_billing", false, "You're out of credits. Top up, or switch to 'Use Your Own Models' in Settings."),
        ManagedUnavailable => row!("chat_error.managed_unavailable", "provider_error", "provider", true, "The AI service is temporarily unavailable — we've been notified. Please try again shortly."),
        MalformedHistory => row!("chat_error.malformed_history", "provider_request_rejected", "provider", true, "We hit a temporary glitch in this conversation — we've cleared it. Please send your message again."),
        ManagedRequestRejected => row!("chat_error.managed_request_rejected", "provider_request_rejected", "provider", false, "The request was rejected — usually a model or parameter mismatch. Try a different model in Connections → API keys → LLM."),
        ManagedMalformedRequest => row!("chat_error.managed_malformed_request", "provider_request_rejected", "provider", false, "Something went wrong with this message. Try rephrasing it — or start a new thread if it keeps happening."),
        ManagedInternal => row!("chat_error.managed_internal", "inference", "provider", true, "Something went wrong — we've been notified. Please try again."),
        _ => unreachable!("neutral failure classes use TinyInference defaults"),
    }
}
