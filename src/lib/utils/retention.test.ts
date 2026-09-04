// Zero-dependency unit tests for the retention window list and the coercion
// that keeps a stored profile inside it. Run with `npm test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  DEFAULT_RETENTION_HOURS,
  RETENTION_OPTIONS,
  normalizeRetentionHours,
} from './retention.ts';

test('one window is offered, and it is one hour', () => {
  assert.deepEqual(RETENTION_OPTIONS, [{ value: 1, label: '1 Hour' }]);
  assert.equal(DEFAULT_RETENTION_HOURS, 1);
});

test('the default is an offered window by construction', () => {
  // types.ts reads DEFAULT_RETENTION_HOURS for DEFAULT_SETTINGS rather than
  // repeating a number, so this cannot drift. types.ts itself is not importable
  // here — it re-exports ./types/logs without a file extension, which Vite
  // resolves and Node's ESM loader does not.
  assert.ok(RETENTION_OPTIONS.some((o) => o.value === DEFAULT_RETENTION_HOURS));
});

test('a window that is no longer offered falls back to the default', () => {
  // 0 was "keep everything", 2 the old default, 168 the 7-day entry. None of
  // them can be shown now, so a profile carrying one starts pruning at an hour.
  for (const stored of [0, 2, 4, 8, 24, 168, -5, 0.5, 1.5]) {
    assert.equal(normalizeRetentionHours(stored), 1);
  }
});

test('an offered window is left alone', () => {
  assert.equal(normalizeRetentionHours(1), 1);
});

test('junk in the profile resolves to the default instead of reaching Rust', () => {
  for (const junk of [null, undefined, '', 'two hours', NaN, Infinity, -Infinity, {}, []]) {
    assert.equal(normalizeRetentionHours(junk), 1);
  }
});

test('a number written as a string is read, then checked against the list', () => {
  // An older profile or a hand edit can leave the value quoted.
  assert.equal(normalizeRetentionHours('1'), 1);
  assert.equal(normalizeRetentionHours(' 1 '), 1);
  assert.equal(normalizeRetentionHours('24'), 1);
});
