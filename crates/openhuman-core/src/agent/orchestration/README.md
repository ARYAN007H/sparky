# orchestration

`agent::orchestration` hosts live OpenHuman adapters for detached sub-agents:
spawning, steering, waiting, progress, and durable
subagent-session projections. TinyAgents owns generic task execution and
registry behavior; OpenHuman supplies product policy, execution context, RPC,
and UI-facing projections.

## Responsibilities

- Host adapters that dispatch work through `agent::subagent_host::run_subagent`.
- Background sub-agent task tracking, cancellation, steering, completion
  delivery, and run-ledger projection.
- Manual cancellation and steering for detached `spawn_async_subagent` runs.
- The shared root `ParentExecutionContext` builder used by background tasks.

## Key files

- `command_center/`: background agent run views and control verbs.
- `running_subagents.rs` and `running_subagents/`: detached run metadata,
  registry, steering, wait, task ledger, and boot reconciliation.
- `background_completions.rs`, `completion_notice.rs`, and
  `background_delivery.rs`: host delivery policy over TinyAgents completion
  routing.
- `subagent_control.rs`: manual cancel and steer RPCs.
- `subagent_sessions/`: durable session reuse and deduplication.
- `parent_context/builder.rs`: construction for tasks without a parent turn.
- `delegation.rs`: live TinyAgents delegation graph adapter.
- `tools.rs`: callable sub-agent tools.

## RPC

| Namespace | Surface |
| --- | --- |
| `agent_work` | Background run list and controls |
| `subagent` | Cancel and steer |

## Tools

The module exposes `spawn_subagent`, `spawn_async_subagent`,
`steer_subagent`, `continue_subagent`,
`close_subagent`, `wait_subagent`, `wait`, `wait_loop`, and `list_subagents`.
`delegate_graph` and archetype delegates provide optional delegation tools.
Execution routes through `agent::subagent_host::run_subagent`.

## Persistence and policy

TinyAgents' session ledger stores `agent_runs` and `run_events`. OpenHuman
keeps scoped sub-agent session records and detached task metadata, and projects
terminal events into the product AgentRun view. Child tools, model routing,
sandbox context, spawn depth, and progress derive from the parent
`ParentExecutionContext`; this module must not widen harness tool visibility.

## Used by

- `agent/registration.rs` registers the callable tools; `tools::ops` composes that family.
- `threads` cancels detached work when a thread closes.
- `crate::core::all` registers the remaining controller pairs.
- The frontend uses the background run and sub-agent controls.
