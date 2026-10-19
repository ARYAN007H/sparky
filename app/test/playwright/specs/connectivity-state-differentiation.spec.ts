import { test } from '@playwright/test';

import { bootAuthenticatedPage, waitForAppReady } from '../helpers/core-rpc';

async function setDeviceOffline(page: import('@playwright/test').Page, offline: boolean) {
  await page.evaluate(isOffline => {
    Object.defineProperty(navigator, 'onLine', { configurable: true, value: !isOffline });
    window.dispatchEvent(new Event(isOffline ? 'offline' : 'online'));
  }, offline);
}

test.describe('Connectivity state differentiation', () => {
  test.beforeEach(async ({ page }, testInfo) => {
    const testSlug = testInfo.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
    await bootAuthenticatedPage(page, 'pw-connectivity-diff-' + testSlug, '/home');
  });

  test.afterEach(async ({ page }) => {
    await setDeviceOffline(page, false);
  });

  // `backend-only` is the browser's Socket.IO link to the local core. The mock
  // immediately accepts reconnects after a server-side disconnect, so the
  // transient reconnect label cannot be asserted deterministically here;
  // selector behavior has focused store coverage.

  test('shows device-offline copy (not backend-only) when window fires offline', async ({ page }) => {
    await setDeviceOffline(page, true);
    await expect(page.getByText('Your device is offline right now')).toBeVisible();
  });

  // The core is a separate process in this lane, but the runner has no
  // supported control for stopping and restarting it during a page test.

  test('baseline app shell is ready in the browser lane', async ({ page }) => {
    await waitForAppReady(page);
  });
});
