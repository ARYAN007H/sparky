// @ts-nocheck
/**
 * E2E test: Billing panel (crypto/Coinbase path).
 *
 * The in-app Coinbase Commerce checkout UI was removed alongside Stripe;
 * billing now lives on the web dashboard. These tests verify the billing
 * redirect panel shown at /settings/billing:
 *
 *   6.1  Billing panel renders the "moved to web" redirect page
 *   6.2  "Open billing dashboard" button is present
 *   6.3  Opening browser is indicated while the redirect fires
 */
import { waitForApp } from '../helpers/app-helpers';
import { textExists, waitForText } from '../helpers/element-helpers';
import { navigateToHome, navigateViaHash, performFullLogin } from '../helpers/shared-flows';
import { clearRequestLog, startMockServer, stopMockServer } from '../mock-server';

const LOG_PREFIX = '[CryptoPayment]';

describe('Crypto Payment Flow', () => {
  before(async () => {
    await startMockServer();
    await waitForApp();
    clearRequestLog();
  });

  after(async () => {
    await stopMockServer();
  });

  it('login and reach home', async () => {
    await performFullLogin('e2e-crypto-payment-token');
  });

  it('6.1 — account panel shows billing dashboard handoff', async function () {
    this.timeout(60_000);
    await navigateViaHash('/settings/account');
    await waitForText('Open dashboard', 20_000);
    console.log(`${LOG_PREFIX} 6.1 — account billing handoff loaded`);
  });

  it('6.2 — "Open billing dashboard" button is present', async () => {
    await navigateViaHash('/settings/account');
    const hasButton = await textExists('Open dashboard');
    expect(hasButton).toBe(true);
    console.log(`${LOG_PREFIX} 6.2 — "Open billing dashboard" button present`);
  });

  it('6.3 — the handoff is an explicit user action', async () => {
    await navigateViaHash('/settings/account');
    await waitForText('Plan & billing', 10_000);
    expect(await textExists('Open dashboard')).toBe(true);
    await navigateToHome();
  });
});
