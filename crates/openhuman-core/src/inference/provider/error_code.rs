//! Generic adapters over an installed backend error classifier.

/// A recognized provider-neutral backend error category retained for callers
/// that need a compact enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendErrorCode {
    RateLimited,
    UserInsufficientCredits,
    UpstreamUnavailable,
    ModelUnavailable,
    PayloadTooLarge,
    ContextLengthExceeded,
    BadRequest,
    InternalError,
}

impl BackendErrorCode {
    pub fn from_token(token: &str) -> Option<Self> {
        Some(match token {
            "RATE_LIMITED" => Self::RateLimited,
            "USER_INSUFFICIENT_CREDITS" => Self::UserInsufficientCredits,
            "UPSTREAM_UNAVAILABLE" => Self::UpstreamUnavailable,
            "MODEL_UNAVAILABLE" => Self::ModelUnavailable,
            "PAYLOAD_TOO_LARGE" => Self::PayloadTooLarge,
            "CONTEXT_LENGTH_EXCEEDED" => Self::ContextLengthExceeded,
            "BAD_REQUEST" => Self::BadRequest,
            "INTERNAL_ERROR" => Self::InternalError,
            _ => return None,
        })
    }
}

fn classify(err: &str) -> Option<crate::inference::registry::BackendErrorClassification> {
    crate::inference::registry::classify_backend_error(None, err)
}

pub fn extract_backend_error_code_token(err: &str) -> Option<String> {
    classify(err)?.token
}

pub fn is_managed_backend_envelope(err: &str) -> bool {
    classify(err).is_some_and(|result| result.provider_owned)
}

pub fn managed_error_skips_sentry(err: &str) -> bool {
    backend_error_code_skips_sentry(err)
}

pub fn extract_backend_error_code(err: &str) -> Option<BackendErrorCode> {
    extract_backend_error_code_token(err).and_then(|token| BackendErrorCode::from_token(&token))
}

pub fn body_flags_malformed(err: &str) -> bool {
    classify(err).is_some_and(|result| result.malformed_bad_request)
}

pub fn is_backend_malformed_bad_request(err: &str) -> bool {
    extract_backend_error_code(err) == Some(BackendErrorCode::BadRequest)
        && body_flags_malformed(err)
}

pub fn is_backend_client_guard_leak(err: &str) -> bool {
    classify(err).is_some_and(|result| {
        result.reportable
            && matches!(
                result.kind,
                Some(
                    crate::inference::registry::BackendErrorKind::PayloadTooLarge
                        | crate::inference::registry::BackendErrorKind::ContextLengthExceeded
                )
            )
    })
}

pub fn backend_error_code_skips_sentry(err: &str) -> bool {
    classify(err).is_some_and(|result| result.backend_owned && !result.reportable)
}
