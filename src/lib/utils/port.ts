import type { PortInfo } from '$lib/types';

export function portNum(name: string): number {
  const m = name.match(/(\d+)$/);
  return m ? parseInt(m[1], 10) : 0;
}

/**
 * Short display name for a port.
 *
 * Windows `COM*` names are already short. Linux device paths are shortened to
 * the device node only (`/dev/ttyUSB20` → `ttyUSB20`) — they used to be
 * relabelled `COM20`, which meant the UI, the log file and the JSON export each
 * called the same port something different.
 */
export function portLabel(name: string): string {
  if (name.startsWith('COM')) return name;
  const node = name.split('/').pop();
  return node && node.length > 0 ? node : name;
}

export type PortStatusKey = 'error' | 'live' | 'connecting' | 'ready' | 'no-modem' | 'disabled';

/**
 * Does this port have a phone number worth acting on?
 *
 * A slot that was probed and stayed silent has no modem in it, so a number left
 * over from whatever card used to be there is not something to select, filter on
 * or count. Counting those is what made the With SIM badge read 34 on a bank
 * holding 32 modems.
 */
export function hasValidSim(p: PortInfo): boolean {
  if (p.alive === false) return false;
  const n = p.sim_number?.trim();
  return Boolean(n && n !== '-' && n !== 'Unknown');
}

export interface PortStatus {
  key: PortStatusKey;
  label: string;
  badge: string;
  tile: string;
}

export function portStatus(p: PortInfo, liveOn: boolean): PortStatus {
  // A probed-silent port is the normal state of an empty SIM slot, not a
  // failure, so it gets its own muted styling ahead of the error branch —
  // otherwise every unused slot in the bank screams red.
  if (p.alive === false) {
    return {
      key: 'no-modem',
      label: 'NO MODEM',
      badge: 'badge-muted',
      tile: 'bg-elevated text-muted-foreground',
    };
  }
  if (p.live_error) {
    return { key: 'error', label: 'ERROR', badge: 'badge-danger', tile: 'bg-danger/10 text-danger' };
  }
  if (p.live_ready) {
    return { key: 'live', label: 'LIVE', badge: 'badge-success', tile: 'bg-success/10 text-success' };
  }
  if (liveOn && p.checked) {
    return { key: 'connecting', label: 'CONNECTING', badge: 'badge-warning', tile: 'bg-warning/10 text-warning' };
  }
  if (p.checked) {
    return { key: 'ready', label: 'READY', badge: 'badge-primary', tile: 'bg-primary/10 text-primary' };
  }
  return { key: 'disabled', label: 'DISABLED', badge: 'badge-muted', tile: 'bg-elevated text-muted-foreground' };
}

