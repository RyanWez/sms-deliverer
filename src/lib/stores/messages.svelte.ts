import type { SmsItem, QuickFilter, ViewMode } from '$lib/types';
import { portsStore } from './ports.svelte';
import { liveStore } from './live.svelte';
import { portLabel } from '$lib/utils/port';

let toastCounter = 10000;

export function createMessagesStore() {
  let items = $state<SmsItem[]>([]);
  let selected = $state<Set<number>>(new Set());
  let query = $state('');
  let quickFilter = $state<QuickFilter>('All');
  let portFilter = $state<string | null>(null);
  let viewMode = $state<ViewMode>('Table');
  let deleteBusy = $state(false);

  let page = $state(1);
  let availH = $state(600);
  let measureTick = $state(0);
  let expandedIds = $state<Set<number>>(new Set());
  let activeId = $state<number | null>(null);
  const hCollapsed = new Map<string, number>();
  const hExpanded = new Map<string, number>();

  const sorted = $derived.by(() =>
    [...items].sort((a, b) => {
      const ta = new Date(a.message.received || 0).getTime();
      const tb = new Date(b.message.received || 0).getTime();
      if (tb !== ta) return tb - ta;
      return b.id - a.id;
    })
  );

  const visible = $derived.by(() => {
    const q = query.toLowerCase();
    const today = new Date();
    today.setHours(0, 0, 0, 0);

    return sorted.filter(m => {
      if (portFilter && m.message.port !== portFilter) return false;
      if (quickFilter === 'Otp' && !m.otp) return false;
      if (quickFilter === 'Today') {
        const d = new Date(m.message.received);
        if (d < today) return false;
      }
      if (q) {
        const sim = portsStore.find(m.message.port)?.sim_number ?? '';
        const haystack = `${m.message.from} ${m.message.text} ${m.message.port} ${m.otp ?? ''} ${sim}`.toLowerCase();
        if (!haystack.includes(q)) return false;
      }
      return true;
    });
  });

  const defaultRowH = $derived(viewMode === 'Cards' ? 118 : 57);

  function cacheKey(id: number, expanded: boolean): string {
    return `${viewMode === 'Cards' ? 'c' : 't'}${expanded ? 'e' : 'n'}${id}`;
  }

  function estHeight(m: SmsItem): number {
    if (expandedIds.has(m.id)) {
      const known = hExpanded.get(cacheKey(m.id, true));
      if (known) return known;
      const lines = Math.max(1, Math.ceil(m.message.text.length / 90));
      return defaultRowH + lines * 17 + 8;
    }
    return hCollapsed.get(cacheKey(m.id, false)) ?? defaultRowH;
  }

  const pages = $derived.by(() => {
    void availH;
    void measureTick;
    void expandedIds.size;
    void viewMode;
    const cap = Math.max(availH, 120);
    const out: SmsItem[][] = [];
    let cur: SmsItem[] = [];
    let used = 0;
    for (const m of visible) {
      const h = estHeight(m);
      if (cur.length > 0 && used + h > cap) {
        out.push(cur);
        cur = [];
        used = 0;
      }
      cur.push(m);
      used += h;
    }
    if (cur.length > 0) out.push(cur);
    return out.length > 0 ? out : [[]];
  });

  const totalPages = $derived(Math.max(1, pages.length));
  const safePage = $derived(Math.min(Math.max(1, page), totalPages));
  const pageRows = $derived(pages[safePage - 1] ?? []);
  const pageIndexStart = $derived.by(() => {
    let n = 0;
    for (let i = 0; i < safePage - 1 && i < pages.length; i++) n += pages[i].length;
    return n;
  });

  const selectedCount = $derived.by(() => selected.size);
  const otpCount = $derived.by(() => items.filter(m => m.otp).length);
  const activeItem = $derived(items.find(m => m.id === activeId) ?? null);

  function isActive(id: number) { return activeId === id; }

  function setActive(id: number | null) {
    activeId = id;
    if (id === null) return;
    const idx = items.findIndex(m => m.id === id);
    if (idx >= 0 && items[idx].is_new) {
      items[idx] = { ...items[idx], is_new: false };
    }
  }

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
    if (activeId !== null && idSet.has(activeId)) activeId = null;
    const next = new Set(selected);
    for (const id of ids) next.delete(id);
    selected = next;
  }

  function goTo(p: number) {
    page = Math.min(Math.max(1, p), totalPages);
  }

  function setAvail(h: number) {
    if (Math.abs(h - availH) >= 2) availH = h;
  }

  function reportHeights(entries: Array<{ id: number; h: number; expanded: boolean }>) {
    let changed = false;
    for (const e of entries) {
      if (e.h <= 0) continue;
      const key = cacheKey(e.id, e.expanded);
      const cache = e.expanded ? hExpanded : hCollapsed;
      if (cache.get(key) !== e.h) {
        cache.set(key, e.h);
        changed = true;
      }
    }
    if (changed) measureTick++;
  }

  function isExpanded(id: number) { return expandedIds.has(id); }

  function toggleExpanded(id: number) {
    const next = new Set(expandedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    expandedIds = next;
  }

  function copyOtp(otp: string | null) {
    if (!otp) return;
    navigator.clipboard.writeText(otp);
    liveStore.addToast({
      id: ++toastCounter,
      kind: 'Success',
      title: 'Copied',
      body: `OTP ${otp} copied to clipboard`,
      otp: null,
    });
  }

  function copyMessage(text: string) {
    navigator.clipboard.writeText(text);
    liveStore.addToast({
      id: ++toastCounter,
      kind: 'Success',
      title: 'Copied',
      body: 'Message text copied to clipboard',
      otp: null,
    });
  }

  function prettyPort(name: string): string {
    const p = portsStore.find(name);
    if (p?.sim_number) return p.sim_number;
    return portLabel(name);
  }

  return {
    get items() { return items; },
    set items(v: SmsItem[]) { items = v; },
    get selected() { return selected; },
    get query() { return query; },
    set query(v: string) { query = v; page = 1; },
    get quickFilter() { return quickFilter; },
    set quickFilter(v: QuickFilter) { quickFilter = v; page = 1; },
    get portFilter() { return portFilter; },
    set portFilter(v: string | null) { portFilter = v; page = 1; },
    get viewMode() { return viewMode; },
    set viewMode(v: ViewMode) { viewMode = v; page = 1; },
    get deleteBusy() { return deleteBusy; },
    set deleteBusy(v: boolean) { deleteBusy = v; },
    get visible() { return visible; },
    get pageRows() { return pageRows; },
    get totalPages() { return totalPages; },
    get page() { return safePage; },
    get pageIndexStart() { return pageIndexStart; },
    goTo,
    setAvail,
    reportHeights,
    isExpanded,
    toggleExpanded,
    copyOtp,
    copyMessage,
    get activeItem() { return activeItem; },
    isActive,
    setActive,
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
