import type { SmsItem, QuickFilter, ViewMode } from '$lib/types';
import { portsStore } from './ports.svelte';

export function createMessagesStore() {
  let items = $state<SmsItem[]>([]);
  let selected = $state<Set<number>>(new Set());
  let query = $state('');
  let quickFilter = $state<QuickFilter>('All');
  let portFilter = $state<string | null>(null);
  let viewMode = $state<ViewMode>('Table');
  let deleteBusy = $state(false);

  const visible = $derived.by(() => {
    const q = query.toLowerCase();
    const today = new Date();
    today.setHours(0, 0, 0, 0);

    return items.filter(m => {
      if (portFilter && m.message.port !== portFilter) return false;
      if (quickFilter === 'Otp' && !m.otp) return false;
      if (quickFilter === 'Today') {
        const d = new Date(m.message.received);
        if (d < today) return false;
      }
      if (q) {
        const haystack = `${m.message.from} ${m.message.text} ${m.message.port} ${m.otp ?? ''}`.toLowerCase();
        if (!haystack.includes(q)) return false;
      }
      return true;
    });
  });

  const selectedCount = $derived.by(() => selected.size);
  const otpCount = $derived.by(() => items.filter(m => m.otp).length);

  function isSelected(id: number) { return selected.has(id); }

  function toggleSelected(id: number) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id); else next.add(id);
    selected = next;
  }

  function selectAll(ids: number[]) {
    selected = new Set(ids);
  }

  function clearSelection() {
    selected = new Set();
  }

  function removeByIds(ids: number[]) {
    const idSet = new Set(ids);
    items = items.filter(m => !idSet.has(m.id));
    const next = new Set(selected);
    for (const id of ids) next.delete(id);
    selected = next;
  }

  function prettyPort(name: string): string {
    const p = portsStore.find(name);
    if (p?.sim_number) return p.sim_number;
    const num = name.replace(/^(COM|ttyUSB|ttyACM)/, '');
    return num ? `Port ${num}` : name;
  }

  return {
    get items() { return items; },
    set items(v: SmsItem[]) { items = v; },
    get selected() { return selected; },
    get query() { return query; },
    set query(v: string) { query = v; },
    get quickFilter() { return quickFilter; },
    set quickFilter(v: QuickFilter) { quickFilter = v; },
    get portFilter() { return portFilter; },
    set portFilter(v: string | null) { portFilter = v; },
    get viewMode() { return viewMode; },
    set viewMode(v: ViewMode) { viewMode = v; },
    get deleteBusy() { return deleteBusy; },
    set deleteBusy(v: boolean) { deleteBusy = v; },
    get visible() { return visible; },
    get selectedCount() { return selectedCount; },
    get otpCount() { return otpCount; },
    isSelected,
    toggleSelected,
    selectAll,
    clearSelection,
    removeByIds,
    prettyPort,
  };
}

export const messagesStore = createMessagesStore();
