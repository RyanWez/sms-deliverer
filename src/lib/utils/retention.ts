/**
 * The retention windows Settings offers, and the only values the store may hold.
 *
 * Deliberately one entry. A SIM bank is a transit point for codes that expire in
 * minutes, so every extra hour a message survives is an hour it can be read off
 * a screen nobody is watching, and the operator asked for the longer windows and
 * the "keep everything" entry to be taken away rather than left selectable.
 *
 * Re-adding a window is an entry in this list and nothing else:
 * `normalizeRetentionHours` reads the same array, so a stored profile can never
 * hold a window the page has no option for. Import it rather than repeating the
 * numbers — a `<select>` whose value matches none of its options renders blank
 * while the old value stays in force, which is a page showing nothing while the
 * backend prunes on something else.
 *
 * Rune-free and `$lib`-free so the test runner can import it directly.
 */

export interface RetentionOption {
  /** Hours. Reaches Rust as the `retentionHours` argument of `start_live`. */
  value: number;
  label: string;
}

export const RETENTION_OPTIONS: RetentionOption[] = [{ value: 1, label: '1 Hour' }];

/** The window a profile falls back to. Always an offered one. */
export const DEFAULT_RETENTION_HOURS = RETENTION_OPTIONS[0].value;

/**
 * The nearest offered window to whatever was stored.
 *
 * `localStorage` can hold anything an older build, a hand edit or a corrupted
 * write left behind — a string, `null`, `0` from the profile of a user who had
 * the old "keep everything" entry selected, or `168` from one who picked 7 days.
 * None of those can be shown or honoured now, so they all resolve to the
 * default; a profile that used to keep everything starts pruning at one hour,
 * because there is no longer a window that does not.
 */
export function normalizeRetentionHours(raw: unknown): number {
  const n = typeof raw === 'number' ? raw : Number.parseFloat(String(raw));
  if (!Number.isFinite(n)) return DEFAULT_RETENTION_HOURS;
  return RETENTION_OPTIONS.some((o) => o.value === n) ? n : DEFAULT_RETENTION_HOURS;
}
