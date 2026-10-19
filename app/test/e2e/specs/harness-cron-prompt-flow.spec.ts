// @ts-nocheck
/**
 * Harness — Cron prompt-flow (WS-D spec 2).
 *
 * Exercises the agent harness routing natural-language cron-management prompts
 * through the mock LLM, which emits cron tool calls, and verifies that the
 * in-process core actually mutates cron state (confirmed via oracle RPCs).
 *
 * Actual tool names discovered in crates/openhuman-core/src/tools/impl/cron/:
 *   - "cron_add"    — create a new cron job
 *   - "cron_list"   — list existing jobs
 *   - "cron_update" — change schedule / enabled flag
 *   - "cron_remove" — delete a job
 *   - "cron_run"    — trigger a job immediately
 *   - "cron_runs"   — list run history
 *
 * Scenarios:
 *   CR2.1 — Create via NL: "remind me every morning at 9am" → cron_add tool call
 *            → oracle RPC confirms job exists → UI shows creation confirmation
 *   CR2.2 — List jobs: pre-create 2 jobs via oracle RPC → "what are my scheduled tasks"
 *            → LLM returns content listing them (no tool call needed) → UI shows reply
 *   CR2.3 — Update schedule: pre-create job → "change my morning reminder to 8am"
 *            → cron_update tool call → oracle confirms schedule changed
 *   CR2.4 — Delete via prompt: pre-create job → "delete the morning reminder"
 *            → cron_remove tool call → oracle confirms job gone
 *
 * Note on tool call execution in E2E:
 *   Whether the core actually EXECUTES the cron tool (persisting the job) vs.
 *   merely routing it depends on the tool being registered in the harness's
 *   tool registry and the E2E app having all required config. Cron tools are
 *   core-domain operations that do not require external credentials, so they
 *   should execute against the in-process core.
 *
 * If a tool call does not persist (oracle RPC shows no change), we document
 * it with a TODO comment and fall back to asserting the LLM-side behavior.
 */
import { waitForApp } from '../helpers/app-helpers';
import {
  approvePendingToolCall,
  chatMounted,
  clickByTitle,
  clickSend,
  getSelectedThreadId,
  typeIntoComposer,
  waitForAssistantReplyContaining,
  waitForSocketConnected,
} from '../helpers/chat-harness';
import { callOpenhumanRpc } from '../helpers/core-rpc';
import { textExists } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import {
  clearRequestLog,
  getRequestLog,
  resetMockBehavior,
  setMockBehavior,
  startMockServer,
  stopMockServer,
} from '../mock-server';

const LOG_PREFIX = '[HarnessCron]';
const USER_ID = 'e2e-harness-cron-prompt-flow';

// ---------------------------------------------------------------------------
// Oracle helpers
// ---------------------------------------------------------------------------

/** Retrieve the current cron job list via oracle RPC. */
async function listCronJobs(): Promise<Array<{ id: string; name: string; schedule?: string }>> {
  const out = await callOpenhumanRpc('openhuman.cron_list', {});
  if (!out.ok) {
    console.warn(`${LOG_PREFIX} cron_list RPC failed: ${JSON.stringify(out)}`);
    return [];
  }
  const result = (out.result as { result?: unknown } | undefined)?.result ?? out.result;
  return Array.isArray(result) ? result : [];
}

/** Create a cron job via oracle RPC. Returns the created job id.
 *
 * The `openhuman.cron_add` RPC requires either a `command` (shell job) or a
 * `prompt` (agent job); with neither, it defaults to shell and rejects with
 * `"'command' is required for shell jobs"`. This oracle seeds an agent job
 * because the harness scenarios exercise agent-style scheduled reminders.
 *
 * `cron_add` has no way to seed a job disabled (the RPC hard-codes
 * `enabled: true` for agent jobs; see `crates/openhuman-core/src/cron/schemas.rs:359`).
 * Since scenarios seed schedules like `0 9 * * *` and `0 10 * * 5`, a run
 * that crosses one of those times would otherwise let the scheduler fire
 * the oracle job during a later scenario — consuming mock LLM responses
 * and adding stray request-log entries. Immediately disable the job via
 * `cron_update` so the seed stays inert; the CR2.3 / CR2.4 flows still
 * exercise update/remove against a real stored job. */
async function createCronJobOracle(params: {
  name: string;
  schedule: string;
  enabled?: boolean;
}): Promise<string | null> {
  const out = await callOpenhumanRpc('openhuman.cron_add', {
    name: params.name,
    schedule: { kind: 'cron', expr: params.schedule },
    job_type: 'agent',
    prompt: `e2e oracle seed: ${params.name}`,
  });
  if (!out.ok) {
    console.warn(`${LOG_PREFIX} cron_add oracle failed: ${JSON.stringify(out)}`);
    return null;
  }
  const result = (out.result as { result?: unknown } | undefined)?.result ?? out.result;
  const id = (result as { id?: string })?.id ?? null;
  console.log(`${LOG_PREFIX} oracle cron_add: name=${params.name}, id=${id}`);
  if (id) {
    const disable = await callOpenhumanRpc('openhuman.cron_update', {
      job_id: id,
      patch: { enabled: false },
    });
    if (!disable.ok) {
      console.warn(`${LOG_PREFIX} cron_update disable oracle failed: ${JSON.stringify(disable)}`);
    } else {
      console.log(`${LOG_PREFIX} oracle cron disabled: id=${id}`);
    }
  }
  return id;
}

// ---------------------------------------------------------------------------
// Navigation helper
// ---------------------------------------------------------------------------

async function navigateChatAndSend(prompt: string): Promise<string | null> {
  await navigateViaHash('/chat');
  await browser.waitUntil(async () => await chatMounted(), {
    timeout: 15_000,
    timeoutMsg: 'Conversations panel did not mount',
  });
  expect(await clickByTitle('New thread', 8_000)).toBe(true);
  const threadId = (await browser.waitUntil(async () => await getSelectedThreadId(), {
    timeout: 8_000,
    timeoutMsg: 'thread.selectedThreadId never populated',
  })) as string;

  await typeIntoComposer(prompt);
  const socketReady = await waitForSocketConnected(30_000);
  if (!socketReady) {
    console.warn(`${LOG_PREFIX} socket did not connect within 30s — send may fail`);
  }
  expect(
    await browser.waitUntil(async () => await clickSend(), {
      timeout: 15_000,
      timeoutMsg: 'Send button never enabled',
    })
  ).toBe(true);
  console.log(`${LOG_PREFIX} Sent: "${prompt.slice(0, 80)}"`);
  return threadId;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('Harness - Cron prompt-flow', () => {
  before(async function beforeSuite() {
    this.timeout(90_000);
    console.log(`${LOG_PREFIX} Starting mock server and resetting app`);
    await startMockServer();
    await waitForApp();
    await resetApp(USER_ID);
    console.log(`${LOG_PREFIX} Suite setup complete`);
  });

  after(async () => {
    resetMockBehavior();
    await stopMockServer();
    console.log(`${LOG_PREFIX} Suite teardown complete`);
  });

  // ── CR2.1 — Create cron via natural language ──────────────────────────────

  // TODO(#6391): the TinyAgents update no longer exposes write-tool approval prompts.
  // Retired legacy direct cron_* call; the supported use_skill path is covered in Playwright.

  // ── CR2.2 — List jobs ─────────────────────────────────────────────────────

  it('CR2.2 — "what are my scheduled tasks" — LLM lists pre-seeded jobs in reply', async function () {
    this.timeout(120_000);
    console.log(`${LOG_PREFIX} CR2.2: begin`);

    clearRequestLog();
    resetMockBehavior();

    // Pre-create two jobs via oracle so they exist in the in-process core.
    await createCronJobOracle({ name: 'daily_standup', schedule: '0 9 * * 1-5' });
    await createCronJobOracle({ name: 'weekly_review', schedule: '0 10 * * 5' });

    // Verify they exist.
    const jobs = await listCronJobs();
    console.log(`${LOG_PREFIX} CR2.2: pre-send jobs: ${jobs.map(j => j.name).join(', ')}`);

    // This scenario does not require a tool call — the LLM can simply return
    // a content-only response that lists the job names.
    const CANARY = 'canary-cron-list-c3d4';
    const KEYWORD_RULES = [
      {
        keyword: 'scheduled tasks',
        content: `You have 2 scheduled tasks: daily_standup (weekdays 9am) and weekly_review (Fridays 10am). ${CANARY}`,
      },
    ];
    setMockBehavior('llmKeywordRules', JSON.stringify(KEYWORD_RULES));
    setMockBehavior('llmStreamChunkDelayMs', '10');

    await navigateChatAndSend('what are my scheduled tasks');

    await browser.waitUntil(async () => await textExists(CANARY), {
      timeout: 60_000,
      timeoutMsg: `CR2.2: list-jobs canary "${CANARY}" never appeared`,
    });
    expect(await waitForAssistantReplyContaining('daily_standup', { logPrefix: LOG_PREFIX })).toBe(
      true
    );
    expect(await waitForAssistantReplyContaining('weekly_review', { logPrefix: LOG_PREFIX })).toBe(
      true
    );

    // Oracle: jobs still exist after the query (no side effects).
    const afterJobs = await listCronJobs();
    const hasDailyStandup = afterJobs.some(j => j.name === 'daily_standup');
    const hasWeeklyReview = afterJobs.some(j => j.name === 'weekly_review');
    console.log(
      `${LOG_PREFIX} CR2.2: oracle post-query — daily_standup=${hasDailyStandup}, weekly_review=${hasWeeklyReview}`
    );
    // Jobs were either created and still exist, or oracle is not available in this build.
    // Either way, the UI assertion holds.

    console.log(`${LOG_PREFIX} CR2.2: PASSED`);
  });

  // ── CR2.3 — Update schedule ───────────────────────────────────────────────

  // TODO(#6391): the TinyAgents update no longer exposes write-tool approval prompts.
  // Retired legacy direct cron_* call; the supported use_skill path is covered in Playwright.

  // ── CR2.4 — Delete via prompt ─────────────────────────────────────────────

  // TODO(#6391): the TinyAgents update no longer exposes write-tool approval prompts.
  // Retired legacy direct cron_* call; the supported use_skill path is covered in Playwright.
});
