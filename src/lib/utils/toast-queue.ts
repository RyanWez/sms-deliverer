/**
 * Toast queue arithmetic: bounded, and coalescing repeats of the same notice.
 *
 * Rune-free and `$lib`-free so the Node test runner can import it directly.
 *
 * The problem this solves is specific to a SIM bank. Toasts live 4 s in a fixed
 * bottom-right column with no max-height, and there was no cap: 16 ports losing
 * their connection at once stacked 16 cards up past the top of the viewport and
 * covered the UI that was reporting the failure. Adding a toast for
 * `live:reconnecting` made that reachable in normal operation rather than in
 * theory.
 */

/** The parts of a toast this module reasons about. */
export interface ToastLike {
  id: number;
  kind: string;
  title: string;
  body: string;
  otp: string | null;
  /** How many identical notices this card stands for. Absent means one. */
  count?: number;
}

/**
 * Most toasts on screen at once.
 *
 * Five is what fits above the fold at the smallest window this app is usable in;
 * past that the column starts hiding the page rather than annotating it.
 */
export const MAX_TOASTS = 5;

/**
 * Notices of the same kind and title collapse onto one card. Two things must
 * never collapse:
 *
 * - anything carrying an OTP, because each code is a distinct thing the operator
 *   came to read and merging would silently discard one;
 * - notices whose title already differs, which is what keeps "Delete complete"
 *   apart from "Delete incomplete".
 *
 * The body is deliberately the newest one rather than a merged list: the count
 * says how many there were, and a body assembled from 16 port names is unreadable
 * in a 4-second card.
 */
function coalescesWith(a: ToastLike, b: ToastLike): boolean {
  return a.otp === null && b.otp === null && a.kind === b.kind && a.title === b.title;
}

/**
 * Add `next` to `list`, coalescing it onto a matching card and capping the
 * result at [`MAX_TOASTS`] newest.
 *
 * Returns a new array; `list` is not mutated.
 */
export function pushToast<T extends ToastLike>(list: readonly T[], next: T): T[] {
  const existing = list.findIndex((t) => coalescesWith(t, next));
  if (existing !== -1) {
    const merged = {
      ...next,
      count: (list[existing].count ?? 1) + (next.count ?? 1),
    };
    // Move the merged card to the end so a repeat re-announces itself at the
    // bottom of the column instead of updating a card the eye has left.
    const kept = list.filter((_, i) => i !== existing);
    return [...kept, merged].slice(-MAX_TOASTS);
  }
  return [...list, next].slice(-MAX_TOASTS);
}

/** Drop the toast with `id`, if it is still there. */
export function dismissToast<T extends ToastLike>(list: readonly T[], id: number): T[] {
  return list.filter((t) => t.id !== id);
}

/**
 * Suffix for a coalesced card's title, e.g. ` (3)`. Empty for a single notice, so
 * the common case reads exactly as it did before.
 */
export function countSuffix(t: ToastLike): string {
  const n = t.count ?? 1;
  return n > 1 ? ` (${n})` : '';
}
