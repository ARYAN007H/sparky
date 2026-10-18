//! Host implementation of [`ToolOutcomeClassifier`] over OpenHuman's
//! `tool_status` domain.
//!
//! This is `docs/specs/plan-agents.md` Phase 4. The agent runtime knows a tool
//! call returned; it does not know whether the thing that came back is a
//! success, something to re-dispatch, or a dead end. OpenHuman already owns that
//! judgement in [`crate::tool_status`], whose
//! [`classify`](tinytools::status::classify) turns raw tool error
//! text into a [`ClassifiedFailure`](tinytools::status::ClassifiedFailure)
//! (a [`ToolFailureClass`] plus a user-facing category). This adapter is the one
//! place that mapping is projected onto the crate's three-way
//! [`OutcomeClass`].
//!
//! # Contract mismatches resolved here
//!
//! **1. `ClassifiedFailure::recoverable` is NOT the retryability signal.**
//! The tempting one-liner is `if failure.recoverable { RetryableFailure }`. It
//! is wrong. `recoverable` means `category == FailureCategory::Recoverable`, and
//! [`ToolFailureClass::Unknown`] — *anything the heuristic could not classify* —
//! sits in that category so the UI can offer "try again" copy. The crate's
//! contract for [`OutcomeClass::RetryableFailure`] is much stronger: it is "an
//! assertion by the host that repeating is acceptable", made without knowing
//! whether the tool had side effects. An unclassified failure is precisely the
//! case where OpenHuman does *not* know that. So this adapter branches on the
//! **class**, not on `recoverable`, and only the three transient classes are
//! called retryable.
//!
//! That is not a new policy invention — it is the split OpenHuman's own steering
//! middleware already makes. `TinyAgents`' recoverable-failure headroom ladder
//! (`middleware.rs`, issue #4463 part 4) gates on exactly
//! `Timeout | ServiceUnavailable | ModelConnection` and lets everything else,
//! `Unknown` included, fall through to the hard-failure path. Mapping `Unknown`
//! to [`OutcomeClass::PermanentFailure`] keeps this seam consistent with that
//! guard and matches the crate's own documented safe default
//! (`ErrorFieldClassifier` classifies every error as permanent for the same
//! side-effect reason).
//!
//! **2. `classify` wants a `timed_out` flag the runtime does not carry.**
//! [`ToolResult`] has no "the executor stopped this at its deadline" bit, so the
//! flag is derived the same way `TinyAgentsToolStatusMiddleware` derives it —
//! sniffing `"timed out"` out of the combined failure text. Doing it identically
//! is the point: the middleware and this classifier must not disagree about
//! whether a call timed out.
//!
//! **3. Error text is canonical content.** [`ToolResult::content`] carries text
//! and structured blocks while [`ToolResult::is_error`] says whether execution
//! reported a failure. This adapter renders the canonical blocks, so a
//! `[policy-denied]` marker is honoured wherever the tool put it and a user
//! refusal can never be re-dispatched.
//!
//! **4. A timeout cannot be assumed safe to repeat.** `Timeout` is the one
//! failure class where "the call failed" and "the call succeeded but the reply
//! was lost" are indistinguishable from the outside. For a tool with an
//! external effect — sending an email, moving money, running a shell command —
//! re-dispatching a timed-out call can commit the effect a second time. The
//! crate's `RetryableFailure` is an assertion that repeating is *acceptable*,
//! so it may only be made about a tool the host has positively identified as
//! side-effect free — declared through
//! [`OpenHumanToolOutcomeClassifier::with_retry_safe_tools`] as an
//! **allowlist**. An allowlist rather than the inverse, because
//! `Tool::external_effect()` is arg-less: `ShellTool` and other raw
//! tool classify their effect from *arguments* (`external_effect_with_args`)
//! and leave the arg-less variant at the default `false`, so a denylist built
//! from it would admit precisely the tools that must be excluded. Absent the
//! allowlist, timeouts stay permanent — the safe direction, since a lost retry
//! costs an iteration while a duplicated payment cannot be undone.
//!
//! The adapter is pure, as the trait requires: no I/O, no interior mutability,
//! same answer for the same `(name, result)` every time.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

use tinyagents_harness::host::{OutcomeClass, ToolOutcomeClassifier, ToolStatusClassifier};
use tinytools::ToolResult;

use tinytools::status::ToolFailureClass;

/// OpenHuman's [`ToolOutcomeClassifier`], backed by
/// [`tinytools::status::classify`].
///
/// Zero-sized and stateless: the classifier's whole knowledge base is the
/// keyword heuristic in the `tool_status` domain, which is itself pure. Every
/// instance behaves identically, so callers may construct one per session
/// without cost.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenHumanToolOutcomeClassifier {
    /// Names of tools the host positively knows are safe to call twice.
    ///
    /// An **allowlist**, not a denylist, and deliberately so. The obvious
    /// inverse — "the tools that declare `Tool::external_effect()`" — is unsafe
    /// here, because a tool whose effect depends on its arguments overrides
    /// `external_effect_with_args` and leaves the arg-less
    /// `external_effect()` at the trait default of `false`. `ShellTool` is
    /// exactly that shape, so a denylist built from the arg-less signal would
    /// have called a timed-out shell write retryable. Anything not named here
    /// is treated as potentially effectful.
    ///
    /// Only consulted for [`ToolFailureClass::Timeout`], where "the call failed"
    /// and "the reply was lost after the effect committed" are
    /// indistinguishable from the outside. See [`Self::class_of`].
    retry_safe_tools: Option<Arc<HashSet<String>>>,
}

impl OpenHumanToolOutcomeClassifier {
    /// Creates the classifier with no retry-safe tools declared.
    ///
    /// Timeouts are then treated as **permanent**, because a classifier that
    /// cannot tell an email sender from a file read must not promise that
    /// repeating the call is safe. Attach [`Self::with_retry_safe_tools`] to
    /// recover retries for the tools that can afford them.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches the tools the host positively knows are safe to repeat.
    ///
    /// List a tool here only when calling it twice is *definitionally*
    /// harmless — a pure read. Do **not** build this by inverting
    /// `Tool::external_effect()`: that signal is arg-less, and a tool whose
    /// effect depends on its arguments (`ShellTool`, raw tools)
    /// overrides `external_effect_with_args` while leaving the arg-less
    /// variant at the default `false`. Inverting it would silently admit
    /// exactly the tools that most need excluding.
    pub fn with_retry_safe_tools(mut self, tools: Arc<HashSet<String>>) -> Self {
        self.retry_safe_tools = Some(tools);
        self
    }

    /// Whether repeating `name` after a timeout is safe.
    ///
    /// Membership is the only way to be safe: an unlisted tool — unknown,
    /// arg-sensitive, or simply new — is treated as potentially effectful.
    fn timeout_is_retry_safe(&self, name: &str) -> bool {
        self.retry_safe_tools
            .as_deref()
            .is_some_and(|safe| safe.contains(name))
    }

}

impl ToolOutcomeClassifier for OpenHumanToolOutcomeClassifier {
    fn classify(&self, name: &str, result: &ToolResult) -> OutcomeClass {
        let retry_safe_tools = self.retry_safe_tools.clone();
        ToolStatusClassifier
            .with_retry_safe_timeouts(move |tool| {
                retry_safe_tools
                    .as_deref()
                    .is_some_and(|safe| safe.contains(tool))
            })
            .classify(name, result)
    }
}

#[cfg(test)]
#[path = "tool_outcome_classifier_tests.rs"]
mod tests;
