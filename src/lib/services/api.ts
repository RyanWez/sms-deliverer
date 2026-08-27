import { isTauri } from '$lib/utils/tauri';
import { portsStore } from '$lib/stores/ports.svelte';
import { messagesStore } from '$lib/stores/messages.svelte';
import { liveStore } from '$lib/stores/live.svelte';
import { settingsStore } from '$lib/stores/settings.svelte';
import type { SmsItem, ToastData, PortInfo } from '$lib/types';

let nextToastId = 1;
let initialized = false;

function toast(kind: ToastData['kind'], title: string, body: string, otp?: string | null) {
  liveStore.addToast({
    id: nextToastId++,
    kind,
    title,
    body,
    otp: otp ?? null,
  });
}

async function refreshFromBackend() {
  if (!isTauri()) return;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const msgs = await invoke<SmsItem[]>('get_messages');
    messagesStore.items = msgs;
    const ports = await invoke<PortInfo[]>('get_ports');
    portsStore.set(ports);
    liveStore.statusText = await invoke<string>('get_status_text');
  } catch {
    // ignore transient backend errors
  }
}

export const api = {
  async init() {
    if (initialized) return;
    initialized = true;

    if (!isTauri()) {
      console.info('[api] Running in web browser preview mode. Initializing demo synthetic data.');
      const { injectSyntheticData } = await import('$lib/utils/synthetic');
      await injectSyntheticData(16, 35);
      liveStore.statusText = 'Browser Preview Mode';
      return;
    }

    await refreshFromBackend();

    try {
      const { listen } = await import('@tauri-apps/api/event');

      await listen('messages:reset', () => {
        messagesStore.items = [];
        messagesStore.clearSelection();
      });

      await listen<{ items: SmsItem[] }>('messages:added', (event) => {
        const existing = new Set(messagesStore.items.map((m) => m.id));
        const incoming = event.payload.items.filter((i) => !existing.has(i.id));
        if (incoming.length > 0) {
          messagesStore.items = [...messagesStore.items, ...incoming];
          messagesStore.triggerWave();
        }
      });

      await listen<{ ids: number[] }>('messages:removed', (event) => {
        messagesStore.removeByIds(event.payload.ids);
        messagesStore.deleteBusy = false;
      });

      await listen('delete:done', () => {
        messagesStore.deleteBusy = false;
      });

      await listen<{ ports: PortInfo[] }>('ports:updated', (event) => {
        portsStore.set(event.payload.ports);
      });

      await listen<{ text: string }>('status:update', (event) => {
        liveStore.statusText = event.payload.text;
      });

      await listen<SmsItem>('sms:new', (event) => {
        const item = event.payload as any;
        const smsItem: SmsItem = {
          id: item.id,
          message: item.message ?? item,
          otp: item.otp ?? null,
          is_new: true,
        };
        if (!messagesStore.items.some(m => m.id === smsItem.id)) {
          messagesStore.items = [...messagesStore.items, smsItem];
        }
        if (smsItem.otp) {
          toast('Otp', 'New OTP', smsItem.message.text ?? '', smsItem.otp);
        }
      });

      await listen<{ port: string }>('sms:ready', (event) => {
        portsStore.updatePort(event.payload.port, { live_ready: true });
        liveStore.readyPorts = portsStore.items
          .filter(p => p.live_ready)
          .map(p => p.name);
      });

      await listen('scan:done', () => {
        liveStore.scanBusy = false;
        messagesStore.triggerWave();
      });

      await listen('ussd:done', () => {
        liveStore.ussdBusy = false;
      });

      await listen<{ path: string }>('export:saved', (event) => {
        toast('Success', 'Export complete', event.payload.path);
      });

      await listen<{ error: string }>('export:failed', (event) => {
        toast('Danger', 'Export failed', event.payload.error);
      });

      if (settingsStore.general.autoStartLive && portsStore.items.some(p => p.checked)) {
        void api.startLive(true);
      }
    } catch (e) {
      console.warn('[api] Failed to setup Tauri event listeners:', e);
    }
  },

  async refreshPorts() {
    if (!isTauri()) {
      const { generateSyntheticPorts } = await import('$lib/utils/synthetic');
      portsStore.set(generateSyntheticPorts(16));
      toast('Success', 'Refreshed', 'Synthetic ports refreshed.');
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const ports = await invoke<PortInfo[]>('refresh_ports');
      portsStore.set(ports);
    } catch (e) {
      toast('Danger', 'Refresh failed', String(e));
    }
  },

  async startScan() {
    if (!isTauri()) {
      liveStore.scanBusy = true;
      toast('Info', 'Scan Simulation', 'Scanning simulated ports...');
      const { generateSyntheticMessages } = await import('$lib/utils/synthetic');
      messagesStore.items = [];
      setTimeout(() => {
        const msgs = generateSyntheticMessages(18, portsStore.items);
        messagesStore.items = msgs;
        messagesStore.triggerWave();
        liveStore.scanBusy = false;
        toast('Success', 'Scan Complete', `Found ${msgs.length} messages`);
      }, 700);
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      liveStore.scanBusy = true;
      await invoke('start_scan');
    } catch (e) {
      liveStore.scanBusy = false;
      toast('Danger', 'Scan failed', String(e));
    }
  },

  async startLive(silent = false) {
    if (!isTauri()) {
      liveStore.on = true;
      liveStore.totalPorts = portsStore.items.filter(p => p.checked).length;
      liveStore.readyPorts = portsStore.items.filter(p => p.checked).map(p => p.name);
      if (!silent) toast('Success', 'Live Started', 'Simulated live mode active');
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const ports = await invoke<PortInfo[]>('get_ports');
      portsStore.set(ports);
      portsStore.resetLive();
      const checked = ports.filter(p => p.checked);
      liveStore.on = true;
      liveStore.totalPorts = checked.length;
      liveStore.readyPorts = [];
      await invoke('start_live');
    } catch (e) {
      liveStore.on = false;
      liveStore.totalPorts = 0;
      liveStore.readyPorts = [];
      portsStore.resetLive();
      if (!silent) toast('Danger', 'Live failed', String(e));
    }
  },

  async stopLive() {
    if (!isTauri()) {
      liveStore.on = false;
      liveStore.totalPorts = 0;
      liveStore.readyPorts = [];
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('stop_live');
    } finally {
      liveStore.on = false;
      liveStore.totalPorts = 0;
      liveStore.readyPorts = [];
      portsStore.resetLive();
    }
  },

  async getSimNumbers() {
    if (!isTauri()) {
      toast('Info', 'USSD Simulation', 'Simulating USSD SIM query');
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      liveStore.ussdBusy = true;
      await invoke('get_sim_numbers');
    } catch (e) {
      liveStore.ussdBusy = false;
      toast('Warning', 'USSD failed', String(e));
    }
  },

  async deleteSelected(ids: number[]) {
    if (ids.length === 0) return;
    if (messagesStore.deleteBusy) return;
    messagesStore.deleteBusy = true;
    try {
      if (isTauri()) {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('delete_selected', { ids });
      } else {
        setTimeout(() => {
          messagesStore.removeByIds(ids);
          messagesStore.deleteBusy = false;
        }, 800);
      }
    } catch (e) {
      messagesStore.deleteBusy = false;
      toast('Danger', 'Delete failed', String(e));
    }
  },

  async clearAll() {
    try {
      if (isTauri()) {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('clear_all');
      }
    } finally {
      messagesStore.items = [];
      messagesStore.clearSelection();
    }
  },

  async togglePortChecked(port: string, checked: boolean) {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('toggle_port_checked', { port, checked });
    } else {
      portsStore.updatePort(port, { checked });
    }
  },

  async setAllPortsChecked(checked: boolean) {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_all_ports_checked', { checked });
    } else {
      portsStore.setCheckedAll(checked);
    }
  },

  /**
   * Export the currently filtered message list to a user-chosen file.
   * In Tauri the native save dialog + write run on the Rust side; in browser
   * preview we fall back to a Blob download.
   */
  async exportMessages(format: 'csv' | 'json') {
    const rows = messagesStore.visible.map((it) => ({
      time: it.message.received || '',
      from: it.message.from || '',
      port: it.message.port || '',
      sim: messagesStore.prettyPort(it.message.port),
      text: it.message.text || '',
      otp: it.otp ?? '',
      status: it.message.status || '',
    }));
    const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
    const fileName = `sms-export-${stamp}.${format}`;

    let contents: string;
    if (format === 'json') {
      contents = JSON.stringify(rows, null, 2);
    } else {
      contents = toCsv(rows);
    }

    if (isTauri()) {
      toast('Info', 'Export', 'Choose a location to save the file…');
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('export_messages', { contents, suggested: fileName });
      } catch (e) {
        toast('Danger', 'Export failed', String(e));
      }
      return;
    }

    // Browser preview — plain download.
    const url = URL.createObjectURL(new Blob([contents], { type: 'text/plain' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = fileName;
    a.click();
    URL.revokeObjectURL(url);
    toast('Success', 'Export complete', fileName);
  },
};

/** Serialize smoke-report style rows to CSV with RFC-4180 escaping. */
function csvCell(v: string): string {
  if (/[",\n\r]/.test(v)) return `"${v.replace(/"/g, '""')}"`;
  return v;
}

function toCsv(rows: Array<Record<string, string>>): string {
  if (rows.length === 0) return 'time,from,port,sim,text,otp,status\n';
  const header = Object.keys(rows[0]).join(',');
  const body = rows
    .map((r) => Object.values(r).map((v) => csvCell(String(v))).join(','))
    .join('\n');
  return `${header}\n${body}\n`;
}
