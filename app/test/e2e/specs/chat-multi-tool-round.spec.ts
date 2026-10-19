// @ts-nocheck
/**
 * Chat multi-tool round — agent uses two tools in sequence.
 *
 * Exercises a three-turn LLM loop:
 *   Turn 1: tool_call → file_read
 *   Turn 2: tool_call → grep
 *   Turn 3: final answer with canary text
 *
 * Verifies:
 *   T2.1 — first tool (file_read) appears in the timeline
 *   T2.2 — second tool (grep) also appears; timeline has 2 entries
 *   T2.3 — final answer renders after both tools complete
 *   T2.5 — tool timeline has 2 entries in correct order (file_read before grep)
 */
import { waitForApp } from '../helpers/app-helpers';
import {
  chatMounted,
  clickByTitle,
  clickSend,
  getSelectedThreadId,
  typeIntoComposer,
  waitForSocketConnected,
} from '../helpers/chat-harness';
import { textExists } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import { clearRequestLog, setMockBehavior, startMockServer, stopMockServer } from '../mock-server';

const LOG_PREFIX = '[chat-multi-tool-round]';
const USER_ID = 'e2e-chat-multi-tool-round';
const PROMPT = 'Read the config file and search for the relevant setting.';
const CANARY_FINAL = 'canary-multi-tool-d4e5f6';

// Three forced responses: tool 1, tool 2, final answer.
const FORCED_RESPONSES = [
  {
    content: '',
    toolCalls: [
      {
        id: 'call_file_read_1',
        name: 'file_read',
        arguments: JSON.stringify({ path: '/etc/openhuman/config.toml' }),
      },
    ],
  },
  {
    content: '',
    toolCalls: [
      {
        id: 'call_grep_1',
        name: 'grep',
        arguments: JSON.stringify({ pattern: 'relevant_setting', path: '/etc/openhuman' }),
      },
    ],
  },
  { content: `Found the content using both tools: ${CANARY_FINAL}` },
];

async function renderedToolCards(): Promise<string[]> {
  return (await browser.execute(() =>
    Array.from(document.querySelectorAll<HTMLElement>('[data-testid="assistant-ui-tool-call"]')).map(
      card => card.innerText
    )
  )) as string[];
}

describe('Chat multi-tool round', () => {
  before(function () {
    this.skip(); // https://github.com/senamakel/openhuman/issues/236 — the Playwright twin owns the current web_fetch/web_search UI flow pending runner assignment.
  });
  let threadId: string;

  before(async () => {
    console.log(`${LOG_PREFIX} Starting mock server and resetting app`);
    await startMockServer();
    await waitForApp();
    await resetApp(USER_ID);

    setMockBehavior('llmForcedResponses', JSON.stringify(FORCED_RESPONSES));
    setMockBehavior('llmStreamChunkDelayMs', '10');
    clearRequestLog();
    console.log(`${LOG_PREFIX} Setup complete — 3 forced responses configured`);
  });

  after(async () => {
    setMockBehavior('llmForcedResponses', '');
    setMockBehavior('llmStreamChunkDelayMs', '');
    await stopMockServer();
    console.log(`${LOG_PREFIX} Teardown complete`);
  });

  it('T2.1 — agent calls tool 1 (file_read); rendered card shows it', async () => {
    console.log(`${LOG_PREFIX} T2.1: navigating to /chat, opening new thread`);
    await navigateViaHash('/chat');
    await browser.waitUntil(async () => await chatMounted(), {
      timeout: 15_000,
      timeoutMsg: 'Conversations panel did not mount',
    });
    expect(await clickByTitle('New thread', 8_000)).toBe(true);

    threadId = (await browser.waitUntil(async () => await getSelectedThreadId(), {
      timeout: 8_000,
      timeoutMsg: 'thread.selectedThreadId never populated',
    })) as string;
    expect(typeof threadId).toBe('string');
    console.log(`${LOG_PREFIX} T2.1: thread created: ${threadId}`);

    await typeIntoComposer(PROMPT);
    const socketReady = await waitForSocketConnected(30_000);
    if (!socketReady) {
      console.warn('[chat-multi-tool-round] socket did not connect within 30 s — send may fail');
    }
    expect(
      await browser.waitUntil(async () => await clickSend(), {
        timeout: 5_000,
        timeoutMsg: 'Send button never enabled',
      })
    ).toBe(true);

    await browser.waitUntil(
      async () => (await renderedToolCards()).some(card => card.includes('Read file')),
      { timeout: 45_000, interval: 100, timeoutMsg: 'Rendered file_read tool card never appeared' }
    );
    expect(await textExists('Read file')).toBe(true);
    console.log(`${LOG_PREFIX} T2.1: passed`);
  });

  it('T2.2 — agent calls tool 2 (grep); timeline shows 2 entries', async () => {
    console.log(`${LOG_PREFIX} T2.2: watching for grep in timeline`);
    await browser.waitUntil(
      async () => (await renderedToolCards()).some(card => /Search(?:ing|ed) code/.test(card)),
      { timeout: 45_000, interval: 100, timeoutMsg: 'Rendered grep tool card never appeared' }
    );
    expect((await renderedToolCards()).some(card => /Search(?:ing|ed) code/.test(card))).toBe(true);
  });

  it('T2.3 — final answer renders after both tools complete', async () => {
    console.log(`${LOG_PREFIX} T2.3: waiting for canary text`);
    await browser.waitUntil(async () => await textExists(CANARY_FINAL), {
      timeout: 50_000,
      timeoutMsg: `final answer "${CANARY_FINAL}" never rendered after multi-tool round`,
    });
    console.log(`${LOG_PREFIX} T2.3: passed — canary visible`);
  });

  it('T2.5 — tool timeline has 2 entries (file_read before grep)', async () => {
    console.log(`${LOG_PREFIX} T2.5: verifying timeline order`);

    const cards = await renderedToolCards();
    const fileReadIndex = cards.findIndex(card => card.includes('Read file'));
    const grepIndex = cards.findIndex(card => /Search(?:ing|ed) code/.test(card));
    expect(fileReadIndex).toBeGreaterThanOrEqual(0);
    expect(grepIndex).toBeGreaterThan(fileReadIndex);
    console.log(`${LOG_PREFIX} T2.5: passed`);
  });
});
