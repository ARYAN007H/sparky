//! [`StopLatch`]: how the failure breaker records why it stopped a run.
//!
//! The stop-note slot ([`HaltSummarySlot`]) is shared by the failure breaker,
//! the repeat guard (`RepeatProgressMiddleware`) and the stop hooks, and the
//! user-facing stop summary is derived from whatever note it holds when the
//! run ends (`session_host::stop_summary`). A stop only takes effect at the
//! top of the next iteration, so every remaining result of the batch still
//! reaches the breakers after one of them stopped the run. Each used to write
//! the slot unconditionally, so the last breaker to trip in a batch named the
//! cause, not the first: the repeat guard stopping a run while admitting a
//! batch of re-reads, then a mistyped path in that batch failing again, was
//! summarised as "something this needs could not be found".
//!
//! The latch keeps the first note, puts this breaker's note back if the
//! repeat guard (which writes unconditionally) replaced it later in the same
//! batch, and relabels a classified stop that a repeat loop produced (see
//! [`super::repeat_loop_stop`]).

use std::sync::Mutex;

use tinyagents_harness::steering::{SteeringCommand, SteeringHandle};

use super::repeat_loop_stop::{is_correctable_class, repeat_loop_halt_summary, RepeatLoopWitness};
use crate::agent::tinyagents::HaltSummarySlot;

/// The failure breaker's side of the shared stop-note slot.
pub(super) struct StopLatch {
    handle: SteeringHandle,
    slot: HaltSummarySlot,
    /// The note this breaker latched in the current run.
    own: Mutex<Option<String>>,
    /// Repeat-guard refusals and repeated failing calls in the current run.
    witness: RepeatLoopWitness,
    /// `(tool, error line)` of a latched classified stop on an identical
    /// re-issue that no refusal has explained yet. A refusal later in the
    /// same batch relabels the stop as a repeat loop.
    candidate: Mutex<Option<(String, String)>>,
}

impl StopLatch {
    /// A latch that pauses runs through `handle` and records notes in `slot`.
    pub(super) fn new(handle: SteeringHandle, slot: HaltSummarySlot) -> Self {
        Self {
            handle,
            slot,
            own: Mutex::default(),
            witness: RepeatLoopWitness::default(),
            candidate: Mutex::default(),
        }
    }

    /// What has been seen of the repeat guard and of repeated failures.
    pub(super) fn witness(&self) -> &RepeatLoopWitness {
        &self.witness
    }

    /// Stop the run, recording `summary` as its stop note unless another
    /// breaker already stopped it. The pause is sent either way; it is
    /// idempotent.
    pub(super) fn halt(&self, summary: String) {
        if let Ok(mut slot) = self.slot.lock() {
            match slot.as_deref() {
                Some(existing) => tracing::debug!(
                    existing = %crate::util::truncate_with_ellipsis(existing, 80),
                    "[tinyagents::mw] run already stopped by an earlier breaker; keeping its stop note"
                ),
                None => {
                    *slot = Some(summary.clone());
                    if let Ok(mut own) = self.own.lock() {
                        *own = Some(summary);
                    }
                }
            }
        }
        self.handle.send(SteeringCommand::Pause);
    }

    /// Stop the run for an exhausted classified-failure budget.
    ///
    /// When the failing call is an identical re-issue (`repeated`) of a
    /// correctable class and the repeat guard has refused calls since the
    /// last success, the run was looping, and the note says so. With no
    /// refusal yet, a refusal later in the batch can still relabel it
    /// ([`Self::record_refusal`]).
    pub(super) fn halt_classified(
        &self,
        summary: String,
        class: &str,
        tool: &str,
        repeated: bool,
        error_line: impl FnOnce() -> String,
    ) {
        if !(repeated && is_correctable_class(class)) {
            return self.halt(summary);
        }
        let error_line = error_line();
        match self.witness.refusals() {
            0 => {
                if let Ok(mut candidate) = self.candidate.lock() {
                    *candidate = Some((tool.to_owned(), error_line));
                }
                self.halt(summary);
            }
            refusals => self.halt(repeat_loop_halt_summary(tool, refusals, &error_line)),
        }
    }

    /// The repeat guard refused a call. Relabels this breaker's classified
    /// stop as a repeat loop if it is waiting for that evidence and is still
    /// the run's stop note.
    pub(super) fn record_refusal(&self) {
        let refusals = self.witness.record_refusal();
        let Some((tool, error_line)) = self.candidate.lock().ok().and_then(|mut c| c.take()) else {
            return;
        };
        let (Ok(mut slot), Ok(mut own)) = (self.slot.lock(), self.own.lock()) else {
            return;
        };
        if own.is_some() && *slot == *own {
            tracing::debug!(
                tool = %tool,
                refusals,
                "[tinyagents::mw] classified stop came from a repeat loop; relabelling its stop note"
            );
            let summary = repeat_loop_halt_summary(&tool, refusals, &error_line);
            *slot = Some(summary.clone());
            *own = Some(summary);
        }
    }

    /// End of a run: put this breaker's note back if a later writer replaced
    /// it, and forget the run's latch state. The repeat guard writes the slot
    /// unconditionally, so a result later in the batch that trips it would
    /// otherwise relabel a stop this breaker made first.
    pub(super) fn finish_run(&self) {
        if let Ok(mut candidate) = self.candidate.lock() {
            *candidate = None;
        }
        let own = self.own.lock().ok().and_then(|mut own| own.take());
        if let (Some(own), Ok(mut slot)) = (own, self.slot.lock()) {
            if slot.as_deref().is_some_and(|current| current != own) {
                tracing::debug!(
                    "[tinyagents::mw] restoring the failure breaker's stop note replaced later in its batch"
                );
                *slot = Some(own);
            }
        }
    }
}
