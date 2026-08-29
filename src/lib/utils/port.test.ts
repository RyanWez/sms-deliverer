// Zero-dependency unit tests for the pure port helpers.
//
// Run with `npm test`. Node's built-in test runner and type stripping are
// used deliberately: the obvious alternative (vitest) currently pulls a
// critical-severity advisory chain in through this project's vite 5 pin, and
// these are plain functions that need no DOM or component harness.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { portLabel, portNum, portStatus } from './port.ts';
import type { PortInfo } from '../types.ts';

function port(over: Partial<PortInfo> = {}): PortInfo {
  return {
    name: '/dev/ttyUSB20',
    path: 'pci-0000:03:00.3-usb-0:4.1:1.0-port0',
    checked: true,
    sim_number: '',
    alive: null,
    live_ready: false,
    live_error: null,
    ...over,
  };
}

test('portLabel keeps COM names and shortens Linux device paths', () => {
  assert.equal(portLabel('COM7'), 'COM7');
  // Relabelling this "COM20" is what made the UI, the log file and the JSON
  // export each call the same port something different.
  assert.equal(portLabel('/dev/ttyUSB20'), 'ttyUSB20');
  assert.equal(portLabel('/dev/ttyACM0'), 'ttyACM0');
  assert.equal(portLabel('ttyUSB5'), 'ttyUSB5');
});

test('portNum reads the trailing digits', () => {
  assert.equal(portNum('/dev/ttyUSB64'), 64);
  assert.equal(portNum('COM3'), 3);
  assert.equal(portNum('no-digits'), 0);
});

test('a probed-silent port reads as NO MODEM, not an error', () => {
  const st = portStatus(port({ alive: false, live_error: 'Modem not responding' }), true);
  assert.equal(st.key, 'no-modem');
  assert.equal(st.label, 'NO MODEM');
  assert.equal(st.badge, 'badge-muted');
});

test('no-modem outranks live_ready so an empty slot never shows green', () => {
  // The exact shape of the bug: live mode used to announce Ready for a port
  // that had never answered an AT command.
  const st = portStatus(port({ alive: false, live_ready: true }), true);
  assert.equal(st.key, 'no-modem');
});

test('a real fault on a live port still reads as ERROR', () => {
  const st = portStatus(port({ alive: true, live_error: 'Port lost: ENODEV' }), true);
  assert.equal(st.key, 'error');
  assert.equal(st.label, 'ERROR');
});

test('an answering port that is monitored reads as LIVE', () => {
  const st = portStatus(port({ alive: true, live_ready: true }), true);
  assert.equal(st.key, 'live');
});

test('checked ports are CONNECTING while live is starting and READY when idle', () => {
  assert.equal(portStatus(port({ alive: true }), true).key, 'connecting');
  assert.equal(portStatus(port({ alive: true }), false).key, 'ready');
});

test('unchecked ports read as DISABLED', () => {
  assert.equal(portStatus(port({ checked: false }), false).key, 'disabled');
});

test('never-probed ports keep the old behaviour', () => {
  // alive === null must not be mistaken for "no modem", or a fresh launch would
  // paint the whole bank grey before anything has been probed.
  assert.equal(portStatus(port({ alive: null, checked: true }), false).key, 'ready');
});
