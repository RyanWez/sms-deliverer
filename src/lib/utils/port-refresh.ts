// Pure helpers for the background port re-enumeration (Settings → General →
// Port Refresh Interval) and for reporting what a refresh changed.
//
// Kept free of runes and of Tauri imports so `npm test` can exercise it: the
// scheduling decision and the topology diff are the two parts worth pinning
// down, and both are plain data in / plain data out.

import type { PortInfo, ToastData } from '$lib/types';
import { portLabel } from './port.ts';

/**
 * Bounds for the refresh period, in seconds.
 *
 * The floor exists because every tick enumerates the serial devices of a bank
 * that can hold ~64 sticks; anything faster than a few seconds buys nothing and
 * keeps the enumeration permanently warm. The ceiling is what stops a corrupt
 * stored value from reaching `setInterval` as a delay above 2^31-1 ms, which
 * overflows to a near-zero delay — the exact tight loop the floor is there to
 * prevent. Mirrors the 1–168 hour clamp in `services/updater.ts`.
 */
export const MIN_PORT_REFRESH_SECONDS = 5;
export const MAX_PORT_REFRESH_SECONDS = 3600;

/**
 * Turn the stored `general.portRefreshInterval` into a `setInterval` delay.
 *
 * Returns `null` for "disabled", which covers the user's own 0 as well as the
 * junk an older profile or a hand-edited localStorage entry can hold (negative,
 * `NaN`, a string, `undefined`). The Settings number input only applies
 * `min`/`max` to the DOM element, so the value arriving here is not trustworthy.
 */
export function portRefreshPeriodMs(seconds: unknown): number | null {
  const n = typeof seconds === 'number' ? seconds : Number(seconds);
  if (!Number.isFinite(n) || n <= 0) return null;
  const clamped = Math.min(
    MAX_PORT_REFRESH_SECONDS,
    Math.max(MIN_PORT_REFRESH_SECONDS, Math.floor(n)),
  );
  return clamped * 1000;
}

/** Port names that came and went between two enumerations. */
export interface PortDiff {
  added: string[];
  removed: string[];
}

/** The identity a diff is keyed on — the OS device name. */
type PortKey = Pick<PortInfo, 'name'>;

/**
 * Compare two port snapshots by device name.
 *
 * Names, not array indices: the backend orders the list by port number, so a
 * stick appearing low in the numbering shifts every later index. Names, not
 * `path` either: a re-plugged stick usually lands on a new tty node under the
 * same USB path, and that renumbering is precisely what the operator needs
 * told, because every card, log line and export refers to the node.
 */
export function diffPorts(prev: readonly PortKey[], next: readonly PortKey[]): PortDiff {
  const before = new Set(prev.map((p) => p.name));
  const after = new Set(next.map((p) => p.name));
  return {
    added: [...after].filter((n) => !before.has(n)),
    removed: [...before].filter((n) => !after.has(n)),
  };
}

/** `a, b, c and 61 more` — a bank of 64 must not spell itself out. */
export function summarizeNames(names: readonly string[], max = 3): string {
  const labels = names.map(portLabel);
  if (labels.length <= max) return labels.join(', ');
  return `${labels.slice(0, max).join(', ')} and ${labels.length - max} more`;
}

export interface PortChangeNotice {
  kind: ToastData['kind'];
  title: string;
  body: string;
}

/**
 * Describe a diff for the toast layer, or `null` when there is nothing to say.
 *
 * Silence is the expected outcome: this runs on a timer, and an unchanged bank
 * is the normal case. A port arriving is good news, a port vanishing mid-shift
 * is a warning, and both at once is the hotplug renumbering case.
 */
export function describePortChanges(diff: PortDiff): PortChangeNotice | null {
  const added = diff.added.length;
  const removed = diff.removed.length;
  if (added === 0 && removed === 0) return null;

  if (removed === 0) {
    return {
      kind: 'Success',
      title: added === 1 ? 'Port connected' : `${added} ports connected`,
      body: `Added to the port list: ${summarizeNames(diff.added)}.`,
    };
  }
  if (added === 0) {
    return {
      kind: 'Warning',
      title: removed === 1 ? 'Port disconnected' : `${removed} ports disconnected`,
      body: `No longer present: ${summarizeNames(diff.removed)}.`,
    };
  }
  return {
    kind: 'Info',
    title: 'Port list changed',
    body:
      `Connected: ${summarizeNames(diff.added)}. ` +
      `Disconnected: ${summarizeNames(diff.removed)}.`,
  };
}
