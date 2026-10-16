//! The first turn of a thread still verbatim in a turn's prompt, kept live
//! across the turn: the turn's memory reads leave the thread out from there,
//! and a compaction during the turn moves it forward.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// The first turn of a thread still verbatim in a turn's prompt.
///
/// The pre-turn step sets it from the transcript's last checkpoint. When a
/// compaction during the turn folds more of the thread, the compaction hook
/// advances it past the user turns it folded, so the turn's next memory read
/// can return them. It never passes the turn's own index: the message being
/// answered, and the reply being written, stay out.
#[derive(Debug)]
pub struct InPromptFrom {
    from: AtomicU32,
    ceiling: u32,
    folded: Mutex<HashSet<u64>>,
}

impl InPromptFrom {
    /// Starts at `from`, never past `ceiling`: the turn's own user-turn index.
    #[must_use]
    pub fn new(from: u32, ceiling: u32) -> Self {
        Self {
            from: AtomicU32::new(from.min(ceiling)),
            ceiling,
            folded: Mutex::default(),
        }
    }

    /// The first turn still in the prompt.
    #[must_use]
    pub fn get(&self) -> u32 {
        self.from.load(Ordering::Acquire)
    }

    /// Moves past the user messages a compaction folded: two memory turns (the
    /// message and its reply) for each one not folded before, so a span folded
    /// again does not count twice. Returns the new value.
    pub fn advance_past<I>(&self, folded_user_texts: I) -> u32
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut seen = self
            .folded
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fresh = folded_user_texts
            .into_iter()
            .filter(|text| {
                let mut hasher = DefaultHasher::new();
                text.as_ref().hash(&mut hasher);
                seen.insert(hasher.finish())
            })
            .count();
        let from = self.get();
        if fresh == 0 {
            return from;
        }
        let step = u32::try_from(fresh).unwrap_or(u32::MAX).saturating_mul(2);
        let next = from.saturating_add(step).min(self.ceiling);
        self.from.store(next, Ordering::Release);
        tracing::debug!(
            from,
            next,
            folded = fresh,
            "[memory:window] compaction moved the in-prompt window"
        );
        next
    }
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
