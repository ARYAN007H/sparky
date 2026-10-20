/**
 * Frontend view of the chat wire contract.
 *
 * Every shape here is DERIVED from `chatWire.generated.ts`, which is generated
 * from the tinychannels `WebChannelEvent` JSON schema (`pnpm chat-wire:generate`;
 * CI runs `pnpm chat-wire:check`). Do not re-declare a wire field by hand: change
 * the schema upstream and regenerate.
 *
 * Two derivations are applied to the generated variants:
 * - `null` is dropped from optional fields. The core omits `None` on the wire
 *   (`skip_serializing_if`), so `null` never arrives; consumers read absence.
 * - fields the core always sets for an event are made required (see each alias).
 *
 * The only hand-written shapes are the typed views of fields the schema leaves
 * as open JSON (`goal`, `todos`, `copy_params`, `citations`, tool `args`). Each
 * is marked "open JSON in the schema" below.
 */
import type { ChatErrorCopyParams } from '../lib/chatErrorCopy';
import type {
  ChatSuggestion,
  ContentPart,
  OutboundContentPart,
  WebChannelEvent,
  WebGuardrailPayload,
  WebGuardrailReason,
  WebQueueItemPayload,
  WebSubagentProgressDetail,
  WebSubagentUsagePayload,
  WebTurnTimingPayload,
  WebTurnUsagePayload,
} from './chatWire.generated';

export type { ContentPart, OutboundContentPart, WebChannelEvent };

type Tag = WebChannelEvent['event'];
type Variant<K extends Tag> = Extract<WebChannelEvent, { event: K }>;
type NoNull<T> = { [P in keyof T]: Exclude<T[P], null> };
type Req<T, K extends keyof T> = Omit<T, K> & { [P in K]-?: Exclude<T[P], null | undefined> };

/**
 * Wire event `K` as consumers see it: nulls dropped, `R` guaranteed present.
 * The `event` tag is the Socket.IO event name, so it is not repeated in the
 * payload type, and `client_id` (routing only) is optional for consumers.
 */
export type WireEvent<K extends Tag, R extends keyof Variant<K> = never> = Omit<
  Req<NoNull<Variant<K>>, R>,
  'event' | 'client_id'
> & { client_id?: string };

// ── Shared payloads ──────────────────────────────────────────────────────────

/** One sub-agent's token/cost contribution within a turn. */
export type SubagentUsageWire = WebSubagentUsagePayload;

/** Where a reported cost came from. */
export type CostSource = NonNullable<WebTurnUsagePayload['cost_source']>;

/** Token/cost/context totals for a completed turn (`chat_done.usage`, `turn_cost`). */
export type TurnUsageWire = WebTurnUsagePayload;

/** Turn latency snapshot (`chat_done.timing`). */
export type TurnTimingWire = NoNull<WebTurnTimingPayload>;

export type GuardrailReason = WebGuardrailReason;
export type GuardrailPayload = Req<WebGuardrailPayload, 'reasons'>;

/** Per-event sub-agent detail attached to `subagent_*` events. */
export type SubagentProgressDetail = NoNull<WebSubagentProgressDetail>;

/** One run-queue item. */
export type QueueItemPayload = NoNull<WebQueueItemPayload>;

/** One follow-up suggestion chip. */
export type ChatSuggestionWire = NoNull<ChatSuggestion>;

/** A memory citation on `chat_done` / `chat_segment` (open JSON in the schema). */
export interface ChatCitation {
  id: string;
  key: string;
  namespace?: string;
  score?: number;
  timestamp: string;
  snippet: string;
}

// ── Turn lifecycle ───────────────────────────────────────────────────────────

export type ChatInferenceStartEvent = WireEvent<'inference_start'>;
export type ChatInferenceHeartbeatEvent = WireEvent<'inference_heartbeat'>;
export type ChatIterationStartEvent = WireEvent<'iteration_start', 'round' | 'message'>;

/**
 * Fields the schema does not carry and the current core never sends: they
 * exist only for the segmented-delivery path and the flat-token fallback in
 * the chat runtime, which read replayed/older-core events. Delete them with
 * those paths.
 */
interface LegacyDoneFields {
  segment_total?: number | null;
  /** @deprecated superseded by `usage`. */
  total_input_tokens?: number;
  /** @deprecated superseded by `usage`. */
  total_output_tokens?: number;
}

export type ChatDoneEvent = Omit<WireEvent<'chat_done', 'full_response'>, 'citations'> & {
  citations?: ChatCitation[] | null;
} & LegacyDoneFields;

export type ChatSegmentEvent = WireEvent<
  'chat_segment',
  'full_response' | 'segment_index' | 'segment_total'
> & { citations?: ChatCitation[] | null };

export type ChatInterimEvent = WireEvent<'chat_interim', 'full_response' | 'round'>;

export type ChatErrorEvent = Omit<
  WireEvent<'chat_error', 'message' | 'error_type'>,
  'copy_params' | 'guardrail'
> & { copy_params?: ChatErrorCopyParams; guardrail?: GuardrailPayload };

export type ChatCancelledEvent = WireEvent<'chat_cancelled'>;

export type ProactiveMessageEvent = WireEvent<'proactive_message', 'full_response'>;

export type ExternalTransferPendingEvent = WireEvent<'external_transfer_pending'>;

/** The egress descriptor the core packs into `external_transfer_pending.args`. */
export function externalTransferProvider(event: ExternalTransferPendingEvent): string | undefined {
  const slug = (event.args as { provider_slug?: unknown } | undefined)?.provider_slug;
  return typeof slug === 'string' ? slug : undefined;
}

// ── Streaming ────────────────────────────────────────────────────────────────

export type ChatTextDeltaEvent = WireEvent<'text_delta', 'delta' | 'round'>;
export type ChatThinkingDeltaEvent = WireEvent<'thinking_delta', 'delta' | 'round'>;
/** Provider event with an OpenHuman row ID only when ownership is exact. */
export type ChatChannelEventEvent = WireEvent<'channel_event', 'channel_event'>;
export type ChatToolArgsDeltaEvent = WireEvent<
  'tool_args_delta',
  'delta' | 'round' | 'tool_call_id' | 'tool_name'
>;

export type ChatToolCallEvent = WireEvent<'tool_call', 'tool_name' | 'skill_id' | 'round'>;
export type ChatToolResultEvent = WireEvent<
  'tool_result',
  'tool_name' | 'skill_id' | 'output' | 'success' | 'round'
>;

// ── Sub-agents ───────────────────────────────────────────────────────────────

type SubagentBase = 'tool_name' | 'skill_id' | 'message' | 'round';
type WithDetail<T> = Omit<T, 'subagent'> & { subagent?: SubagentProgressDetail };

export type ChatSubagentSpawnedEvent = WithDetail<WireEvent<'subagent_spawned', SubagentBase>>;
export type ChatSubagentSpawnedEventV2 = ChatSubagentSpawnedEvent;
export type ChatSubagentDoneEvent = WithDetail<
  WireEvent<'subagent_completed', SubagentBase | 'success'>
>;
export type ChatSubagentIterationStartEvent = WithDetail<
  WireEvent<'subagent_iteration_start', SubagentBase>
>;
export type ChatSubagentToolCallEvent = WithDetail<
  WireEvent<'subagent_tool_call', 'tool_name' | 'skill_id' | 'round' | 'tool_call_id'>
>;
export type ChatSubagentToolResultEvent = WithDetail<
  WireEvent<'subagent_tool_result', 'tool_name' | 'skill_id' | 'round' | 'tool_call_id' | 'success'>
>;
export type ChatSubagentTextDeltaEvent = WithDetail<
  WireEvent<'subagent_text_delta', 'delta' | 'round'>
>;
export type ChatSubagentThinkingDeltaEvent = WithDetail<
  WireEvent<'subagent_thinking_delta', 'delta' | 'round'>
>;

// ── Approvals and plan review ────────────────────────────────────────────────

export type ChatApprovalRequestEvent = Omit<
  WireEvent<'approval_request', 'tool_name' | 'message'>,
  'args'
> & { args?: Record<string, unknown> };

/**
 * `approval_decided` as consumers see it. The wire carries the terminal
 * outcome in `cancel_reason` (`expired`, `cancelled`, or a decision echo);
 * {@link decodeApprovalDecided} lifts it into `resolution`.
 */
export type ChatApprovalDecidedEvent = WireEvent<'approval_decided'> & {
  resolution?: 'expired' | 'cancelled' | string;
};

export function decodeApprovalDecided(
  wire: WireEvent<'approval_decided'>
): ChatApprovalDecidedEvent {
  return { ...wire, resolution: wire.cancel_reason };
}

export type ChatPlanReviewRequestEvent = Omit<
  WireEvent<'plan_review_request', 'message'>,
  'args'
> & { args?: { steps?: string[] } };

// ── Thread state (goal, todos, run mode) ─────────────────────────────────────

/** One item of a thread's todo list (open JSON in the schema). */
export interface ChatThreadTodoItem {
  content: string;
  status: 'pending' | 'in_progress' | 'completed' | 'cancelled';
  reason?: string;
}

/** The thread's durable goal (open JSON in the schema; owned by tinyagents-graph). */
export interface ThreadGoal {
  goal_id: string;
  objective: string;
  status: 'active' | 'paused' | 'budget_limited' | 'complete';
  token_budget?: number;
  tokens_used: number;
  time_used_seconds: number;
}

export type ChatThreadTodosChangedEvent = Omit<WireEvent<'thread_todos_changed'>, 'todos'> & {
  todos: ChatThreadTodoItem[];
};
export type ChatThreadGoalUpdatedEvent = Omit<WireEvent<'thread_goal_updated'>, 'goal'> & {
  goal: ThreadGoal;
};
export type ChatThreadGoalClearedEvent = WireEvent<'thread_goal_cleared'>;
export type RunMode = 'plan' | 'build';

/** `run_mode_changed` as consumers see it; the wire carries the mode in `message`. */
export type ChatRunModeChangedEvent = Omit<WireEvent<'run_mode_changed'>, 'message'> & {
  mode: RunMode;
};

/** Lift the wire's `message` into `mode`; `null` when it is not a known mode. */
export function decodeRunModeChanged(
  wire: WireEvent<'run_mode_changed'>
): ChatRunModeChangedEvent | null {
  const { message, ...rest } = wire;
  return message === 'plan' || message === 'build' ? { ...rest, mode: message } : null;
}

// ── Artifacts ────────────────────────────────────────────────────────────────
// The artifact payload rides the wire event's open `args` JSON; chatService
// validates it and flattens it into these views.

export type ArtifactKind = 'presentation' | 'document' | 'image' | 'video' | 'other';

interface ArtifactBase {
  thread_id: string;
  client_id?: string;
  artifact_id: string;
  kind: ArtifactKind;
  title: string;
  workspace_dir: string;
  tool_call_id?: string;
  request_id?: string;
}

export interface ArtifactReadyEvent extends ArtifactBase {
  path: string;
  size_bytes: number;
}
export interface ArtifactFailedEvent extends ArtifactBase {
  error: string;
}
export interface ArtifactPendingEvent extends ArtifactBase {
  path: string;
}

// ── Queue and suggestions ────────────────────────────────────────────────────

/** These two ride outside a request, so `request_id` may be empty or absent. */
type OutOfRequest<T extends { request_id: string }> = Omit<T, 'request_id'> & {
  request_id?: string;
};
export type QueueItemEvent = OutOfRequest<WireEvent<'queue_item_queued'>>;
export type ChatSuggestionsEvent = OutOfRequest<WireEvent<'chat_suggestions'>>;
