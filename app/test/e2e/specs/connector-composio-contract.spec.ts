/**
 * E2E: Composio connector contract — one table-driven spec covering the
 * toolkits that share the identical connect/sync/execute/disconnect flow
 * (plan.md §2.2). Collapses 11 byte-identical connector-*.spec.ts files.
 *
 * Bespoke connectors keep their own specs:
 *   - connector-jira.spec.ts           (subdomain-field UI)
 *   - connector-gmail-composio.spec.ts (400-on-fetch-emails handling)
 */
import { type ConnectorContractConfig, runConnectorContract } from '../helpers/connector-contract';

const TOOLKITS: ConnectorContractConfig[] = [
  { name: 'Slack', slug: 'slack', idBase: 'c-slack', executeAction: 'SLACK_LIST_CHANNELS' },
];

for (const toolkit of TOOLKITS) {
  runConnectorContract(toolkit);
}
