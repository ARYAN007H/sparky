/**
 * E2E coverage for device-offline feedback in the signed-in home surface.
 * Socket recovery and local-core health selection have focused store and
 * service tests; this suite exercises the user-visible browser event path.
 */
import { waitForApp, waitForAppReady } from '../helpers/app-helpers';
import { textExists } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import { resetMockBehavior, startMockServer, stopMockServer } from '../mock-server';

const USER_ID = 'e2e-connectivity-state-differentiation';

async function setDeviceOffline(offline: boolean): Promise<void> {
  await browser.execute((value: boolean) => {
    Object.defineProperty(navigator, 'onLine', { configurable: true, value: !value });
    window.dispatchEvent(new Event(value ? 'offline' : 'online'));
  }, offline);
}

describe('Connectivity state differentiation', () => {
  before(async function beforeSuite() {
    this.timeout(120_000);
    await startMockServer();
    await waitForApp();
    await resetApp(USER_ID);
  });

  afterEach(async () => {
    resetMockBehavior();
    try {
      await setDeviceOffline(false);
    } catch {
      // A failed WebView will be reset by the next test setup.
    }
  });

  after(async () => {
    await stopMockServer();
  });

  it('shows device-offline copy when the browser fires offline', async function () {
    this.timeout(30_000);
    await navigateViaHash('/home');
    await waitForAppReady(10_000);
    await setDeviceOffline(true);

    await browser.waitUntil(async () => await textExists('Your device is offline right now'), {
      timeout: 10_000,
      interval: 250,
      timeoutMsg: 'home did not render its device-offline message',
    });
  });
});
