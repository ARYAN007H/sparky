import { expect, test } from '@playwright/test';

import { bootAuthenticatedPage, callCoreRpc, waitForAppReady } from '../helpers/core-rpc';

async function openCronJobsPanel(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/#/settings/cron-jobs');
  await waitForAppReady(page);
  // Panel title dropped in the PanelPage migration; the panel test id and its
  // Scheduled Jobs section confirm it mounted.
  await expect(page.getByTestId('cron-jobs-panel')).toBeVisible();
  await expect(page.getByText('Scheduled Jobs').first()).toBeVisible();
}

test.describe('Cron jobs settings panel', () => {
  test.beforeEach(async ({ page }) => {
    await bootAuthenticatedPage(page, 'pw-cron-jobs-flow', '/home');
  });

  test('chat surface is reachable after login', async ({ page }) => {
    // Home folded into the unified chat surface: post-login landing is /chat.
    await waitForAppReady(page);
    await expect(page.getByTestId('chat-message-input')).toBeVisible();
  });

  test('a cron row toggles, persists after refresh, and can be removed', async ({ page }) => {
    const name = 'e2e_runner_consolidation';
    const created = (await callCoreRpc<unknown>('openhuman.cron_add', {
      name,
      schedule: { kind: 'cron', expr: '0 8 * * *' },
      job_type: 'agent',
      prompt: 'Disposable runner ownership fixture',
    })) as { result?: { id?: string } };
    const jobId = created.result?.id;
    expect(jobId, 'cron_add returns the created row id').toBeTruthy();
    await openCronJobsPanel(page);
    const row = page.getByTestId(`cron-job-row-${jobId}`);
    const toggle = page.getByTestId(`cron-job-toggle-${jobId}`);
    const remove = page.getByTestId(`cron-job-remove-${jobId}`);

    await expect(row).toBeVisible();
    await expect(toggle).toHaveAttribute('aria-label', 'Pause');
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-label', 'Resume');
    await page.getByTestId('cron-refresh').click();
    await expect(toggle).toHaveAttribute('aria-label', 'Resume');
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-label', 'Pause');

    await remove.click();
    await expect(row).toHaveCount(0);
    const listed = (await callCoreRpc<unknown>('openhuman.cron_list', {})) as unknown;
    const jobs = Array.isArray(listed)
      ? listed
      : listed && typeof listed === 'object' && 'result' in listed
        ? (listed as { result: unknown }).result
        : listed;
    expect(Array.isArray(jobs), 'cron_list returns rows after removal').toBe(true);
    expect((jobs as Array<{ id?: string }>).some(job => job.id === jobId)).toBe(false);
  });

  test('refresh action is visible in the cron jobs panel', async ({ page }) => {
    await openCronJobsPanel(page);
    await expect(page.getByRole('button', { name: 'Refresh Cron Jobs' })).toBeVisible();
  });
});
