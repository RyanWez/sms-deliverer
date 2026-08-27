import type { LogEntry, LogLevelFilter } from '$lib/types/logs';
import { liveStore } from '$lib/stores/live.svelte';

let nextToastId = 1000;

function toast(kind: 'Success' | 'Info' | 'Warning' | 'Danger' | 'Otp', title: string, body: string) {
  liveStore.addToast({
    id: nextToastId++,
    kind,
    title,
    body,
    otp: null,
  });
}

export function createLogsStore() {
  let items = $state<LogEntry[]>([]);
  let filterLevel = $state<LogLevelFilter>('ALL');
  let searchQuery = $state('');
  let isStreaming = $state(true);
  let autoScroll = $state(true);

  const counts = $derived.by(() => {
    let error = 0;
    let warn = 0;
    let info = 0;
    let debug = 0;
    for (const item of items) {
      const lvl = item.level.toUpperCase();
      if (lvl === 'ERROR') error++;
      else if (lvl === 'WARN') warn++;
      else if (lvl === 'INFO') info++;
      else if (lvl === 'DEBUG' || lvl === 'TRACE') debug++;
    }
    return {
      all: items.length,
      error,
      warn,
      info,
      debug,
    };
  });

  const filtered = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    return items.filter((item) => {
      // Level filter
      if (filterLevel !== 'ALL') {
        const lvl = item.level.toUpperCase();
        if (filterLevel === 'DEBUG') {
          if (lvl !== 'DEBUG' && lvl !== 'TRACE') return false;
        } else if (lvl !== filterLevel) {
          return false;
        }
      }
      // Search filter
      if (q) {
        const haystack = `${item.timestamp} ${item.level} ${item.target} ${item.message}`.toLowerCase();
        if (!haystack.includes(q)) return false;
      }
      return true;
    });
  });

  return {
    get items() { return items; },
    get filtered() { return filtered; },
    get counts() { return counts; },
    get filterLevel() { return filterLevel; },
    set filterLevel(v: LogLevelFilter) { filterLevel = v; },
    get searchQuery() { return searchQuery; },
    set searchQuery(v: string) { searchQuery = v; },
    get isStreaming() { return isStreaming; },
    set isStreaming(v: boolean) { isStreaming = v; },
    get autoScroll() { return autoScroll; },
    set autoScroll(v: boolean) { autoScroll = v; },

    addEntry(entry: LogEntry) {
      if (!isStreaming) return;
      // Cap in-memory logs to latest 1000 items
      if (items.length >= 1000) {
        items = [...items.slice(items.length - 999), entry];
      } else {
        items = [...items, entry];
      }
    },

    setLogs(newLogs: LogEntry[]) {
      items = newLogs.slice(-1000);
    },

    clear() {
      items = [];
      toast('Info', 'Logs Cleared', 'Console display cleared.');
    },

    async copyAll() {
      if (filtered.length === 0) {
        toast('Info', 'Copy Logs', 'No logs matching current filter to copy.');
        return;
      }
      const text = filtered
        .map((e) => `[${e.timestamp}] [${e.level.padEnd(5)}] [${e.target}] ${e.message}`)
        .join('\n');
      try {
        await navigator.clipboard.writeText(text);
        toast('Success', 'Logs Copied', `${filtered.length} log line(s) copied to clipboard.`);
      } catch (err) {
        toast('Danger', 'Copy Failed', String(err));
      }
    },

    async exportLogs() {
      if (filtered.length === 0) {
        toast('Info', 'Export Logs', 'No logs to export.');
        return;
      }
      const text = filtered
        .map((e) => `[${e.timestamp}] [${e.level.padEnd(5)}] [${e.target}] ${e.message}`)
        .join('\n');

      const blob = new Blob([text], { type: 'text/plain;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      const filename = `sms-reader-logs-${new Date().toISOString().replace(/[:.]/g, '-')}.log`;
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast('Success', 'Export Complete', `Saved ${filename}`);
    },
  };
}

export const logsStore = createLogsStore();
