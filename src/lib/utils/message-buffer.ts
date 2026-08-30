// Pure merge decisions for the inbox add-buffer.
//
// `services/api.ts` batches `messages:added` bursts for a few tens of
// milliseconds before committing them to the store, so at any moment a row can
// live in one of two places: the store, or the pending buffer. Every event that
// addresses a row by id (`messages:updated`, `messages:removed`, `sms:new`)
// therefore has to consider both, or it silently operates on half the inbox.
//
// The decisions are plain data in / plain data out, kept out of the rune store
// so `npm test` can pin them down.

/** Anything the buffer can hold; only the identity matters here. */
interface HasId {
  id: number;
}

/**
 * Pick the rows from a pending burst that should actually be appended.
 *
 * Drops ids the store already holds — `sms:new` can commit a row directly while
 * a copy of it is still sitting in the buffer — and collapses duplicates inside
 * the burst itself, keeping the last occurrence because a later frame for the
 * same id is the newer version of that row.
 */
export function selectRowsToAdd<T extends HasId>(
  existingIds: ReadonlySet<number>,
  incoming: readonly T[],
): T[] {
  const byId = new Map<number, T>();
  for (const row of incoming) {
    if (existingIds.has(row.id)) continue;
    byId.set(row.id, row);
  }
  return [...byId.values()];
}

/**
 * Apply an update to a row that has not been committed yet.
 *
 * Returns the patched buffer, or `null` when the id is not buffered (the caller
 * then falls back to the store). Replacing the buffered row in place is what
 * makes the ordering safe: the flush appends whatever the buffer holds at flush
 * time, so there is no stale copy left to overwrite the update afterwards.
 */
export function applyBufferedUpdate<T extends HasId>(
  buffer: readonly T[],
  updated: T,
): T[] | null {
  let found = false;
  const next = buffer.map((row) => {
    if (row.id !== updated.id) return row;
    found = true;
    return updated;
  });
  return found ? next : null;
}

/**
 * Forget buffered rows that have just been deleted.
 *
 * Without this a row removed while buffered reappears in the store on the next
 * flush — the delete is applied to the store only, and the buffer still holds
 * the row it never saw.
 */
export function dropBufferedIds<T extends HasId>(
  buffer: readonly T[],
  ids: Iterable<number>,
): T[] {
  const drop = new Set(ids);
  if (drop.size === 0) return [...buffer];
  return buffer.filter((row) => !drop.has(row.id));
}
