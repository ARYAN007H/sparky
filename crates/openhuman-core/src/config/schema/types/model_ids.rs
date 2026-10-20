//! Provider-neutral workload roles used by inference routing.

/// Workload roles understood by the generic inference router.
pub const WORKLOAD_ROLES: [&str; 7] = [
    "chat",
    "reasoning",
    "agentic",
    "coding",
    "burst",
    "summarization",
    "vision",
];
