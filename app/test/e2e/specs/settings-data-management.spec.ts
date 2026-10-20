// @ts-nocheck
/**
 * Settings → Data Management (capability 13.5).
 *
 * Rewritten to follow the cron-jobs-flow pattern. The "Full State Reset"
 * test intentionally runs LAST — it logs the user out, so anything that
 * follows would need its own resetApp() pass. We keep this spec
 * self-contained so the suite ordering doesn't matter.
 *
 * Covers:
 *   - 13.5.1 Clear App Data confirmation dialog + Cancel
 *   - 13.5.3 Full State Reset → back to Welcome screen
 */
import fs from 'node:fs';
import path from 'node:path';

import { waitForApp } from '../helpers/app-helpers';
import { callOpenhumanRpc } from '../helpers/core-rpc';
import { clickTestId, clickText, textExists, waitForText } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import { startMockServer, stopMockServer } from '../mock-server';

const USER_ID = 'e2e-settings-data-mgmt';

describe('Settings - Data Management', function () {
  this.timeout(90_000);

  before(async () => {
    await startMockServer();
    await waitForApp();
    await resetApp(USER_ID);
  });

  after(async () => {
    await stopMockServer();
  });

  it('shows Clear App Data confirmation dialog and handles Cancel (13.5.1)', async () => {
    await navigateViaHash('/settings/account');
    await clickTestId('settings-nav-logout-and-clear', 15_000);

    await waitForText('This will sign you out and permanently delete local app data', 5_000);

    await clickText('Cancel');
    expect(await textExists('This will sign you out and permanently delete local app data')).toBe(
      false
    );
    expect(await textExists('Clear app data')).toBe(true);
  });

  it('performs Full State Reset (13.5.3)', async function () {
    this.timeout(90_000);
    const workspace = await callOpenhumanRpc('openhuman.test_support_workspace_root', {});
    expect(workspace.ok).toBe(true);
    const workspacePath = workspace.result?.result?.path;
    expect(typeof workspacePath).toBe('string');
    expect(workspace.result?.result?.exists).toBe(true);
    const authBeforeReset = await callOpenhumanRpc('openhuman.auth_get_state', {});
    expect(authBeforeReset.ok).toBe(true);
    expect(authBeforeReset.result?.result?.userId).toBe(USER_ID);
    expect(process.env.OPENHUMAN_WORKSPACE).toBeTruthy();
    const isolatedWorkspace = path.resolve(process.env.OPENHUMAN_WORKSPACE as string);
    const workspaceRelativePath = path.relative(isolatedWorkspace, workspacePath);
    expect(workspaceRelativePath.startsWith('..')).toBe(false);
    expect(path.isAbsolute(workspaceRelativePath)).toBe(false);
    const sentinelPath = path.join(workspacePath, 'settings-data-management-sentinel.txt');
    fs.writeFileSync(sentinelPath, 'user-scoped clear-data sentinel', { flag: 'wx' });
    expect(fs.readFileSync(sentinelPath, 'utf8')).toBe('user-scoped clear-data sentinel');

    await navigateViaHash('/settings/account');
    await clickTestId('settings-nav-logout-and-clear', 15_000);

    await waitForText('This will sign you out', 5_000);
    const confirmations = await browser.$$('button*=Clear App Data');
    expect(confirmations.length).toBeGreaterThan(1);
    await confirmations[confirmations.length - 1].click();
    await browser.waitUntil(async () => await textExists('Welcome to OpenHuman'), {
      timeout: 30_000,
      interval: 250,
      timeoutMsg: 'clear-data reset did not reload the signed-out Welcome screen',
    });
    expect(await browser.$('[data-testid="welcome-cta-tinyhumans"]').isExisting()).toBe(true);
    expect(fs.existsSync(sentinelPath)).toBe(false);

    const auth = await callOpenhumanRpc('openhuman.auth_get_state', {});
    expect(auth.ok).toBe(true);
    expect(auth.result?.result?.isAuthenticated).toBe(false);
    expect(auth.result?.result?.credential).toBeUndefined();
    const ping = await callOpenhumanRpc('core.ping', {});
    expect(ping.ok).toBe(true);
  });
});
