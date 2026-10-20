/**
 * End-to-end: client-side Composio trigger toggles (PR for backend #671).
 *
 * Opens the Composio connection modal against the shared mock backend and
 * checks that available and active triggers render in the
 * Composio connection modal and asserts the Triggers section renders
 * the expected toggle for an ACTIVE Gmail connection.
 *
 * The mock backend (`scripts/mock-api-core.mjs`) seeds:
 *   - one ACTIVE Gmail connection (`c1`)
 *   - one available trigger (`GMAIL_NEW_GMAIL_MESSAGE`)
 *   - an empty active-trigger list that mutates as enable/disable run
 *
 * Trigger list and mutation behavior belongs to Composio Rust tests. This spec
 * owns the rendered Triggers section.
 */
import { waitForApp } from '../helpers/app-helpers';
import { textExists, waitForText } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateToSkills } from '../helpers/shared-flows';
import { clearRequestLog, setMockBehavior, startMockServer, stopMockServer } from '../mock-server';

describe('Composio trigger section UI', () => {
  before(async function () {
    // waitForApp() + resetApp() can exceed the default 30s Mocha hook budget.
    this.timeout(90_000);
    await startMockServer();
    setMockBehavior(
      'composioConnections',
      JSON.stringify([{ id: 'c1', toolkit: 'gmail', status: 'ACTIVE' }])
    );
    setMockBehavior(
      'composioAvailableTriggers',
      JSON.stringify([
        { slug: 'GMAIL_NEW_GMAIL_MESSAGE', scope: 'static' },
        { slug: 'SLACK_NEW_MESSAGE', scope: 'static', requiredConfigKeys: ['channel'] },
      ])
    );
    setMockBehavior('composioActiveTriggers', JSON.stringify([]));
    await waitForApp();
    await resetApp('e2e-composio-triggers-token');
    clearRequestLog();
  });

  after(async () => {
    await stopMockServer();
  });

  it('Triggers section renders in the Composio modal for an ACTIVE connection', async () => {
    // Seed one active trigger so the modal shows both the enabled and
    // available rows when it loads.
    setMockBehavior(
      'composioActiveTriggers',
      JSON.stringify([
        { id: 'ti-seeded', slug: 'GMAIL_NEW_GMAIL_MESSAGE', toolkit: 'gmail', connectionId: 'c1' },
      ])
    );

    await navigateToSkills();

    await waitForText('Integrations', 10_000);
    await waitForText('Gmail', 10_000);

    const opened = await browser.execute(() => {
      const buttons = Array.from(document.querySelectorAll<HTMLButtonElement>('button'));
      const gmailManage = buttons.find(button => {
        const label = button.getAttribute('aria-label') ?? '';
        return /Gmail/i.test(label) && /Manage/i.test(label);
      });
      if (!gmailManage) return false;
      ['mousedown', 'mouseup', 'click'].forEach(type => {
        gmailManage.dispatchEvent(
          new MouseEvent(type, { bubbles: true, cancelable: true, view: window, button: 0 })
        );
      });
      return true;
    });
    if (!opened) {
      throw new Error('Could not find connected Gmail Manage button on Skills page');
    }

    await waitForText('Triggers', 10_000);
    const togglesVisible = await browser.waitUntil(
      async () =>
        Boolean(
          await browser.execute(
            () => document.querySelector('[data-testid="trigger-toggles"]') !== null
          )
        ),
      { timeout: 10_000, interval: 500, timeoutMsg: 'trigger toggles did not render' }
    );
    expect(togglesVisible).toBe(true);
    // formatTriggerLabel('GMAIL_NEW_GMAIL_MESSAGE', { toolkit: 'gmail' }) strips
    // the leading 'Gmail' prefix, producing "New Gmail Message".
    expect(await textExists('New Gmail Message')).toBe(true);
  });
});
