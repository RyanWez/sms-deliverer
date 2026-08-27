import type { SmsItem, QuickFilter, ViewMode } from '$lib/types';
import { portsStore } from './ports.svelte';
import { liveStore } from './live.svelte';
import { settingsStore } from './settings.svelte';
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
  let waveVersion = $state(0);

  function triggerWave() {
    waveVersion++;
  }

  let page = $state(1);
  let availH = $state(600);
  // Measured average height of a collapsed table row; drives 'auto' page size.
  let avgRowH = $state<number | null>(null);
  let expandedIds = $state<Set<number>>(new Set());
  let activeId = $state<number | null>(null);

  // Map port -> sim for fast lookup during filtering (avoids O(ports) find per message)
  const simLookup = $derived.by(() => {
    const m = new Map<string, string>();
    for (const p of portsStore.items) m.set(p.name, p.sim_number ?? '');
    return m;
  });

  // Single-pass aggregates: per-port counts and total OTP
  const aggregates = $derived.by(() => {
    const msgByPort = new Map<string, number>();
    const otpByPort = new Map<string, number>();
    let totalOtp = 0;
    for (const it of items) {
      msgByPort.set(it.message.port, (msgByPort.get(it.message.port) ?? 0) + 1);
      if (it.otp) {
        otpByPort.set(it.message.port, (otpByPort.get(it.message.port) ?? 0) + 1);
        totalOtp++;
      }
    }
    return { msgByPort, otpByPort, totalOtp };
  });

  const sorted = $derived.by(() =>
    [...items].sort((a, b) => {
      const ta = new Date(a.message.received || 0).getTime();
      const tb = new Date(b.message.received || 0).getTime();
      if (tb !== ta) return tb - ta;
      return b.id - a.id;
    })
  );

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const hasQuery = q.length > 0;
    const todayStart = (() => {
      const d = new Date();
      d.setHours(0, 0, 0, 0);
      return d.getTime();
    })();

    // Fast path: no filters at all
    if (!hasQuery && !portFilter && quickFilter === 'All') return sorted;

    return sorted.filter(m => {
      if (portFilter && m.message.port !== portFilter) return false;
      if (quickFilter === 'Otp' && !m.otp) return false;
      if (quickFilter === 'Today') {
        const d = new Date(m.message.received).getTime();
        if (Number.isNaN(d) || d < todayStart) return false;
      }
      if (hasQuery) {
        const sim = simLookup.get(m.message.port) ?? '';
        // build haystack only when needed; lowercasing once
        const haystack = `${m.message.from} ${m.message.text} ${m.message.port} ${m.otp ?? ''} ${sim}`.toLowerCase();
        if (!haystack.includes(q)) return false;
      }
      return true;
    });
  });

  const CARDS_PAGE_SIZE = 24;
  const TABLE_ROW_FALLBACK_H = 46;
  const MIN_PAGE_ROWS = 6;
  const MAX_PAGE_ROWS = 200;

  const rowsPerPage = $derived.by(() => {
    void viewMode;
    void availH;
    const cfg = settingsStore.settings.appearance.pageSize;
    if (typeof cfg === 'number') {
      return Math.max(4, Math.min(cfg, MAX_PAGE_ROWS));
    }
    if (viewMode === 'Cards') return CARDS_PAGE_SIZE;
    // Auto: fill the available viewport using the measured average row height,
    // with a floor so a bad measurement can never produce tiny pages.
    const rowH = Math.max(avgRowH ?? TABLE_ROW_FALLBACK_H, 30);
    const cap = Math.max(availH, 120);
    return Math.min(Math.max(Math.floor(cap / rowH), MIN_PAGE_ROWS), MAX_PAGE_ROWS);
  });

  // Deterministic, count-based paging: every page except the last is exactly
  // rowsPerPage items, so pages fill up instead of breaking early.
  const pages = $derived.by(() => {
    const out: SmsItem[][] = [];
    for (let i = 0; i < visible.length; i += rowsPerPage) {
      out.push(visible.slice(i, i + rowsPerPage));
    }
    return out.length > 0 ? out : [[]];
  });

  const totalPages = $derived(Math.max(1, pages.length));
  const safePage = $derived(Math.min(Math.max(1, page), totalPages));
  const pageRows = $derived(pages[safePage - 1] ?? []);
  const pageIndexStart = $derived((safePage - 1) * rowsPerPage);

  const selectedCount = $derived.by(() => selected.size);
  const otpCount = $derived(aggregates.totalOtp);
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

  function purgeExpired(retentionHours: number): number {
    if (retentionHours <= 0 || items.length === 0) return 0;
    const maxAgeMs = retentionHours * 3600 * 1000;
    const now = Date.now();
    const beforeCount = items.length;
    const purgedIds: number[] = [];

    items = items.filter(m => {
      const receivedTime = new Date(m.message.received).getTime();
      if (Number.isNaN(receivedTime)) return true;
      const isExpired = (now - receivedTime) > maxAgeMs;
      if (isExpired) {
        purgedIds.push(m.id);
        return false;
      }
      return true;
    });

    if (purgedIds.length > 0) {
      if (activeId !== null && purgedIds.includes(activeId)) activeId = null;
      const nextSel = new Set(selected);
      for (const id of purgedIds) nextSel.delete(id);
      selected = nextSel;
    }

    return beforeCount - items.length;
  }

  function goTo(p: number) {
    const next = Math.min(Math.max(1, p), totalPages);
    if (next !== page) {
      page = next;
      waveVersion++;
    }
  }

  function setAvail(h: number) {
    if (Math.abs(h - availH) >= 2) availH = h;
  }

  function reportHeights(entries: Array<{ id: number; h: number; expanded: boolean }>) {
    // Track the average collapsed-row height so 'auto' page size fills the
    // viewport with real measurements instead of guesses.
    const collapsed = entries.filter(e => !e.expanded && e.h > 0);
    if (collapsed.length === 0) return;
    const candidate =
      collapsed.reduce((s, e) => s + e.h, 0) / collapsed.length;
    if (avgRowH === null || Math.abs(candidate - avgRowH) >= 4) {
      avgRowH = Math.round(candidate);
    }
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

  function getMessageCountForPort(port: string): number {
    return aggregates.msgByPort.get(port) ?? 0;
  }
  function getOtpCountForPort(port: string): number {
    return aggregates.otpByPort.get(port) ?? 0;
  }

  return {
    get items() { return items; },
    set items(v: SmsItem[]) { items = v; },
    get selected() { return selected; },
    get query() { return query; },
    set query(v: string) { query = v; page = 1; waveVersion++; },
    get quickFilter() { return quickFilter; },
    set quickFilter(v: QuickFilter) { quickFilter = v; page = 1; waveVersion++; },
    get portFilter() { return portFilter; },
    set portFilter(v: string | null) { portFilter = v; page = 1; waveVersion++; },
    get viewMode() { return viewMode; },
    set viewMode(v: ViewMode) { viewMode = v; page = 1; waveVersion++; },
    get deleteBusy() { return deleteBusy; },
    set deleteBusy(v: boolean) { deleteBusy = v; },
    get visible() { return visible; },
    get pageRows() { return pageRows; },
    get totalPages() { return totalPages; },
    get page() { return safePage; },
    get pageIndexStart() { return pageIndexStart; },
    get waveVersion() { return waveVersion; },
    triggerWave,
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
    get countsByPort() { return aggregates; },
    getMessageCountForPort,
    getOtpCountForPort,
    isSelected,
    toggleSelected,
    selectAll,
    clearSelection,
    removeByIds,
    purgeExpired,
    prettyPort,
  };
}

export const messagesStore = createMessagesStore();
