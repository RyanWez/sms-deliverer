/**
 * Browser-preview stand-in for the three Rust Telegram commands.
 *
 * The preview at localhost:1420 has no Tauri and therefore no network at all, so
 * the Forwarding section would be dead there — unclickable buttons and no way to
 * see any outcome. These functions answer the way the backend would, which keeps
 * the layout, the toasts and the error states reviewable without a bot token, a
 * group, or the bank plugged in.
 *
 * They reject with a plain string exactly like `invoke` does, so `api.ts` runs
 * one code path for both branches.
 *
 * The token-shape check duplicates `commands::telegram::require_token`, and that
 * is not frontend validation creeping back in: the real check stays in Rust
 * because the settings store is rehydrated from `localStorage` and can hold
 * anything. This module simulates that Rust answer.
 */

export interface PreviewGroup {
  chatId: string;
  title: string;
  kind: string;
}

export interface PreviewTestResult {
  bot: string;
  migratedChatId: string | null;
}

/** Mirrors the Rust check: `<digits>:<secret>`, both halves non-empty. */
export function looksLikeBotToken(token: string): boolean {
  const [id, secret, ...rest] = token.trim().split(':');
  if (rest.length > 0) return false;
  return !!id && !!secret && /^\d+$/.test(id);
}

/**
 * Derive a stable fake `@name` from the token's numeric half, so the preview
 * shows a different bot for a different token instead of one hardcoded string.
 */
export function previewBotName(token: string): string {
  const id = token.trim().split(':')[0] ?? '';
  return `@preview_bot_${id.slice(-4)}`;
}

const TOKEN_REQUIRED = 'Enter the bot token from @BotFather first';
const TOKEN_MALFORMED = 'That does not look like a bot token — expected 123456789:AA...';

function requireToken(token: string): string {
  const trimmed = token.trim();
  if (!trimmed) throw TOKEN_REQUIRED;
  if (!looksLikeBotToken(trimmed)) throw TOKEN_MALFORMED;
  return trimmed;
}

export async function previewVerifyToken(token: string): Promise<string> {
  return previewBotName(requireToken(token));
}

/**
 * A basic group, deliberately — its `-100`-less id is what makes the UI's
 * "upgrade this to a supergroup" warning visible in the preview, and it sets up
 * the migration that `previewSendTest` then simulates.
 */
export async function previewDetectGroup(token: string): Promise<PreviewGroup> {
  requireToken(token);
  return { chatId: '-4855120394', title: 'OTP Vault (preview)', kind: 'group' };
}

/**
 * Simulates the supergroup migration for any id that is not already a `-100…`
 * supergroup id, so the auto-heal path can be exercised without waiting for a
 * real group to be upgraded.
 */
export async function previewSendTest(
  token: string,
  chatId: string,
): Promise<PreviewTestResult> {
  const bot = previewBotName(requireToken(token));
  const id = chatId.trim();
  if (!id) throw 'No destination group yet — press Detect Group ID first';
  if (!id.startsWith('-')) throw 'Telegram rejected the request: Bad Request: chat not found';
  if (id.startsWith('-100')) return { bot, migratedChatId: null };
  return { bot, migratedChatId: `-100${id.slice(1)}` };
}
