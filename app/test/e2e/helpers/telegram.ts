/**
 * Telegram channel E2E helpers.
 *
 * Wraps `callOpenhumanRpc` so UI smoke specs can exercise Telegram channel
 * configuration without knowing the raw RPC method names.
 *
 * Design principles:
 *  - All helpers are pure async functions — no hidden state.
 *  - RPC helpers forward to `callOpenhumanRpc` using the exact field names from
 *    `crates/openhuman-core/src/channels/controllers/schemas.rs` (camelCase for the wire
 *    format; the Rust serde layer translates).
 *
 * Key RPC shapes (verified from schemas.rs / ops.rs):
 *   channels_connect  -> { channel, authMode, credentials: { bot_token, allowed_users?, mention_only? } }
 *   channels_disconnect -> { channel, authMode }
 *   channels_status   -> { channel? }   -> entries: ChannelStatusEntry[]
 */
import { callOpenhumanRpc } from './core-rpc';

const LOG_PREFIX = '[TelegramChannel]';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface TelegramConnectOptions {
  /** Bot token issued by BotFather. */
  botToken: string;
  /**
   * Optional allowlist of Telegram usernames (without `@`) or numeric user
   * IDs. When provided only those identities can trigger the bot. When
   * omitted the bot uses the pairing-code flow.
   */
  allowedUsers?: string[];
  /**
   * When true the bot only responds to messages that mention it by
   * `@username` in group chats.
   */
  mentionOnly?: boolean;
}

interface TelegramConnectResult {
  ok: boolean;
  status?: string;
  restartRequired?: boolean;
  message?: string;
  error?: string;
}

interface TelegramStatusEntry {
  channelId: string;
  authMode: string;
  connected: boolean;
  hasCredentials: boolean;
}

// ---------------------------------------------------------------------------
// RPC wrappers
// ---------------------------------------------------------------------------

/**
 * Connect a Telegram bot via the `channels_connect` RPC.
 *
 * Maps to `openhuman.channels_connect` with `authMode: "bot_token"`.
 * The connect call writes TOML config and sets `restart_required: true` —
 * it does NOT start the live polling loop immediately.
 *
 * Returns the raw RPC result so callers can assert on specific fields.
 */
export async function connectTelegramBot(
  opts: TelegramConnectOptions
): Promise<TelegramConnectResult> {
  const credentials: Record<string, unknown> = { bot_token: opts.botToken };
  if (opts.allowedUsers !== undefined) {
    credentials.allowed_users = opts.allowedUsers;
  }
  if (opts.mentionOnly !== undefined) {
    credentials.mention_only = opts.mentionOnly;
  }

  console.log(
    `${LOG_PREFIX} connectTelegramBot: token=***${opts.botToken.slice(-4)} ` +
      `allowedUsers=${JSON.stringify(opts.allowedUsers ?? [])} ` +
      `mentionOnly=${opts.mentionOnly ?? false}`
  );

  const out = await callOpenhumanRpc('openhuman.channels_connect', {
    channel: 'telegram',
    authMode: 'bot_token',
    credentials,
  });

  if (!out.ok) {
    console.warn(`${LOG_PREFIX} connectTelegramBot: RPC failed — ${JSON.stringify(out)}`);
    return { ok: false, error: String(out.error ?? 'unknown error') };
  }

  // The result shape from ops.rs is { status, restart_required, message? }.
  // It is wrapped by Outcome which the Node RPC client unwraps one level.
  const result = (out.result as Record<string, unknown> | null) ?? {};
  const inner =
    typeof result.result === 'object' && result.result !== null
      ? (result.result as Record<string, unknown>)
      : result;

  console.log(`${LOG_PREFIX} connectTelegramBot: ok — ${JSON.stringify(inner)}`);
  return {
    ok: true,
    status: inner.status as string | undefined,
    restartRequired: inner.restart_required as boolean | undefined,
    message: inner.message as string | undefined,
  };
}

/**
 * Disconnect the Telegram bot via `channels_disconnect` RPC.
 *
 * Removes stored credentials and clears TOML config. Returns true on success.
 */
export async function disconnectTelegramBot(): Promise<boolean> {
  console.log(`${LOG_PREFIX} disconnectTelegramBot: calling channels_disconnect`);

  const out = await callOpenhumanRpc('openhuman.channels_disconnect', {
    channel: 'telegram',
    authMode: 'bot_token',
  });

  if (!out.ok) {
    console.warn(`${LOG_PREFIX} disconnectTelegramBot: RPC failed — ${JSON.stringify(out)}`);
    return false;
  }

  console.log(`${LOG_PREFIX} disconnectTelegramBot: ok`);
  return true;
}

/**
 * Fetch the channel status for Telegram.
 *
 * Calls `channels_status` with `channel: "telegram"` and returns the first
 * matching entry (the `bot_token` mode entry). Returns `null` if no entry is
 * found or the RPC fails.
 */
export async function getTelegramChannelStatus(): Promise<TelegramStatusEntry | null> {
  console.log(`${LOG_PREFIX} getTelegramChannelStatus: calling channels_status`);

  const out = await callOpenhumanRpc('openhuman.channels_status', { channel: 'telegram' });

  if (!out.ok) {
    console.warn(`${LOG_PREFIX} getTelegramChannelStatus: RPC failed — ${JSON.stringify(out)}`);
    return null;
  }

  // channels_status returns entries: ChannelStatusEntry[].
  // The core wraps with Outcome so the Node client may unwrap one level.
  const result = (out.result as Record<string, unknown> | null) ?? {};
  const entries: TelegramStatusEntry[] = Array.isArray(result)
    ? result
    : Array.isArray((result as Record<string, unknown>).entries)
      ? ((result as Record<string, unknown>).entries as TelegramStatusEntry[])
      : Array.isArray((result as Record<string, unknown>).result)
        ? ((result as Record<string, unknown>).result as TelegramStatusEntry[])
        : [];

  // The core serialises these in snake_case, so each entry carries keys the
  // camelCase interface does not declare. `as unknown as` rather than a direct
  // assertion: the two types genuinely do not overlap, and TS is right to say
  // so — going through `unknown` states that we know we are reading past the
  // declared shape instead of pretending the shapes match.
  const asLoose = (entry: TelegramStatusEntry): Record<string, unknown> =>
    entry as unknown as Record<string, unknown>;
  const raw = entries.find(
    (e: TelegramStatusEntry) =>
      (e.channelId === 'telegram' || asLoose(e).channel_id === 'telegram') &&
      (e.authMode === 'bot_token' || asLoose(e).auth_mode === 'bot_token')
  ) as (TelegramStatusEntry & Record<string, unknown>) | undefined;

  // Normalise snake_case fields that the Rust core serialises.
  const match: TelegramStatusEntry | undefined = raw
    ? {
        channelId: (raw.channelId ?? raw.channel_id) as string,
        authMode: (raw.authMode ?? raw.auth_mode) as string,
        connected: raw.connected,
        hasCredentials: (raw.hasCredentials ?? raw.has_credentials ?? false) as boolean,
      }
    : undefined;

  console.log(`${LOG_PREFIX} getTelegramChannelStatus: ${JSON.stringify(match ?? null)}`);
  return match ?? null;
}
