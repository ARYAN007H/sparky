//! OpenHuman-specific policy layered on TinyInference's provider-neutral classifier.

use tinyinference_llm::failure::{
    classify_provider_failure_with, ProviderFailureClass,
};

fn classify(err: &anyhow::Error) -> ProviderFailureClass {
    let message = err.to_string();

    let status = err
        .downcast_ref::<reqwest::Error>()
        .and_then(reqwest::Error::status)
        .map(|status| status.as_u16());
    classify_provider_failure_with(status, None, &message, |value| {
        crate::core::observability::is_session_expired_message(value)
    })
}

pub(crate) fn is_rate_limited(err: &anyhow::Error) -> bool {
    matches!(
        classify(err),
        ProviderFailureClass::RateLimited | ProviderFailureClass::NonRetryableRateLimit
    )
}

pub(crate) fn is_upstream_unhealthy(err: &anyhow::Error) -> bool {
    classify(err) == ProviderFailureClass::UpstreamUnhealthy
}

pub(crate) fn parse_retry_after_ms(err: &anyhow::Error) -> Option<u64> {
    let message = err.to_string();
    tinyinference_core::retry_after::parse_retry_after_ms(Some(&message))
}
