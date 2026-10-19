/**
 * The Recovery Phrase panel lets a user create and back up a local wallet.
 * This flow covers its visible phrase and persisted wallet setup state.
 */
import { waitForApp } from '../helpers/app-helpers';
import { callOpenhumanRpc } from '../helpers/core-rpc';
import { clickText, textExists } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import { setMockBehavior, startMockServer, stopMockServer } from '../mock-server';

const USER_ID = 'e2e-chat-harness-wallet-flow';
const JOHN_ADDRESS = '0x00000000000000000000000000000000000000aa';

async function clickRecoveryConsentCheckbox(): Promise<void> {
  const checkbox = await browser.$('#mnemonic-confirm-checkbox');
  if (!(await checkbox.isExisting())) {
    throw new Error('Recovery phrase consent checkbox not found');
  }
  if (!(await checkbox.isSelected())) {
    try {
      await checkbox.click();
    } catch (err) {
      console.warn(
        `[chat-harness-wallet-flow] checkbox click intercepted; applying DOM fallback: ${err}`
      );
      await browser.execute(() => {
        const input = document.querySelector<HTMLInputElement>('#mnemonic-confirm-checkbox');
        if (!input) return;
        input.checked = true;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
      });
    }
  }
  await browser.waitUntil(async () => await checkbox.isSelected(), {
    timeout: 5_000,
    timeoutMsg: 'Recovery phrase consent checkbox did not become selected',
  });
}

describe('Chat harness — wallet flow', () => {
  before(async function beforeSuite() {
    this.timeout(90_000);
    await startMockServer();
    await waitForApp();
    // clearAuthSession drops a prior chat-harness spec's leftover session token
    // so the wallet turn starts from a clean signed-in state (a
    // polluted session was the source of the intermittent quote-store failures).
    await resetApp(USER_ID, { clearAuthSession: true });
  });

  after(async () => {
    setMockBehavior('llmForcedResponses', '');
    setMockBehavior('llmStreamChunkDelayMs', '');
    await stopMockServer();
  });

  it('sets up the local wallet through the Recovery Phrase panel and persists wallet state', async function () {
    this.timeout(90_000);
    await navigateViaHash('/settings/recovery-phrase');
    await browser.waitUntil(
      async () =>
        (await textExists('Save Recovery Phrase')) ||
        (await textExists('Your wallet is already set up')),
      { timeout: 20_000, timeoutMsg: 'Recovery Phrase panel did not mount' }
    );

    const alreadyConfigured = await callOpenhumanRpc<{ result: { configured: boolean } }>(
      'openhuman.wallet_status',
      {}
    );
    if (alreadyConfigured.ok && alreadyConfigured.result?.result?.configured === true) {
      console.log(
        '[chat-harness-wallet-flow] Wallet already configured after reset; verifying state'
      );
    } else {
      await clickRecoveryConsentCheckbox();
      await clickText('Save Recovery Phrase', 10_000);

      await browser.waitUntil(async () => await textExists('Recovery phrase saved'), {
        timeout: 20_000,
        timeoutMsg: 'wallet setup success message never rendered',
      });
    }

    await browser.waitUntil(
      async () => {
        const status = await callOpenhumanRpc<{ result: { configured: boolean } }>(
          'openhuman.wallet_status',
          {}
        );
        return status.ok && status.result?.result?.configured === true;
      },
      { timeout: 20_000, timeoutMsg: 'wallet_status never became configured' }
    );

    const walletState = await callOpenhumanRpc<{
      result: { content_utf8: string; truncated: boolean };
    }>('openhuman.test_support_read_workspace_file', {
      rel_path: 'state/wallet-state.json',
      max_bytes: 131_072,
    });
    expect(walletState.ok).toBe(true);
    const content = walletState.result?.result?.content_utf8 ?? '';
    expect(content).toContain('"consentGranted": true');
    expect(content).toContain('"source": "generated"');
    expect(content).toContain('"chain": "evm"');
    expect(content).toContain('"chain": "btc"');
    expect(content).toContain('"chain": "solana"');
    expect(content).toContain('"chain": "tron"');
  });
});
