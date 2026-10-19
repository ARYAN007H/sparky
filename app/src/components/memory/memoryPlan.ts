/**
 * What the account's plan says about hosted (TinyHumans) memory: the storage
 * each plan includes, whether a plan includes memory at all, and where the app
 * sends an "upgrade" or "top up".
 */

/** Free hosted memory per plan; memory inference itself is never charged. */
// ponytail: hard-coded until the backend exposes a memory entitlement; the core's
// PLAN_REQUIRED refusal (#8081) is the backstop when this drifts.
export const MEMORY_QUOTA = { BASIC: '1 GB', PRO: '20 GB' } as const;

/** Whether `plan` (FREE / BASIC / PRO) includes hosted memory. */
export function planIncludesMemory(plan: string): boolean {
  return Object.keys(MEMORY_QUOTA).includes(plan);
}

/** Where the app sends every "top up" / "upgrade" action (`open_billing` in useAppNotices). */
export const BILLING_ROUTE = '/settings/account';
