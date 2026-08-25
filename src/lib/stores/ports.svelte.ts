import type { PortInfo } from '$lib/types';

export function createPortsStore() {
  let items = $state<PortInfo[]>([]);

  return {
    get items() { return items; },
    set(v: PortInfo[]) { items = v; },
    updatePort(name: string, changes: Partial<PortInfo>) {
      const idx = items.findIndex(p => p.name === name);
      if (idx >= 0) {
        items[idx] = { ...items[idx], ...changes };
      }
    },
    find(name: string): PortInfo | undefined {
      return items.find(p => p.name === name);
    }
  };
}

export const portsStore = createPortsStore();
