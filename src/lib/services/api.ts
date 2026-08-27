import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
  try {
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

    await refreshFromBackend();

    await listen('messages:reset', () => {
      messagesStore.items = [];
      messagesStore.clearSelection();
    });

    await listen<{ items: SmsItem[] }>('messages:added', (event) => {
      const existing = new Set(messagesStore.items.map((m) => m.id));
      const incoming = event.payload.items.filter((i) => !existing.has(i.id));
      if (incoming.length > 0) {
        messagesStore.items = [...messagesStore.items, ...incoming];
      }
    });

    await listen<{ ids: number[] }>('messages:removed', (event) => {
      messagesStore.removeByIds(event.payload.ids);
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

    if (settingsStore.general.autoStartLive && portsStore.items.some(p => p.checked)) {
      void api.startLive(true);
    }
  },

  async refreshPorts() {
    const ports = await invoke<PortInfo[]>('refresh_ports');
    portsStore.set(ports);
  },

  async startScan() {
    try {
      await invoke('start_scan');
    } catch (e) {
      toast('Danger', 'Scan failed', String(e));
    }
  },

  async startLive(silent = false) {
    try {
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
    try {
      await invoke('stop_live');
    } finally {
      liveStore.on = false;
      liveStore.totalPorts = 0;
      liveStore.readyPorts = [];
      portsStore.resetLive();
    }
  },

  async getSimNumbers() {
    try {
      await invoke('get_sim_numbers');
    } catch (e) {
      toast('Warning', 'USSD failed', String(e));
    }
  },

  async deleteSelected(ids: number[]) {
    if (ids.length === 0) return;
    messagesStore.deleteBusy = true;
    try {
      await invoke('delete_selected', { ids });
    } catch (e) {
      toast('Danger', 'Delete failed', String(e));
    } finally {
      messagesStore.deleteBusy = false;
    }
  },

  async clearAll() {
    try {
      await invoke('clear_all');
    } finally {
      messagesStore.items = [];
      messagesStore.clearSelection();
    }
  },

  async togglePortChecked(port: string, checked: boolean) {
    await invoke('toggle_port_checked', { port, checked });
  },

  async setAllPortsChecked(checked: boolean) {
    await invoke('set_all_ports_checked', { checked });
  },
};
