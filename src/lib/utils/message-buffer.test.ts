// Unit tests for the add-buffer merge decisions.
//
// These pin down the ordering rules that `services/api.ts` relies on for rows
// that are still pending: an update must reach a buffered row, a delete must
// remove it, and the flush must never re-introduce a stale version.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  applyBufferedUpdate,
  dropBufferedIds,
  selectRowsToAdd,
} from './message-buffer.ts';

/** Minimal stand-in for SmsItem: id plus the text an update would complete. */
function row(id: number, text: string) {
  return { id, text };
}

test('an update for a buffered row patches it in place', () => {
  const buffer = [row(1, 'first'), row(2, 'Your code is'), row(3, 'third')];
  const patched = applyBufferedUpdate(buffer, row(2, 'Your code is 123456'));
  assert.deepEqual(patched, [row(1, 'first'), row(2, 'Your code is 123456'), row(3, 'third')]);
});

test('patching keeps the row at its original buffer position', () => {
  // Position is the flush order, so a reassembled long message must not jump.
  const buffer = [row(7, 'a'), row(8, 'b')];
  const patched = applyBufferedUpdate(buffer, row(7, 'a-complete'));
  assert.equal(patched?.[0].id, 7);
  assert.equal(patched?.[1].id, 8);
});

test('an update for an unbuffered row reports "not here"', () => {
  assert.equal(applyBufferedUpdate([row(1, 'a')], row(99, 'z')), null);
  assert.equal(applyBufferedUpdate([], row(1, 'a')), null);
});

test('the flush cannot re-stale a patched row', () => {
  // The whole point: patch the buffered copy, then flush. What lands in the
  // store is the updated text, not the partial fragment.
  let buffer = [row(2, 'Your code is')];
  buffer = applyBufferedUpdate(buffer, row(2, 'Your code is 123456')) ?? buffer;
  const toAdd = selectRowsToAdd(new Set<number>(), buffer);
  assert.deepEqual(toAdd, [row(2, 'Your code is 123456')]);
});

test('a delete while buffered stops the row from leaking in at flush', () => {
  const buffer = dropBufferedIds([row(1, 'a'), row(2, 'b'), row(3, 'c')], [2, 3]);
  assert.deepEqual(buffer, [row(1, 'a')]);
  assert.deepEqual(selectRowsToAdd(new Set<number>(), buffer), [row(1, 'a')]);
});

test('deleting nothing leaves the buffer intact', () => {
  const buffer = [row(1, 'a')];
  assert.deepEqual(dropBufferedIds(buffer, []), buffer);
  assert.deepEqual(dropBufferedIds(buffer, [42]), buffer);
});

test('dropping ids does not mutate the buffer it was given', () => {
  const buffer = [row(1, 'a'), row(2, 'b')];
  dropBufferedIds(buffer, [1]);
  assert.equal(buffer.length, 2);
});

test('rows already committed to the store are not added twice', () => {
  const toAdd = selectRowsToAdd(new Set([1, 2]), [row(1, 'a'), row(3, 'c')]);
  assert.deepEqual(toAdd, [row(3, 'c')]);
});

test('a duplicate id inside one burst collapses to the newest frame', () => {
  // Two `messages:added` frames for the same id in one window: the later frame
  // is the newer version of that row, and only one row may be appended.
  const toAdd = selectRowsToAdd(new Set<number>(), [
    row(5, 'partial'),
    row(6, 'other'),
    row(5, 'complete'),
  ]);
  assert.deepEqual(toAdd, [row(5, 'complete'), row(6, 'other')]);
});

test('an empty burst adds nothing', () => {
  assert.deepEqual(selectRowsToAdd(new Set([1]), []), []);
});
