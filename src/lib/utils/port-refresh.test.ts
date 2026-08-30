// Unit tests for the background port-refresh helpers.
//
// Same rationale as port.test.ts: Node's built-in runner, no DOM, no rune
// stores — everything under test here is a plain function.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  MAX_PORT_REFRESH_SECONDS,
  MIN_PORT_REFRESH_SECONDS,
  describePortChanges,
  diffPorts,
  portRefreshPeriodMs,
  summarizeNames,
} from './port-refresh.ts';

function ports(...names: string[]) {
  return names.map((name) => ({ name }));
}

test('a sane interval passes through as milliseconds', () => {
  assert.equal(portRefreshPeriodMs(30), 30_000);
  assert.equal(portRefreshPeriodMs(300), 300_000);
});

test('0 means the refresh is turned off', () => {
  assert.equal(portRefreshPeriodMs(0), null);
});

test('junk from an old profile disables rather than tight-loops', () => {
  // The Settings input only puts min/max on the DOM element, so any of these can
  // reach the scheduler from localStorage.
  assert.equal(portRefreshPeriodMs(-5), null);
  assert.equal(portRefreshPeriodMs(Number.NaN), null);
  assert.equal(portRefreshPeriodMs(undefined), null);
  assert.equal(portRefreshPeriodMs('' as unknown), null);
  assert.equal(portRefreshPeriodMs(Number.POSITIVE_INFINITY), null);
});

test('a too-small positive interval is floored, not treated as off', () => {
  // 1s would re-enumerate 64 serial devices every second.
  assert.equal(portRefreshPeriodMs(1), MIN_PORT_REFRESH_SECONDS * 1000);
  assert.equal(portRefreshPeriodMs(4.9), MIN_PORT_REFRESH_SECONDS * 1000);
});

test('an absurd interval is capped below the setInterval overflow point', () => {
  // A delay above 2^31-1 ms wraps around and fires almost immediately.
  const ms = portRefreshPeriodMs(1e12);
  assert.equal(ms, MAX_PORT_REFRESH_SECONDS * 1000);
  assert.ok(ms !== null && ms < 2 ** 31 - 1);
});

test('numeric strings are accepted', () => {
  assert.equal(portRefreshPeriodMs('45' as unknown), 45_000);
});

test('an unchanged bank diffs to nothing', () => {
  const before = ports('/dev/ttyUSB0', '/dev/ttyUSB1');
  const after = ports('/dev/ttyUSB0', '/dev/ttyUSB1');
  assert.deepEqual(diffPorts(before, after), { added: [], removed: [] });
});

test('a re-plugged stick shows up as added', () => {
  const diff = diffPorts(ports('COM3'), ports('COM3', 'COM4'));
  assert.deepEqual(diff, { added: ['COM4'], removed: [] });
});

test('a pulled stick shows up as removed', () => {
  const diff = diffPorts(ports('COM3', 'COM4'), ports('COM3'));
  assert.deepEqual(diff, { added: [], removed: ['COM4'] });
});

test('the diff is keyed on name, not on position', () => {
  // The backend orders by port number, so inserting ttyUSB1 shifts every later
  // index — an index-wise comparison would report the whole tail as churn.
  const diff = diffPorts(
    ports('/dev/ttyUSB0', '/dev/ttyUSB2'),
    ports('/dev/ttyUSB0', '/dev/ttyUSB1', '/dev/ttyUSB2'),
  );
  assert.deepEqual(diff, { added: ['/dev/ttyUSB1'], removed: [] });
});

test('a hotplug that renumbers the node reports both sides', () => {
  const diff = diffPorts(ports('/dev/ttyUSB20'), ports('/dev/ttyUSB24'));
  assert.deepEqual(diff, { added: ['/dev/ttyUSB24'], removed: ['/dev/ttyUSB20'] });
});

test('nothing changed means nothing is announced', () => {
  assert.equal(describePortChanges({ added: [], removed: [] }), null);
});

test('a single new port is named', () => {
  const notice = describePortChanges({ added: ['/dev/ttyUSB24'], removed: [] });
  assert.equal(notice?.kind, 'Success');
  assert.equal(notice?.title, 'Port connected');
  assert.match(notice!.body, /ttyUSB24/);
});

test('a first-enumeration-sized burst collapses to a count', () => {
  const many = Array.from({ length: 64 }, (_, i) => `/dev/ttyUSB${i}`);
  const notice = describePortChanges({ added: many, removed: [] });
  assert.equal(notice?.title, '64 ports connected');
  assert.match(notice!.body, /and 61 more/);
  assert.ok(notice!.body.length < 120);
});

test('a disappearing port is a warning, not good news', () => {
  const notice = describePortChanges({ added: [], removed: ['COM7'] });
  assert.equal(notice?.kind, 'Warning');
  assert.equal(notice?.title, 'Port disconnected');
  assert.match(notice!.body, /COM7/);
});

test('churn in both directions is reported as one neutral message', () => {
  const notice = describePortChanges({
    added: ['/dev/ttyUSB24'],
    removed: ['/dev/ttyUSB20'],
  });
  assert.equal(notice?.kind, 'Info');
  assert.equal(notice?.title, 'Port list changed');
  assert.match(notice!.body, /ttyUSB24/);
  assert.match(notice!.body, /ttyUSB20/);
});

test('names are summarized with the short display label', () => {
  assert.equal(summarizeNames(['/dev/ttyUSB0', 'COM3']), 'ttyUSB0, COM3');
  assert.equal(summarizeNames(['a', 'b', 'c', 'd'], 2), 'a, b and 2 more');
});
