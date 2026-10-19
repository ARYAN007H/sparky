// @ts-nocheck
/**
 * E2E spec: Interactive guided tour — gates and resume behaviour (#1215).
 *
 * Three scenarios are exercised:
 *
 *   1. Skills gate: start tour, reach the skills step, confirm skills UI is
 *      present. The tooltip advances via Next — the current implementation
 *      navigates to /connections (was /skills) and highlights the grid via a `before` async hook
 *      in walkthroughSteps.ts. The test polls for the hash change rather than
 *      reading it immediately, because the Joyride `before` hook is awaited
 *      asynchronously and the hash may lag by a render cycle.
 *      Skill-connection gating is NOT implemented; that assertion is skipped
 *      and the gap is called out explicitly (GP-1).
 *
 *   2. Chat gate: the final (7th) step has a `before` hook that creates a
 *      thread and seeds a welcome message, then navigates to /chat. Reaching
 *      step 7 by clicking Next 6 times is inherently fragile in CI (any one
 *      before-hook timeout aborts the sequence). The multi-step-advance test
 *      is therefore skipped (GP-3: no shortcut to jump to an arbitrary step),
 *      and replaced by two fast, independent assertions:
 *        a) The data-walkthrough="chat-agent-panel" target exists on /chat.
 *        b) The Skip button is absent on the last Joyride step (verified by
 *           WalkthroughTooltip rendering `!isLastStep && <skip>` — tested by
 *           unit tests, not duplicated here).
 *      Sending-a-message gating is NOT implemented; skipped with GP-1 comment.
 *
 *   3. Resume after reload: set walkthrough pending flag, reload the renderer
 *      without clearing localStorage, and assert the tour auto-starts. The
 *      AppWalkthrough component reads `isWalkthroughPending()` on mount and
 *      sets `run=true`, so the tooltip should appear after reload. True
 *      mid-step resume (restoring last step index) is NOT implemented; that
 *      assertion is skipped and documented as GP-2.
 *
 * Product gaps surfaced (skipped):
 *   - GP-1: No skill-connection gate on the /connections tour step (Phase 2: was /skills).
 *   - GP-2: No step-index persistence — tour always restarts from step 0
 *           on reload rather than resuming at the last incomplete step.
 *   - GP-3: No API to jump to an arbitrary Joyride step — the only way to
 *           reach step N is to click Next N-1 times, which is fragile in CI.
 *
 * Implementation notes:
 *   - The walkthrough is driven by manipulating localStorage keys directly
 *     (`openhuman:walkthrough_pending`, `openhuman:walkthrough_completed`)
 *     rather than walking the full onboarding flow, because (a) resetApp
 *     already handles onboarding and (b) the Joyride component reads these
 *     keys on mount.
 *   - `data-walkthrough` attributes are queried to verify step targets are
 *     present without coupling to tooltip text that may be i18n-translated.
 *   - The spec uses `supportsExecuteScript()` guards so it degrades
 *     gracefully on Appium Mac2 (where `browser.execute` is unavailable in
 *     a WKWebView context).
 */
import { waitForApp } from '../helpers/app-helpers';
import { textExists } from '../helpers/element-helpers';
import { supportsExecuteScript } from '../helpers/platform';
import { resetApp } from '../helpers/reset-app';
import { dismissWalkthroughIfVisible, navigateViaHash } from '../helpers/shared-flows';
import { startMockServer, stopMockServer } from '../mock-server';

const USER_ID = 'e2e-guided-tour-gates';

// localStorage keys mirrored from AppWalkthrough.tsx
const WALKTHROUGH_KEY = 'openhuman:walkthrough_completed';
const WALKTHROUGH_PENDING_KEY = 'openhuman:walkthrough_pending';

// ── helpers ──────────────────────────────────────────────────────────────────

/**
 * Arm the walkthrough: clear the completed flag, set the pending flag.
 * Equivalent to what resetWalkthrough() does in production code.
 * Returns false when execute() is unavailable (Mac2).
 */
async function armWalkthrough(): Promise<boolean> {
  if (!supportsExecuteScript()) return false;
  await browser.execute(
    ({ pendingKey, completedKey }: { pendingKey: string; completedKey: string }) => {
      try {
        localStorage.removeItem(completedKey);
        localStorage.setItem(pendingKey, 'true');
      } catch (_) {
        // swallow — mirrors AppWalkthrough try/catch
      }
    },
    { pendingKey: WALKTHROUGH_PENDING_KEY, completedKey: WALKTHROUGH_KEY }
  );
  return true;
}

/**
 * Mark walkthrough complete in localStorage so subsequent specs start clean.
 */
async function disarmWalkthrough(): Promise<void> {
  if (!supportsExecuteScript()) return;
  await browser.execute(
    ({ completedKey, pendingKey }: { completedKey: string; pendingKey: string }) => {
      try {
        localStorage.setItem(completedKey, 'true');
        localStorage.removeItem(pendingKey);
      } catch (_) {
        // ignore
      }
    },
    { completedKey: WALKTHROUGH_KEY, pendingKey: WALKTHROUGH_PENDING_KEY }
  );
}

/**
 * Wait up to `timeout` ms for the Joyride dialog overlay to be visible.
 * WalkthroughTooltip is an accessible dialog, rendered with `role="alertdialog"`.
 */
async function waitForTourTooltip(timeout = 15_000): Promise<boolean> {
  if (!supportsExecuteScript()) return false;
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const visible = await browser.execute(() => {
      return document.querySelector('[role="alertdialog"]') !== null;
    });
    if (visible) return true;
    await browser.pause(400);
  }
  return false;
}

/**
 * Advance the tour by clicking the primary (Next/Let's go) button inside
 * the tooltip overlay. Returns true if the click landed, false if no button
 * was found within `timeout`.
 */
async function clickTourNext(timeout = 8_000): Promise<boolean> {
  if (!supportsExecuteScript()) return false;
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const clicked = await browser.execute(() => {
      const tooltip = document.querySelector('[role="alertdialog"]');
      if (!tooltip) return false;
      // Primary button carries data-action="primary" (set by Joyride on primaryProps)
      const primary = tooltip.querySelector<HTMLButtonElement>('[data-action="primary"]');
      if (!primary) return false;
      primary.click();
      return true;
    });
    if (clicked) return true;
    await browser.pause(300);
  }
  return false;
}

/**
 * Advance the tour N times, pausing between clicks to let the `before` hook
 * complete and the DOM settle. Uses a longer inter-step pause (2 s) so async
 * before hooks (navigate + waitForTarget) finish before the next click.
 */
async function advanceTourSteps(count: number): Promise<void> {
  for (let i = 0; i < count; i++) {
    const clicked = await clickTourNext(8_000);
    if (!clicked) {
      console.warn(`[guided-tour-gates] clickTourNext: no primary button on advance ${i + 1}`);
      break;
    }
    // Allow the before() hook to navigate and the DOM to settle. 2 s is generous
    // enough for the HashRouter to update and waitForTarget to resolve.
    await browser.pause(2_000);
  }
}

// ── suite ─────────────────────────────────────────────────────────────────────

describe('Guided tour — gates and resume behaviour (#1215)', function () {
  this.timeout(180_000);

  before(async () => {
    await startMockServer();
    await waitForApp();
    await resetApp(USER_ID);
  });

  afterEach(async () => {
    // Always disarm so the next scenario starts clean.
    await disarmWalkthrough();
    await dismissWalkthroughIfVisible(4_000);
  });

  after(async () => {
    await stopMockServer();
  });

  // ── Scenario 1: Start and navigate ─────────────────────────────────────────

  describe('Scenario 1 — start and navigate', () => {
    it('tour starts on the current chat panel at step 1 of 7', async () => {
      await navigateViaHash('/home');
      expect(await armWalkthrough()).toBe(true);
      await browser.refresh();
      expect(await waitForTourTooltip()).toBe(true);
      expect(await textExists('1 of 7')).toBe(true);
      expect(
        await browser.execute(
          () => document.querySelector('[data-walkthrough="chat-agent-panel"]') !== null
        )
      ).toBe(true);
    });

    it('tour navigates to /connections and highlights skills-grid after 1 Next click', async () => {
      await navigateViaHash('/home');
      expect(await armWalkthrough()).toBe(true);
      await browser.refresh();
      expect(await waitForTourTooltip()).toBe(true);
      await advanceTourSteps(1);
      await browser.waitUntil(
        async () =>
          String(await browser.execute(() => window.location.hash)).includes('/connections'),
        { timeout: 15_000, timeoutMsg: 'tour did not navigate to /connections after one step' }
      );
      expect(
        await browser.execute(
          () => document.querySelector('[data-walkthrough="skills-grid"]') !== null
        )
      ).toBe(true);
    });
  });

  // ── Scenario 2: Chat gate (final step) ────────────────────────────────────

  describe('Scenario 2 — chat gate (first message)', () => {
    it('final tour step renders on /chat with a pre-seeded welcome note', async () => {
      // Drive the visible controls through the actual tour instead of seeking
      // into Joyride's internal step state.
      //   - Navigate to /home and arm walkthrough.
      //   - Advance through the earlier steps.
      //   - "You're all set!" title appears in tooltip.
      //   - Skip button is absent on the last step.
      //
      await navigateViaHash('/home');
      expect(await armWalkthrough()).toBe(true);
      await browser.refresh();
      expect(await waitForTourTooltip(10_000)).toBe(true);
      await advanceTourSteps(6);

      const hasLastStepTitle = await textExists("You're all set!");
      expect(hasLastStepTitle).toBe(true);

      const skipVisible = await browser.execute(() => {
        const tooltip = document.querySelector('[role="alertdialog"]');
        if (!tooltip) return false;
        const skip = tooltip.querySelector<HTMLButtonElement>('[data-action="skip"]');
        return skip !== null && !skip.hidden;
      });
      expect(skipVisible).toBe(false);
    });

    it('chat panel target element is present when on /chat route', async () => {
      if (!supportsExecuteScript()) {
        console.log('[guided-tour-gates] skipping: execute() unsupported on this driver');
        return;
      }

      // Navigate directly to /chat and verify the data-walkthrough target that
      // Joyride must spotlight on steps 1 and 7 is present in the DOM.
      // This is independent of the full tour advance sequence.
      await navigateViaHash('/chat');

      // Hash navigation settles before the chat runtime has finished mounting
      // its composer footer. Poll for the target so this proves the rendered
      // route contract instead of racing that asynchronous mount in CI.
      const chatPanel = await browser.waitUntil(
        async () =>
          (await browser.execute(
            () => document.querySelector('[data-walkthrough="chat-agent-panel"]') !== null
          )) as boolean,
        {
          timeout: 15_000,
          interval: 250,
          timeoutMsg: 'chat walkthrough target did not mount after navigating to /chat',
        }
      );
      // The data-walkthrough attribute must exist for Joyride to focus the step.
      expect(chatPanel).toBe(true);
    });
  });

  // ── Scenario 3: Resume after relaunch ─────────────────────────────────────

  describe('Scenario 3 — resume after relaunch (close + reopen)', () => {
    it('walkthrough re-shows after renderer reload when pending flag is set', async () => {
      await navigateViaHash('/home');
      expect(await armWalkthrough()).toBe(true);
      await browser.refresh();
      expect(await waitForTourTooltip(20_000)).toBe(true);
    });
  });
});
