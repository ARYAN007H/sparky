// @ts-nocheck
import { browser, expect } from '@wdio/globals';

import { waitForApp } from '../helpers/app-helpers';
import { callOpenhumanRpc } from '../helpers/core-rpc';
import { clickSelector, clickText, textExists, waitForText } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import { startMockServer, stopMockServer } from '../mock-server';

const USER_ID = 'e2e-settings-account-preferences';

async function waitForHashContains(fragment: string, timeout = 10_000): Promise<void> {
  await browser.waitUntil(
    async () => String(await browser.execute(() => window.location.hash)).includes(fragment),
    { timeout, interval: 250, timeoutMsg: `hash did not include ${fragment}` }
  );
}

describe('Settings - Account Preferences', function () {
  this.timeout(90_000);

  before(async function beforeSuite() {
    this.timeout(90_000);
    await startMockServer();
    await waitForApp();
    await resetApp(USER_ID);
  });

  after(async () => {
    await stopMockServer();
  });

  it('renders the account settings section route', async function () {
    this.timeout(90_000);
    await navigateViaHash('/settings/account');

    await waitForText('Account', 15_000);
    await waitForText('Preferences', 15_000);
    await waitForText('Language', 15_000);
  });

  it('saves a generated recovery phrase and exposes configured wallet state', async function () {
    this.timeout(90_000);
    await navigateViaHash('/settings/recovery-phrase');

    // A previous suite or persisted local wallet can leave this panel in its
    // existing-wallet view. Follow the product's replacement flow before
    // asserting the generated-phrase controls.
    if (await textExists('Replace wallet')) {
      await clickText('Replace wallet', 10_000);
      await clickText('I understand, replace my wallet', 10_000);
    }
    await clickText('Reveal recovery phrase', 10_000);
    await waitForText('Copy to Clipboard', 15_000);
    await browser.waitUntil(
      async () =>
        await browser.execute(() => {
          const checkbox = document.querySelector<HTMLInputElement>('#mnemonic-confirm-checkbox');
          return checkbox !== null && !checkbox.disabled;
        }),
      {
        timeout: 5_000,
        interval: 100,
        timeoutMsg: 'recovery phrase consent stayed disabled after reveal',
      }
    );
    await clickSelector('input[type="checkbox"]');
    expect(
      await browser.execute(
        () => document.querySelector<HTMLInputElement>('#mnemonic-confirm-checkbox')?.checked
      )
    ).toBe(true);
    await clickText('Save Recovery Phrase', 10_000);

    await waitForText('Recovery phrase saved', 20_000);
    await waitForText('Multi-chain wallet identities are ready', 20_000);

    const wallet = await callOpenhumanRpc('openhuman.wallet_status', {});
    expect(wallet.ok).toBe(true);
    expect(wallet.result?.result?.configured).toBe(true);
    expect((wallet.result?.result?.accounts ?? []).length).toBeGreaterThan(0);

    // The dedicated /settings/connections page (with the "Web3 Wallet:
    // Configured" status card) was removed in PR #2550 settings cleanup.
    // The wallet_status RPC assertion above is the canonical signal that
    // the recovery-phrase flow wired through to the wallet domain.
  });

  it('persists the privacy analytics toggle to core config', async function () {
    this.timeout(90_000);
    const beforeAnalytics = await callOpenhumanRpc('openhuman.config_get_analytics_settings', {});
    expect(beforeAnalytics.ok).toBe(true);

    const initialAnalytics = Boolean(beforeAnalytics.result?.result?.enabled);

    await navigateViaHash('/settings/privacy');
    await waitForText('Privacy', 15_000);
    // Renamed: t('privacy.shareAnonymizedData') = 'Share Product Analytics and Diagnostics'.
    await waitForText('Share Product Analytics and Diagnostics', 15_000);

    await clickSelector('[data-testid="privacy-analytics-toggle"]');
    await browser.waitUntil(
      async () => {
        const analytics = await callOpenhumanRpc('openhuman.config_get_analytics_settings', {});
        return analytics.ok && Boolean(analytics.result?.result?.enabled) === !initialAnalytics;
      },
      { timeout: 15_000, interval: 500, timeoutMsg: 'analytics setting did not persist' }
    );
    await browser.waitUntil(
      async () => {
        const analytics = await callOpenhumanRpc('openhuman.config_get_analytics_settings', {});
        return analytics.ok && Boolean(analytics.result?.result?.enabled) === !initialAnalytics;
      },
      { timeout: 15_000, interval: 500, timeoutMsg: 'privacy settings did not persist' }
    );

    const snapshot = await callOpenhumanRpc('openhuman.app_state_snapshot', {});
    expect(snapshot.ok).toBe(true);
    expect(Boolean(snapshot.result?.result?.analyticsEnabled)).toBe(!initialAnalytics);
  });

  it('opens the billing route and shows the account billing handoff', async function () {
    this.timeout(60_000);
    await navigateViaHash('/settings/account');

    await waitForHashContains('/settings/account');
    await waitForText('Plan & billing', 15_000);
    expect(await textExists('Open dashboard')).toBe(true);
  });
});
