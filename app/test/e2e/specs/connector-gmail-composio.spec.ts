/**
 * E2E: Gmail (Composio) connector flow.
 * Covers the connector card and its connection states.
 */
import { waitForApp } from '../helpers/app-helpers';
import {
  assertConnectorCardVisible,
  assertModalPhase,
  assertSessionNotNuked,
  openConnectorModal,
  seedComposioConnection,
  seedComposioToolkits,
} from '../helpers/composio-helpers';
import {
  textExists,
  waitForText,
  waitForWebView,
  waitForWindowVisible,
} from '../helpers/element-helpers';
import { triggerAuthLoopbackBypass } from '../helpers/loopback-auth-helpers';
import { completeOnboardingIfVisible, navigateToSkills } from '../helpers/shared-flows';
import {
  clearRequestLog,
  resetMockBehavior,
  setMockBehavior,
  startMockServer,
  stopMockServer,
} from '../mock-server';

const LOG = '[ConnectorGmailComposioE2E]';
const CONNECTOR_NAME = 'Gmail';
const TOOLKIT_SLUG = 'gmail';
const AUTH_TOKEN = 'e2e-connector-gmail-composio-token';

describe('Gmail (Composio) connector flow', () => {
  before(async function () {
    this.timeout(90_000);
    await startMockServer();
    seedComposioToolkits([TOOLKIT_SLUG]);
    seedComposioConnection(TOOLKIT_SLUG, 'ACTIVE', 'c-gmail-1');
    await waitForApp();
    clearRequestLog();
    await triggerAuthLoopbackBypass(AUTH_TOKEN);
    await waitForWindowVisible(25_000);
    await waitForWebView(15_000);
    await completeOnboardingIfVisible(LOG);
  });

  after(async () => {
    await stopMockServer();
  });

  afterEach(async () => {
    resetMockBehavior();
    seedComposioToolkits([TOOLKIT_SLUG]);
    seedComposioConnection(TOOLKIT_SLUG, 'ACTIVE', 'c-gmail-1');
  });

  it('card is visible and selectable', async function () {
    this.timeout(60_000);
    await assertConnectorCardVisible(CONNECTOR_NAME);
    console.log(`${LOG} PASS: card visible`);
  });


  it('failed connection shows error state, not blank screen', async function () {
    this.timeout(60_000);
    seedComposioConnection(TOOLKIT_SLUG, 'FAILED', 'c-gmail-fail');
    await navigateToSkills();
    await waitForText(CONNECTOR_NAME, 10_000);
    const alive = await textExists(CONNECTOR_NAME);
    expect(alive).toBe(true);
    await assertSessionNotNuked();
    console.log(`${LOG} PASS: failed state does not blank screen`);
  });

  it('expired auth shows Reconnect button and does not log user out', async function () {
    this.timeout(60_000);
    seedComposioConnection(TOOLKIT_SLUG, 'EXPIRED', 'c-gmail-expired');
    await navigateToSkills();
    await waitForText(CONNECTOR_NAME, 10_000);
    const modal = await openConnectorModal(CONNECTOR_NAME, 15_000, 'Auth expired');
    expect(modal).toBeTruthy();
    await assertModalPhase('expired', CONNECTOR_NAME);
    await assertSessionNotNuked();
    console.log(`${LOG} PASS: expired auth does not log user out`);
  });
});
