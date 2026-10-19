// @ts-nocheck
/**
 * E2E test: Voice mode integration
 *
 * Current desktop flow:
 *   - Chat defaults to the text composer.
 *   - The microphone button switches the composer into `MicComposer`.
 *   - `MicComposer` exposes a "Switch to text" control to restore the text
 *     textarea.
 *
 * The older "Text / Voice" segmented toggle no longer exists. This spec
 * covers the current desktop-only voice entry surface and keeps the
 * `openhuman.voice_status` RPC contract assertions below.
 */
import { waitForApp, waitForAppReady } from '../helpers/app-helpers';
import { callOpenhumanRpc } from '../helpers/core-rpc';
import { triggerAuthDeepLink } from '../helpers/deep-link-helpers';
import {
  clickButton,
  dumpAccessibilityTree,
  textExists,
  waitForWebView,
  waitForWindowVisible,
} from '../helpers/element-helpers';
import { supportsExecuteScript } from '../helpers/platform';
import { completeOnboardingIfVisible } from '../helpers/shared-flows';
import { clearRequestLog, getRequestLog, startMockServer, stopMockServer } from '../mock-server';

async function waitForRequest(method, urlFragment, timeout = 15_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const log = getRequestLog();
    const match = log.find(r => r.method === method && r.url.includes(urlFragment));
    if (match) return match;
    await browser.pause(500);
  }
  return undefined;
}

async function waitForHome(timeout = 20_000) {
  // Home.tsx renders t('home.askAssistant') = 'Ask your assistant anything...' as stable CTA.
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await textExists('Ask your assistant anything')) return true;
    if (await textExists('Your device is connected')) return true;
    await browser.pause(700);
  }
  return false;
}

async function waitForAnyText(candidates, timeout = 20_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    for (const t of candidates) {
      if (await textExists(t)) return t;
    }
    await browser.pause(600);
  }
  return null;
}

// Both cases in this suite have current Playwright equivalents:
// `chat mic button switches into MicComposer and can return to text mode` and
// the `switchChatIntoMicComposer` control checks for permission/device cases.
// Issue #236 decides which runner owns these duplicate SPA-only assertions.
describe.skip(// https://github.com/senamakel/openhuman/issues/236
'Voice mode integration', () => {
  before(async () => {
    await startMockServer();
    await waitForApp();
    clearRequestLog();
  });

  after(async () => {
    await stopMockServer();
  });

  it('can switch into the mic composer and back to text mode', async () => {
    await triggerAuthDeepLink('e2e-voice-token');
    await waitForWindowVisible(25_000);
    await waitForWebView(15_000);
    await waitForAppReady(15_000);

    const consume = await waitForRequest('POST', '/auth/login-token/consume');
    expect(consume).toBeDefined();

    await completeOnboardingIfVisible('[VoiceModeE2E]');

    const onHome = await waitForHome(20_000);
    if (!onHome) {
      const tree = await dumpAccessibilityTree();
      console.log('[VoiceModeE2E] Home not reached. Tree:\n', tree.slice(0, 4000));
    }
    expect(onHome).toBe(true);

    const hasTextInput = await waitForAnyText(['How can I help', 'Threads', 'New'], 10_000);
    expect(hasTextInput).not.toBeNull();

    await clickButton('Voice mode', 10_000);

    const voiceStatusMessage = await waitForAnyText(
      [
        'Tap and speak',
        'Tap to send',
        'Speech-to-text unavailable',
        'whisper-cli binary',
        'Ready',
        'Start Talking',
        'Voice input needs a speech model',
      ],
      15_000
    );
    expect(voiceStatusMessage).not.toBeNull();

    await clickButton('Switch to text', 10_000);
    const textRestored = await waitForAnyText(['How can I help', 'Threads', 'New'], 10_000);
    expect(textRestored).not.toBeNull();
  });

  it('surfaces a mic entry button from the text composer', async () => {
    const onConversations = await waitForAnyText(['How can I help', 'Threads', 'New'], 10_000);
    if (!onConversations) {
      const tree = await dumpAccessibilityTree();
      console.log('[VoiceModeE2E] Conversations not ready. Tree:\n', tree.slice(0, 4000));
    }
    expect(onConversations).not.toBeNull();
    expect(await textExists('Voice mode')).toBe(true);
  });
});

/**
 * Hosted STT engine — core RPC contract tests.
 *
 * These tests exercise the `openhuman.voice_status` RPC to assert the
 * availability contract without touching the UI voice toggle (which was
 * removed in #717). The RPC contract is:
 *
 *   - `stt_engine` is the configured backend or third-party provider route.
 *   - `stt_available` reports whether that route can be constructed from the
 *     active configuration, with `stt_error` explaining a missing provider.
 */
describe('Voice mode — hosted STT contract (voice_status RPC)', () => {
  before(async () => {
    await startMockServer();
    await waitForApp();
  });

  after(async () => {
    await stopMockServer();
  });

  it('5.1 — voice_status RPC returns a well-formed response', async () => {
    const result = await callOpenhumanRpc('openhuman.voice_status', {});
    expect(result).toBeDefined();
    expect(typeof result).toBe('object');
    const status = (result as any).result ?? result;
    expect(typeof status.stt_available).toBe('boolean');
    expect(typeof status.tts_available).toBe('boolean');
    expect(typeof status.stt_engine).toBe('string');
    expect(status.stt_error === null || typeof status.stt_error === 'string').toBe(true);
  });

  it('5.2 — voice_status reports the resolved STT engine', async () => {
    const result = await callOpenhumanRpc('openhuman.voice_status', {});
    const status = (result as any).result ?? result;

    expect(status.stt_engine.length).toBeGreaterThan(0);
    expect(status.stt_error === null || typeof status.stt_error === 'string').toBe(true);
  });
});

/**
 * Remaining desktop voice coverage: the shipped mic surface on the current
 * chat route, plus the unsupported spoken-audio round-trip. The synthetic
 * `voice_stt_dispatch` call moved to the JSON-RPC integration layer;
 * permission/device and beep-placeholder browser API flows live in the
 * Playwright voice-mode spec.
 */
// This suite is no longer blanket-skipped: its current mic UI assertion runs,
// while only the real-audio injection case remains blocked by #435.
describe('Voice mode — Human tab capture & error mapping', () => {
  before(async () => {
    await startMockServer();
    await waitForApp();
  });

  after(async () => {
    await stopMockServer();
  });

  // ---------------------------------------------------------------------------
  // Helper: navigate to the Assistant (Chat) tab via hash routing.
  // /human merged into /chat: the mascot docks on the chat composer, and
  // /human is kept only as a back-compat redirect.
  // ---------------------------------------------------------------------------
  async function navigateToHumanTab(): Promise<void> {
    if (supportsExecuteScript()) {
      await browser.execute(() => {
        // /human → /chat (back-compat redirect preserved)
        window.location.hash = '#/chat';
      });
    } else {
      // Mac2 path: use the shared helper which abstracts the XCUIElementTypeButton
      // XPath so the selector stays cross-driver and policy-compliant.
      await clickButton('Assistant');
    }
    // Allow React router to settle and the Chat page to mount.
    await browser.pause(1_500);
  }

  // ---------------------------------------------------------------------------
  // 6.1 — Human tab renders with MicComposer in idle state.
  //
  // Checks that the Human tab mounts the MicComposer idle button (aria-label="Start recording"
  // / visible label "Tap and speak") is present.
  // ---------------------------------------------------------------------------
  it('6.1 — Human tab renders with MicComposer in idle state', async () => {
    await triggerAuthDeepLink('e2e-voice-human-tab-token');
    await waitForWindowVisible(25_000);
    await waitForWebView(15_000);
    await waitForAppReady(15_000);
    await completeOnboardingIfVisible('[HumanTabE2E]');

    await navigateToHumanTab();

    // /human is a compatibility redirect into the current chat composer. Pin
    // the controls users can invoke there rather than the removed setup label.
    expect(await textExists('Voice mode')).toBe(true);
    await clickButton('Voice mode');
    expect(await textExists('Switch to text')).toBe(true);
    expect(await waitForAnyText(['Tap and speak', 'Waiting for agent'], 10_000)).not.toBeNull();
  });

  // 6.2 was an RPC-only call with a synthetic payload. The owner-layer JSON-RPC
  // integration test now pins dispatch and structured invalid-audio handling.

  // The permission-denied and missing-device outcomes are asserted by the
  // current Playwright voice-mode UI cases; beep-placeholder behavior has its
  // own Playwright regression. Those browser-API copies were removed here so
  // this desktop suite only keeps its current native surface and the
  // unsupported real-audio round-trip below.

  // ---------------------------------------------------------------------------
  // 6.6 — Actual transcription round-trip with mocked audio device (SKIPPED).
  //
  // A full round-trip — speak → MediaRecorder captures audio → STT RPC →
  // transcript appears as user message → agent replies — requires:
  //   a) A real or virtual audio device (unavailable in headless docker).
  //   b) The ability to inject PCM frames into MediaRecorder (not possible
  //      via WebDriver — WDIO has no W3C audio injection API for CEF).
  //   c) The mock server to handle /openai/v1/audio/transcriptions (it does).
  //
  // The contract is tested at the RPC layer in test 6.2 (voice_stt_dispatch)
  // and at the unit level in MicComposer.test.tsx (transcribeWithFactory mock).
  // This skip records the gap: there is no E2E path that drives real audio
  // through MediaRecorder in the CI headless environment.
  //
  // Tracked: issue #435. Remove skip when the test harness supports audio
  // injection (e.g. via a virtual ALSA device + pre-recorded WAV replayer, or
  // a fake getUserMedia implementation that returns a pre-seeded MediaStream).
  // ---------------------------------------------------------------------------
  it.skip(// https://github.com/senamakel/openhuman/issues/435
  '6.6 — spoken prompt round-trip: mic → STT → user bubble → agent reply (requires real/virtual audio device)', async () => {
    // When unblocked:
    //   1. Navigate to /human.
    //   2. Inject a pre-recorded WAV as a fake MediaStream via getUserMedia mock
    //      (or use a virtual ALSA loopback device seeded by the test harness).
    //   3. Click "Start recording", let the recorder run for ~500 ms, click again.
    //   4. Wait for the STT RPC to complete (mock returns known transcript text).
    //   5. Assert the known transcript text appears as a user bubble in the thread.
    //   6. Assert the agent responds (at minimum: a non-empty agent message bubble).
    //   7. Assert the user bubble does not contain "beep" or other placeholder text.
  });
});
