/* eslint-disable */
// @generated from OpenHuman and TinyAgents Rust JSON Schemas.
// Do not edit by hand; run `pnpm chat-wire:generate`.

/**
 * Lifecycle of an in-flight (or formerly in-flight) turn.
 *
 * `Started` is set when the user sends and the agent loop is about
 * to enter the iteration loop. `Streaming` is set after the first
 * progress signal arrives. `Interrupted` is stamped at startup on
 * any snapshot that survived a process restart — there is no live
 * driver to resume it, so the UI should surface a retry affordance.
 */
export type TurnLifecycle = ('started' | 'streaming' | 'interrupted') | 'completed';
/**
 * High-level phase the agent is in within an iteration.
 */
export type TurnPhase = 'thinking' | 'tool_use' | 'subagent';
/**
 * Per-tool entry shown in the live timeline UI.
 */
export type ToolTimelineStatus = 'running' | 'success' | 'error';
/**
 * One ordered item in a sub-agent's processing transcript — its streamed
 * reasoning (`thinking`), visible narration (`text`), or a tool call, in the
 * exact order they occurred. Mirrors the frontend `SubagentTranscriptItem`
 * union 1:1 (order = push order; no `seq` is needed because each sub-agent's
 * transcript is built as a single ordered list). Persisting these lets the
 * inline "Agentic task insights" thoughts survive a settled turn / reload.
 */
export type SubagentTranscriptItem =
  | {
      iteration?: number | null;
      kind: 'thinking';
      text: string;
    }
  | {
      iteration?: number | null;
      kind: 'text';
      text: string;
    }
  | {
      callId: string;
      detail?: string | null;
      displayName?: string | null;
      elapsedMs?: number | null;
      iteration?: number | null;
      kind: 'tool';
      outputChars?: number | null;
      status: ToolTimelineStatus;
      toolName: string;
    };
/**
 * One ordered item in the parent turn's processing transcript.
 *
 * Unlike [`ToolTimelineEntry`] (a flat list of tool rows), the transcript
 * preserves the **interleaving** of the agent's visible narration, its
 * hidden reasoning, and its tool calls in the exact order they streamed —
 * so the chat "View processing" panel can render prose between tool groups
 * the way Claude / Hermes does. `seq` is a monotonic per-turn ordering key
 * (round alone can't order narration vs thinking within one round). Tool
 * items hold only a `call_id` pointer into [`TurnState::tool_timeline`] so
 * the row's status/label live in exactly one place.
 */
export type TranscriptItem =
  | {
      kind: 'narration';
      round: number;
      seq: number;
      text: string;
    }
  | {
      endedAt?: number | null;
      kind: 'thinking';
      round: number;
      seq: number;
      startedAt?: number | null;
      text: string;
    }
  | {
      callId: string;
      kind: 'toolCall';
      round: number;
      seq: number;
    };

/**
 * Root schema whose properties provide the UI-facing thread record aliases.
 */
export interface ThreadWireTypes {
  agentRun: AgentRun;
  agentRunGet: RunLedgerGetWireResponse;
  agentRunList: AgentRunListResponse;
  clearTurnState: ClearTurnStateResponse;
  getTurnState: GetTurnStateResponse;
  listTurnStates: ListTurnStatesResponse;
  message: ConversationMessageRecord;
  purgeResult: PurgeConversationThreadsResponse;
  runEvent: RunEvent;
  runEventList: RunEventListResponse;
  runTelemetry: RunTelemetry;
  thread: ConversationThreadSummary;
  threadDelete: DeleteConversationThreadResponse;
  threadMessages: ConversationMessagesResponse;
  threadSearch: ThreadSearchResponse;
  threadSearchHit: ThreadSearchHit;
  threadsList: ConversationThreadsListResponse;
}
/**
 * A persisted background agent run: a subagent, worker thread, background
 * agent, team member, or workflow child, as classified by `kind`.
 */
export interface AgentRun {
  /**
   * Identifier of the agent definition driving this run.
   */
  agentId: string | null;
  /**
   * Inline resumable checkpoint state, if small enough to store directly.
   */
  checkpoint: {
    [k: string]: any;
  };
  /**
   * Filesystem path to a resumable checkpoint for this run, if persisted
   * out-of-band from `checkpoint`.
   */
  checkpointPath: string | null;
  /**
   * When the run reached a terminal status, if it has.
   */
  completedAt: string | null;
  /**
   * Error message, if the run ended in `Failed`.
   */
  error: string | null;
  /**
   * Caller-supplied stable identifier.
   */
  id: string;
  /**
   * What kind of run this is.
   */
  kind: 'subagent' | 'worker_thread' | 'background_agent' | 'team_member' | 'workflow_child';
  /**
   * Arbitrary caller-supplied metadata.
   */
  metadata: {
    [k: string]: any;
  };
  /**
   * The run that spawned this one, if any.
   */
  parentRunId: string | null;
  /**
   * The parent conversation thread this run is nested under, if any.
   */
  parentThreadId: string | null;
  /**
   * Opaque reference to the prompt/instructions this run was given.
   */
  promptRef: string | null;
  startedAt: string;
  /**
   * Current lifecycle state.
   */
  status:
    | 'pending'
    | 'running'
    | 'awaiting_user'
    | 'paused'
    | 'completed'
    | 'failed'
    | 'cancelled'
    | 'interrupted';
  /**
   * Human-readable summary of the run's outcome, once known.
   */
  summary: string | null;
  /**
   * Aggregated token/cost telemetry for this run, when joined from
   * `run_telemetry`.
   */
  telemetry: RunTelemetry | null;
  updatedAt: string;
  /**
   * The worker thread id this run executes on, for `WorkerThread` runs.
   */
  workerThreadId: string | null;
}
/**
 * Aggregated token, cost, and status telemetry for a single run.
 *
 * One row per `run_id`, updated in place as a run progresses (see
 * `super::ops::upsert_run_telemetry` for how partial updates are coalesced).
 */
export interface RunTelemetry {
  cachedInputTokens: number;
  costUsd: number;
  /**
   * Total wall-clock time spent executing the run so far, in milliseconds.
   */
  elapsedMs: number | null;
  /**
   * Most recent error message, if any step of the run failed.
   */
  error: string | null;
  inputTokens: number;
  model: string | null;
  outputTokens: number;
  provider: string | null;
  runId: string;
  /**
   * Number of tool calls made so far.
   */
  toolCount: number;
  updatedAt: string | null;
}
export interface RunLedgerGetWireResponse {
  run: AgentRun | null;
}
/**
 * Response shape for `super::ops::list_agent_runs`.
 */
export interface AgentRunListResponse {
  /**
   * Number of runs in `runs` (not the total matching count).
   */
  count: number;
  runs: AgentRun[];
}
/**
 * Response payload for `openhuman.threads_turn_state_clear`.
 */
export interface ClearTurnStateResponse {
  cleared: boolean;
}
/**
 * Response payload for `openhuman.threads_turn_state_get`.
 */
export interface GetTurnStateResponse {
  /**
   * `None` when no snapshot exists for the thread.
   */
  turnState?: TurnState | null;
}
/**
 * Persisted snapshot of an in-flight (or just-finished) agent turn for one
 * thread.
 *
 * Written to disk by the web-channel progress consumer at iteration
 * boundaries, tool start/complete, and on terminal events. On normal
 * completion it is marked [`TurnLifecycle::Completed`] and **kept** (so the
 * "View processing" panel can replay the finished turn's transcript); a
 * non-terminal snapshot surviving startup is marked
 * [`TurnLifecycle::Interrupted`].
 */
export interface TurnState {
  activeSubagent?: string | null;
  activeTool?: string | null;
  iteration: number;
  lifecycle: TurnLifecycle;
  maxIterations: number;
  phase?: TurnPhase | null;
  requestId: string;
  startedAt: string;
  streamingText: string;
  thinking: string;
  threadId: string;
  toolTimeline: ToolTimelineEntry[];
  /**
   * Ordered, interleaved record of the agent's narration, reasoning, and
   * tool calls for the "View processing" panel. `#[serde(default)]` so
   * snapshots written before this field still load (as empty).
   */
  transcript?: TranscriptItem[];
  updatedAt: string;
}
/**
 * One row in the per-turn tool timeline.
 *
 * Field names use camelCase on the wire so a snapshot can be applied
 * directly to `chatRuntimeSlice.toolTimelineByThread` without a
 * translation layer in the UI.
 */
export interface ToolTimelineEntry {
  argsBuffer?: string | null;
  detail?: string | null;
  displayName?: string | null;
  /**
   * Plain-language failure explanation for a FAILED row, carried in the
   * snapshot so it survives a thread switch / cold boot (#4459). `None` on
   * success and on legacy snapshots.
   */
  failure?: PersistedToolFailure | null;
  id: string;
  name: string;
  /**
   * Size-capped tool result text, persisted so the "View processing"
   * panel can show what a tool returned after a thread switch / cold
   * boot — the live socket forwards the same capped payload on
   * `tool_result`. `None` while running and on legacy snapshots.
   */
  output?: string | null;
  round: number;
  /**
   * Per-turn monotonic ordering key stamped at the moment the row is first
   * created, so a rehydrated timeline can order rows identically to the live
   * stream (conversations-timeline-refactor, Phase 4 amendment). Shares the
   * per-turn ordering space with each [`TranscriptItem`]'s `seq`. `None` on snapshots
   * written before this field.
   */
  seq?: number | null;
  sourceToolName?: string | null;
  status: ToolTimelineStatus;
  subagent?: SubagentActivity | null;
}
/**
 * Persisted, plain-language explanation of a FAILED tool row (#4459).
 *
 * Mirrors the live socket `failure` object and the frontend
 * `PersistedToolFailure` (`app/src/types/turnState.ts`) 1:1 — camelCase on the
 * wire, `class`/`category` as the taxonomy's stable variant names — so a
 * settled/reloaded turn keeps its "why + what to do next" copy across a thread
 * switch or a cold boot. Absent on successful rows and on snapshots written
 * before this field.
 */
export interface PersistedToolFailure {
  /**
   * Stable category variant name, e.g. `"Recoverable"`, `"UserDeclined"`.
   */
  category: string;
  /**
   * Plain-language cause (`causePlain` on the wire).
   */
  causePlain: string;
  /**
   * Stable failure-class variant name, e.g. `"Timeout"`, `"Denied"`.
   */
  class: string;
  /**
   * Plain-language next action (`nextAction` on the wire).
   */
  nextAction: string;
  /**
   * Whether the core considers the failure automatically recoverable.
   */
  recoverable: boolean;
}
/**
 * Live sub-agent activity nested under a `subagent:*` timeline row.
 */
export interface SubagentActivity {
  agentId: string;
  childIteration?: number | null;
  childMaxIterations?: number | null;
  dedicatedThread?: boolean | null;
  elapsedMs?: number | null;
  iterations?: number | null;
  mode?: string | null;
  /**
   * Size-capped final assistant text, persisted so a rehydrated row can
   * still show what the sub-agent answered — mirrors the live
   * `subagent_completed` socket payload's `subagent.output`. `None`
   * while running, on failure, and on legacy snapshots.
   */
  output?: string | null;
  outputChars?: number | null;
  /**
   * The parent turn's tool-call id (the `spawn_subagent` / dispatch call)
   * this delegation is attributed to. Mirrors
   * the host's `AgentProgress::SubagentSpawned::parent_call_id`.
   * `None` for legacy snapshots and spawn sites the harness gave no call
   * context (e.g. `orchestration::ops`).
   */
  parentCallId?: string | null;
  /**
   * High-level status: `"running"`, `"awaiting_user"`, `"completed"`,
   * `"failed"`. `None` for legacy snapshots written before this field.
   */
  status?: string | null;
  taskId: string;
  toolCalls: SubagentToolCall[];
  /**
   * Ordered reasoning/narration/tool transcript for this sub-agent — what
   * the inline "Agentic task insights" thoughts render from. Persisted (not
   * live-only) so the thoughts survive a settled turn / reload.
   * `#[serde(default)]` so snapshots written before this field load empty.
   */
  transcript?: SubagentTranscriptItem[];
  /**
   * Persistent worker sub-thread backing this delegation, when one was
   * created. Lets the UI reopen the full parent↔subagent conversation
   * from memory after a cold boot / interrupted turn.
   */
  workerThreadId?: string | null;
}
/**
 * One child tool call performed by a running sub-agent.
 */
export interface SubagentToolCall {
  /**
   * Size-capped arguments the child invoked the tool with, so a rehydrated
   * row shows *what* the sub-agent did and not just which tool it reached
   * for (#5987). Mirrors the live `subagent_tool_call` event's `args`
   * verbatim when it fits the cap; an oversized payload degrades to a
   * truncated string. Taken from the started event when it carries the
   * arguments, otherwise backfilled from
   * the harness's completed sub-agent tool-call event — the tinyagents path
   * emits `Value::Null` at start and only captures the input on
   * completion. `None` when the harness captured no input at all
   * (`PayloadCapture::tool_io` off) and on legacy snapshots.
   *
   * Deliberately absent from [`SubagentTranscriptItem::Tool`]: like
   * `output` and `failure`, this is a heavy payload that lives once on the
   * call row, and the frontend grafts it onto the matching transcript item
   * by `call_id` when it rehydrates.
   */
  args?: {
    [k: string]: any;
  };
  callId: string;
  /**
   * Server-computed contextual detail (e.g. the path / recipient).
   */
  detail?: string | null;
  /**
   * Server-computed human label for this child call (e.g. "Reading file"),
   * or `None` to defer to the client formatter. Mirrors the parent
   * [`ToolTimelineEntry::display_name`] so the same reusable row renderer
   * reads the same field for both main-agent and sub-agent calls.
   */
  displayName?: string | null;
  elapsedMs?: number | null;
  /**
   * Plain-language failure explanation for a FAILED child call, so a
   * sub-agent's failed row carries the same "why + next" copy as a
   * main-agent row and it survives a snapshot round-trip (#4459). `None` on
   * success and on legacy snapshots.
   */
  failure?: PersistedToolFailure | null;
  iteration?: number | null;
  /**
   * Size-capped child tool result text, persisted for the same reason as
   * [`ToolTimelineEntry::output`]. `None` while running and on legacy
   * snapshots.
   */
  output?: string | null;
  outputChars?: number | null;
  status: ToolTimelineStatus;
  toolName: string;
}
/**
 * Response payload for `openhuman.threads_turn_state_list` and
 * `openhuman.threads_turn_state_history`.
 */
export interface ListTurnStatesResponse {
  count: number;
  turnStates: TurnState[];
}
/**
 * A single persisted conversation message.
 */
export interface ConversationMessageRecord {
  content: string;
  createdAt: string;
  extraMetadata: {
    [k: string]: any;
  };
  id: string;
  sender: string;
  type: string;
}
/**
 * Response payload for purging all workspace-backed conversations.
 */
export interface PurgeConversationThreadsResponse {
  agentMessagesDeleted: number;
  agentThreadsDeleted: number;
  messagesDeleted: number;
}
/**
 * One entry in a run's append-only event log.
 *
 * `sequence` is allocated by the insert itself (see `super::ops`) rather
 * than computed by the caller, so concurrent appenders cannot collide.
 */
export interface RunEvent {
  /**
   * Caller-defined event type discriminator.
   */
  eventType: string;
  /**
   * Arbitrary event payload.
   */
  payload: {
    [k: string]: any;
  };
  runId: string;
  /**
   * Per-run monotonically increasing sequence number; the primary key
   * together with `run_id`.
   */
  sequence: number;
  timestamp: string;
}
/**
 * Response shape for `super::ops::list_recent_run_events`.
 */
export interface RunEventListResponse {
  /**
   * Number of events in `events` (not the total matching count).
   */
  count: number;
  events: RunEvent[];
}
/**
 * Summary information for a workspace-backed conversation thread.
 */
export interface ConversationThreadSummary {
  /**
   * Working folder bound to this thread; absent means the global default.
   */
  actionDir?: string | null;
  chatId?: number | null;
  createdAt: string;
  id: string;
  isActive: boolean;
  labels: string[];
  lastMessageAt: string;
  messageCount: number;
  parentThreadId?: string | null;
  personalityId?: string | null;
  title: string;
}
/**
 * Response payload for single-thread deletion.
 */
export interface DeleteConversationThreadResponse {
  deleted: boolean;
}
/**
 * Response payload for message list operations.
 */
export interface ConversationMessagesResponse {
  count: number;
  messages: ConversationMessageRecord[];
}
/**
 * Response for [`thread_search`]: hits newest first.
 */
export interface ThreadSearchResponse {
  hits: ThreadSearchHit[];
}
/**
 * One matched message.
 */
export interface ThreadSearchHit {
  createdAt: string;
  messageId: string;
  role: string;
  /**
   * The message text around the first match, with `…` where it was cut.
   */
  snippet: string;
  threadId: string;
}
/**
 * Response payload for thread list operations.
 */
export interface ConversationThreadsListResponse {
  count: number;
  threads: ConversationThreadSummary[];
}
