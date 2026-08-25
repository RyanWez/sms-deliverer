import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { portsStore } from '$lib/stores/ports.svelte';
import { messagesStore } from '$lib/stores/messages.svelte';
import { liveStore } from '$lib/stores/live.svelte';
import type { SmsItem, ToastData } from '$lib/types';

let nextToastId = 1;
let pollingInterval: ReturnType<typeof setInterval> | null = null;

function toast(kind: ToastData['kind'], title: string, body: string, otp?: string | null) {
  liveStore.addToast({
    id: nextToastId++,
    kind,
    title,
    body,
    otp: otp ?? null,
  });
}

export const api = {
  async init() {
    const ports = await invoke<SmsItem[]>('get_ports');
    portsStore.set(ports as any);

    await listen<SmsItem>('sms:new', (event) => {
      const item = event.payload as any;
      const smsItem: SmsItem = {
        id: item.id,
        message: item.message ?? item,
        otp: item.otp ?? null,
        is_new: true,
      };
      messagesStore.items = [...messagesStore.items, smsItem];
      if (smsItem.otp) {
        toast('Otp', 'New OTP', smsItem.message.text, smsItem.otp);
      }
    });

    await listen<any>('sms:ready', (event) => {
      const payload = event.payload;
      portsStore.updatePort(payload.port, { live_ready: true });
      const ready = portsStore.items.filter(p => p.live_ready).map(p => p.name);
      liveStore.readyPorts = ready;
    });

    pollingInterval = setInterval(async () => {
      try {
        const msgs = await invoke<SmsItem[]>('get_messages');
        if (msgs.length > messagesStore.items.length) {
          messagesStore.items = msgs;
        }
        const status = await invoke<string>('get_status_text');
        const ports = await invoke<any[]>('get_ports');
        portsStore.set(ports);
      } catch {
        // ignore polling errors
      }
    }, 1500);
  },

  async refreshPorts() {
    const ports = await invoke<any[]>('refresh_ports');
    portsStore.set(ports);
  },

  async startScan() {
    await invoke('start_scan');
    this._startStatusPolling();
  },

  async startLive() {
    const ports = await invoke<any[]>('get_ports');
    portsStore.set(ports);
    const checked = portsStore.items.filter(p => p.checked);
    liveStore.on = true;
    liveStore.totalPorts = checked.length;
    await invoke('start_live');
  },

  async stopLive() {
    await invoke('stop_live');
    liveStore.on = false;
  },

  async getSimNumbers() {
    await invoke('get_sim_numbers');
    this._startStatusPolling();
  },

  async deleteSelected(ids: number[]) {
    messagesStore.deleteBusy = true;
    try {
      await invoke('delete_selected', { ids });
      setTimeout(() => {
        messagesStore.removeByIds(ids);
        messagesStore.deleteBusy = false;
      }, 500);
    } catch {
      messagesStore.deleteBusy = false;
    }
  },

  async clearAll() {
    await invoke('clear_all');
    messagesStore.items = [];
    messagesStore.clearSelection();
  },

  async togglePortChecked(port: string, checked: boolean) {
    await invoke('toggle_port_checked', { port, checked });
  },

  async setAllPortsChecked(checked: boolean) {
    await invoke('set_all_ports_checked', { checked });
  },

  _startStatusPolling() {
    if (pollingInterval) return;
    pollingInterval = setInterval(async () => {
      try {
        const msgs = await invoke<SmsItem[]>('get_messages');
        messagesStore.items = msgs;
        const ports = await invoke<any[]>('get_ports');
        portsStore.set(ports);
        const status = await invoke<string>('get_status_text');
        if (status === 'idle' || status === '') {
          if (pollingInterval) { clearInterval(pollingInterval); pollingInterval = null; }
        }
      } catch {
        // ignore
      }
    }, 1000);
  }
};
