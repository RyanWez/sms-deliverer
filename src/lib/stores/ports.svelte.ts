import type { PortInfo } from '$lib/types';

/**
 * How long a freshly detected port keeps its NEW marker.
 *
 * The toast is gone after 4s, which is easy to miss on an unattended machine —
 * long enough that an operator who plugs a stick in and then walks over to the
 * screen still sees which card is the new one, short enough that it is not
 * mistaken for a permanent state.
 */
const NEW_PORT_MARKER_MS = 60_000;

export function createPortsStore() {
  let items = $state<PortInfo[]>([]);
  let hasLoaded = $state(false);
  let recentlyAdded = $state<string[]>([]);

  return {
    get items() { return items; },
    get hasLoaded() { return hasLoaded; },
    /** Port names detected within the last {@link NEW_PORT_MARKER_MS}. */
    get recentlyAdded() { return recentlyAdded; },
    set(v: PortInfo[]) {
      items = v;
      hasLoaded = true;
      // A marked port that has since vanished must not keep its marker: the
      // snapshot is authoritative about what exists.
      if (recentlyAdded.length > 0) {
        const present = new Set(v.map(p => p.name));
        const kept = recentlyAdded.filter(n => present.has(n));
        if (kept.length !== recentlyAdded.length) recentlyAdded = kept;
      }
    },
    /** Flag ports the last refresh discovered so the Ports page can point at them. */
    markRecentlyAdded(names: string[]) {
      if (names.length === 0) return;
      recentlyAdded = [...new Set([...recentlyAdded, ...names])];
      setTimeout(() => {
        recentlyAdded = recentlyAdded.filter(n => !names.includes(n));
      }, NEW_PORT_MARKER_MS);
    },
    isRecentlyAdded(name: string): boolean {
      return recentlyAdded.includes(name);
    },
    setCheckedAll(checked: boolean) {
      items = items.map(p => (p.checked === checked ? p : { ...p, checked }));
    },
    updatePort(name: string, changes: Partial<PortInfo>) {
      const idx = items.findIndex(p => p.name === name);
      if (idx >= 0) {
        items[idx] = { ...items[idx], ...changes };
      }
    },
    batchUpdatePorts(updates: Array<{ name: string; changes: Partial<PortInfo> }>) {
      if (updates.length === 0) return;
      const map = new Map(updates.map(u => [u.name, u.changes]));
      items = items.map(p => {
        const ch = map.get(p.name);
        return ch ? { ...p, ...ch } : p;
      });
    },
    resetLive() {
      items = items.map(p => ({ ...p, live_ready: false, live_error: null }));
    },
    find(name: string): PortInfo | undefined {
      return items.find(p => p.name === name);
    },
  };
}

export const portsStore = createPortsStore();
