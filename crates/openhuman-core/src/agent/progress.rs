//! Real-time progress events emitted during an agent turn.
//!
//! Consumers (e.g. the web channel provider) create an
//! `mpsc::Sender<AgentProgress>` and attach it to the [`Agent`] via
//! [`OpenHumanSessionHost::set_on_progress`] before calling [`OpenHumanSessionHost::run_single`].
//! The agent's turn loop sends events through this channel as it
//! progresses — tool calls starting/completing, iteration boundaries,
//! sub-agent lifecycle, etc.
//!
//! This is intentionally separate from [`DomainEvent`] (the global
//! broadcast bus) because progress events are **per-request scoped**:
//! they carry no routing info (client_id, thread_id) — the consumer
//! that created the channel already knows those and tags the outgoing
//! socket events accordingly.

/// The progress vocabulary is TinyAgents' (`tinyagents_harness::progress`): the
/// host consumes the very events the harness emits, and the web chat's turn
/// runner carries them as its canonical progress.
pub use tinyagents_harness::progress::AgentProgress;
