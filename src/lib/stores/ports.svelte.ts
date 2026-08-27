import type { PortInfo } from '$lib/types';

export function createPortsStore() {
  let items = $state<PortInfo[]>([]);
  let hasLoaded = $state(false);

  return {
    get items() { return items; },
    get hasLoaded() { return hasLoaded; },
    set(v: PortInfo[]) {
      items = v;
      hasLoaded = true;
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
