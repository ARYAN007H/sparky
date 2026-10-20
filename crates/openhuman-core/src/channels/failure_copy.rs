//! Shared localized copy for channel-host failures that are not inference
//! failures (for example module setup, durable history, or reset errors).

const GENERIC_FAILURE_KEY: &str = "chat_error.inference";
const GENERIC_FAILURE_EN: &str = "Something went wrong. Please try again.";

/// Render a non-inference channel failure without exposing implementation
/// details or classifying it as a provider failure.
pub(crate) fn generic_failure(config: Option<&crate::config::Config>) -> String {
    match config {
        Some(config) => config.render_native_channel_failure(
            GENERIC_FAILURE_EN,
            Some(GENERIC_FAILURE_KEY),
            None,
        ),
        None => GENERIC_FAILURE_EN.to_owned(),
    }
}
