//! Business logic for the `flows::` domain: validate-on-save CRUD plus the
//! end-to-end `flows_run` / `flows_resume` path. Delegated to from
//! `schemas.rs`'s `handle_*` RPC/CLI handlers, mirroring
//! `crates/openhuman-core/src/cron/ops.rs`.
//!
//! The module is split by responsibility; every operation the rest of the
//! crate reaches through `flows::ops::` is re-exported from here so the
//! external paths are unchanged:
//!
//! | Submodule | Responsibility |
//! | --- | --- |
//! | `validation` | migrate/validate a raw graph, engine compatibility, `flows_validate` / `flows_import` |
//! | `builder_gates` | the author-time hard-gate stack, `strict_gate`, `build_builder_proposal` |
//! | `wiring_warnings` | TinyFlows output and split-path advisories via the scoped host contract callback |
//! | `inference_readiness` | B45 provider-connectivity evaluation + probe cache |
//! | `host_gates` | OpenHuman checks served to TinyFlows for one authoring call |
//! | `definitions` | create / duplicate / get / list / delete + saved-graph lookups |
//! | `updates` | update / history / rollback with optimistic concurrency |
//! | `triggers` | enable/disable, trigger binding, boot reconciliation, trigger warnings |
//! | `connections` | the `connection_ref` picker list and required-connection CTAs |
//! | `run` | `flows_run` / `flows_run_detached` entry points and run prep |
//! | `execution` | Langfuse export and approval notification |
//! | `resume` | `flows_resume` and the T-M1 graph pin |
//! | `run_management` | cancel, run listing/pruning, TTL and boot sweeps |
//! | `discovery` | Flow Scout discovery and suggestion lifecycle |
//! | `streaming` | copilot/scout progress bridge onto the web-channel socket |
//! | `builder_toolset` | tools hidden from the `workflow_builder` belt per path |
//! | `builder` | `flows_build` / `flows_build_cancel` and proposal extraction |
//! | `trail_off` | the builder-convergence question backstop |
//! | `approval_manifest` | save-time approval manifest |
//! | `catalog` | tool-catalog search / contract RPCs |
//! | `drafts` | core-managed local drafts |

use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use serde_json::{json, Value};
use tinyflows::model::{NodeKind, TriggerKind, WorkflowGraph};
// Generic graph analysis used for UI notes and authoring summaries comes from
// the TinyFlows bus. OpenHuman-specific save and dispatch policy stays here.
use tinyflows::triggers::graph_trigger_warnings;
// The save/run safety predicates are `tinyflows-catalog`'s: whether a graph
// fires unattended, whether it can act on the world, whether it has anything to
// do at all are properties of the graph, not of this host. Re-exported at
// `ops::` scope because the agent tools and this module's tests already name
// them there.
use crate::config::Config;
use crate::core::Outcome;
use crate::flows::bus;
use crate::flows::draft_store;
use crate::flows::store;
use crate::security::approval::{ApprovalChatContext, ApprovalTurnContext, FlowRunContext};
use crate::security::turn_origin::{with_origin, AgentTurnOrigin, TrustedAutomationSource};
use tinyflows_catalog::build_registry;
pub(crate) use tinyflows_catalog::graph_policy::{
    enforce_side_effect_approval, graph_has_actionable_nodes, trigger_is_automatic,
};
use tinyflows_catalog::types::{FlowConnection, FlowRunTrigger, FlowSuggestion, SuggestionStatus};
use tokio_util::sync::CancellationToken;
use {tinyflows_catalog::Flow, tinyflows_catalog::FlowRun};

/// Overall safety bound on a single `flows_run` / `flows_resume`. Individual
/// capabilities have their own timeouts (HTTP, sandbox), but a hung LLM/tool
/// call must never let the RPC block indefinitely — this caps the whole run.
const FLOW_RUN_TIMEOUT_SECS: u64 = crate::flows::runs_adapter::RUN_TIMEOUT_SECS;

/// How long a run may sit parked at a human-in-the-loop approval gate
/// (`pending_approval`) before the TTL sweep expires it to a terminal
/// `"cancelled"` (issue G4). Aligned with the agent tool-call `ApprovalGate`'s
/// 10-minute fail-closed TTL (`crates/openhuman-core/src/security/approval/`), so a flow HITL gate a
/// human never answers doesn't wedge a run — and its durable checkpoint —
/// forever. The two are distinct mechanisms (flow runs execute as
/// `TrustedAutomation { Workflow }`, which the tool-call gate lets through), so
/// this is a dedicated flows-side TTL, not a reuse of the approval store's.
#[cfg(test)]
const FLOW_PARKED_TTL_SECS: i64 = tinyflows_catalog::run_management::PARKED_RUN_TTL_SECS;

/// T-M1 fail-closed refusal: the graph hash pinned when this run parked no
/// longer matches the flow's current graph (`save_workflow` rewrote it while
// ─────────────────────────────────────────────────────────────────────────────
// Phase 2 — autonomy-tier gating of acting flow nodes
// ─────────────────────────────────────────────────────────────────────────────
//
// A `flows_run` / `flows_resume` executes under a trusted workflow origin, but
// its acting power is still bounded by the user's `[autonomy]` tier. The
// `CapsHost` adapter applies OpenHuman policy and approvals at the host
// callback; TinyFlows owns graph execution and lifecycle settlement.
//
// Before an acting node dispatches, its capability adapter
// (`crates/openhuman-core/src/flows/caps_host/`) maps the node to a
// `CommandClass` and consults `SecurityPolicy::gate_decision`. `Block` refuses
// outright (`[policy-blocked]` error, no dispatch); `Prompt`/`Allow` fall through
// to the process-global `ApprovalGate`, which performs the human round-trip for
// `Prompt` exactly as the agent tool-loop does. Node → class → per-tier decision:
//
//   Flow node        CommandClass   read-only     supervised    full
//   ────────────     ────────────   ──────────    ──────────    ──────────
//   http_request     Network        BLOCK         Prompt        Prompt
//   code             Write          BLOCK         Prompt        Allow
//   tool_call        (curation +    (curated +    Prompt        Prompt/Allow¹
//                     ApprovalGate)   scope gate)
//   agent (llm)      — (no acting side effect; not tier-gated, only the
//                        inference/privacy chokepoint applies)
//   state (kv)       — (host-internal flow KV; not an outbound act)
//
//   ¹ tool_call routes through the deny-by-default curation/scope gate plus the
//     ApprovalGate rather than `gate_decision`; a Network-class Composio action
//     still prompts under supervised/full and the curation gate is the hard
//     allowlist. See `caps_host::service_tools`.
//
// `Network` is never `Allow` in any tier (always `Prompt` when not blocked), so
// even a full-tier http_request node prompts unless a pre-declared trust root /
// `auto_approve` short-circuits the ApprovalGate — matching `curl`/`shell`.
// `Write` (code) is `Allow` under full, so trusted automations run sandboxed
// code unattended; read-only blocks both outright.
mod approval_manifest;
mod builder;
mod builder_gates;
mod builder_toolset;
mod catalog;
mod connections;
mod definitions;
mod discovery;
mod drafts;
mod execution;
mod inference_readiness;
mod resume;
mod run;
mod run_management;
mod streaming;
mod trail_off;
mod triggers;
mod updates;
mod validation;
mod wiring_warnings;

pub use approval_manifest::*;
pub use builder::*;
pub(crate) use builder_gates::*;
use builder_toolset::*;
pub use catalog::*;
pub use connections::*;
pub use definitions::*;
pub use discovery::*;
pub use drafts::*;
pub(crate) use execution::{export_run_to_langfuse, notify_pending_approval};
pub(crate) use inference_readiness::*;
pub use resume::*;
pub use run::*;
pub use run_management::*;
pub use streaming::*;
use trail_off::*;
pub use triggers::*;
pub use updates::*;
pub use validation::*;
pub(crate) use wiring_warnings::*;

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
