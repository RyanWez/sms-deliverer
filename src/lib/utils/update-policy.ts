/**
 * Rate limiting for update checks.
 *
 * "Check Now" talks to a public release endpoint, and a button that fires a
 * request per click invites someone to sit on it. The rules are kept here as
 * plain functions so they can be tested without a webview or a network stub.
 */

/** Manual checks are refused this long after the previous completed check. */
export const MANUAL_COOLDOWN_MS = 60_000;

/** A background check is skipped if one already ran this recently. */
export const BACKGROUND_MIN_GAP_MS = 15 * 60_000;

/** Milliseconds left on the manual cooldown, `0` once a check is allowed. */
export function cooldownRemaining(
  lastCheckedAt: number | null,
  now: number,
  cooldownMs: number = MANUAL_COOLDOWN_MS,
): number {
  if (!lastCheckedAt) return 0;
  // A clock that jumped backwards would otherwise strand the button for hours.
  if (lastCheckedAt > now) return 0;
  return Math.max(0, cooldownMs - (now - lastCheckedAt));
}

/** Whole seconds left, rounded up, for the countdown on the button. */
export function cooldownSeconds(remainingMs: number): number {
  return Math.ceil(remainingMs / 1000);
}

/**
 * Whether a check may start now.
 *
 * `busy` covers a check already in flight; a manual check additionally waits
 * out the cooldown. Background checks use the longer gap and never queue.
 */
export function canCheck(
  opts: {
    busy: boolean;
    lastCheckedAt: number | null;
    now: number;
    interactive: boolean;
  },
): boolean {
  if (opts.busy) return false;
  const gap = opts.interactive ? MANUAL_COOLDOWN_MS : BACKGROUND_MIN_GAP_MS;
  return cooldownRemaining(opts.lastCheckedAt, opts.now, gap) === 0;
}

/** `1.4 MB` / `812 KB` / `''` for an unknown size. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '';
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${Math.round(bytes / 1024)} KB`;
}

/** Download progress as a whole percentage, `null` when the size is unknown. */
export function downloadPercent(downloaded: number, total: number): number | null {
  if (total <= 0) return null;
  return Math.min(100, Math.max(0, Math.round((downloaded / total) * 100)));
}
