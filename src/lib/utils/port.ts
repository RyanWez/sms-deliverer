import type { PortInfo } from '$lib/types';

export function portNum(name: string): number {
  const m = name.match(/(\d+)$/);
  return m ? parseInt(m[1], 10) : 0;
}

export function portLabel(name: string): string {
  if (name.startsWith('COM')) return name;
  return `COM${portNum(name)}`;
}

export type PortStatusKey = 'error' | 'live' | 'connecting' | 'ready' | 'disabled';

export interface PortStatus {
  key: PortStatusKey;
  label: string;
  badge: string;
  tile: string;
}

export function portStatus(p: PortInfo, liveOn: boolean): PortStatus {
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

