//! One host copy table for user-facing failure text.
//!
//! - [`table`]: failure class -> (`error_type` wire token, `source`,
//!   `retryable`, user copy), consumed by `web_chat::web_errors` to build the
//!   `chat_error` envelope.
//! - [`halt`]: the loop-guard halt summaries and the delegated-inference
//!   recogniser that picks between them.
//!
//! Provider-text matching (including the one budget-phrase matcher,
//! `tinyinference_providers::is_budget_message`) lives upstream in
//! `tinyinference`; deciding *which class an error is* stays in the callers.

mod halt;
mod chat_error_catalog;
mod localized;
mod table;

#[cfg(test)]
pub(crate) use halt::TerminalInferenceFailure;
pub(crate) use halt::{
    recoverable_identical_halt_summary, recoverable_no_progress_halt_summary,
    terminal_inference_failure_kind, terminal_inference_halt_summary, user_actionable_escalation,
};
pub(crate) use localized::{render_channel_failure, supported_channel_locale};
pub(crate) use table::{failure_copy, FailureClass};

#[cfg(test)]
#[path = "localized_tests.rs"]
mod localized_tests;
