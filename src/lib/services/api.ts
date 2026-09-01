import { isTauri } from '$lib/utils/tauri';
import { portsStore } from '$lib/stores/ports.svelte';
import { messagesStore } from '$lib/stores/messages.svelte';
import { liveStore } from '$lib/stores/live.svelte';
import { settingsStore } from '$lib/stores/settings.svelte';
import { logsStore } from '$lib/stores/logs.svelte';
import { describePortChanges, diffPorts } from '$lib/utils/port-refresh';
import { portLabel } from '$lib/utils/port';
import { toCsv } from '$lib/utils/csv';
import type { CsvRow } from '$lib/utils/csv';
import {
  applyBufferedUpdate,
  dropBufferedIds,
  selectRowsToAdd,
} from '$lib/utils/message-buffer';
import type { SmsItem, ToastData, PortInfo, LogEntry } from '$lib/types';

let nextToastId = 1;
let initialized = false;
/** Guard against a manual Refresh and the background timer overlapping. */
let portRefreshInFlight = false;

function toast(kind: ToastData['kind'], title: string, body: string, otp?: string | null) {
  liveStore.addToast({
    id: nextToastId++,
    kind,
    title,
    body,
    otp: otp ?? null,
  });
}

/**
 * Announce a freshly detected OTP, unless the user has muted notifications.
 *
 * Only OTP announcements are gated: operational feedback (scan/detect/export
 * failures) always goes through `toast` directly, because silencing an error is
 * never what "Enable Notifications: off" is asking for.
 */
function notifyOtp(title: string, body: string, otp: string) {
  if (!settingsStore.notifications.enabled) return;
  toast('Otp', title, body, otp);
}

/**
 * Publish a re-enumerated port list and say what changed.
 *
 * The operator plugs and pulls sticks by hand, so a refresh that silently swaps
 * the list out is the one thing this must not be — but it also runs on a timer,
 * so an unchanged bank has to pass without a word. The very first enumeration is
 * exempt: there is no previous state, and everything would read as "new".
 *
 * Port topology is operational information, like `detect:done`, so it goes
 * through `toast` and is deliberately not gated on `notifications.enabled` —
 * that switch mutes OTP announcements only.
 */
function applyRefreshedPorts(ports: PortInfo[]) {
  const first = !portsStore.hasLoaded;
  const diff = diffPorts(portsStore.items, ports);
  portsStore.set(ports);
  if (first) return;

  const notice = describePortChanges(diff);
  if (!notice) return;
  portsStore.markRecentlyAdded(diff.added);
  toast(notice.kind, notice.title, notice.body);
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

      // High-performance batching buffer for Live & Scan events
      let msgBuffer: SmsItem[] = [];
      let msgTimer: ReturnType<typeof setTimeout> | null = null;
      let isInitialGathering = false;

      function flushMsgBuffer() {
        if (msgBuffer.length === 0) {
          msgTimer = null;
          isInitialGathering = false;
          return;
        }
        const incoming = msgBuffer;
        msgBuffer = [];
        msgTimer = null;
        isInitialGathering = false;

        const existing = new Set(messagesStore.items.map((m) => m.id));
        const toAdd = selectRowsToAdd(existing, incoming);
        if (toAdd.length > 0) {
          messagesStore.items = [...messagesStore.items, ...toAdd];
        }
      }

      let readyBuffer: string[] = [];
      let readyTimer: ReturnType<typeof setTimeout> | null = null;
      function flushReadyBuffer() {
        if (readyBuffer.length === 0) return;
        const ports = readyBuffer;
        readyBuffer = [];
        readyTimer = null;
        const updates = ports.map((p) => ({
          name: p,
          changes: { live_ready: true, live_error: null },
        }));
        portsStore.batchUpdatePorts(updates);
        liveStore.readyPorts = portsStore.items
          .filter((p) => p.live_ready)
          .map((p) => p.name);
      }

      await listen('messages:reset', () => {
        if (msgTimer) clearTimeout(msgTimer);
        msgBuffer = [];
        msgTimer = null;
        isInitialGathering = true;
        messagesStore.items = [];
        messagesStore.clearSelection();
      });

      await listen<{ items: SmsItem[] }>('messages:added', (event) => {
        msgBuffer.push(...event.payload.items);
        if (!msgTimer) {
          // Gather initial thread outputs for 200ms so all rows drop in together as a unified waterfall!
          const delay = isInitialGathering ? 200 : 50;
          msgTimer = setTimeout(flushMsgBuffer, delay);
        }
      });

      await listen<{ ids: number[] }>('messages:removed', (event) => {
        // A row deleted while it is still buffered has to be forgotten here too,
        // otherwise the next flush appends it back into the store.
        msgBuffer = dropBufferedIds(msgBuffer, event.payload.ids);
        messagesStore.removeByIds(event.payload.ids);
        messagesStore.deleteBusy = false;
      });

      await listen<{
        requested: number;
        freed: number;
        removed: number;
        kept: number;
        failed_ports: number;
      }>('delete:done', (event) => {
        messagesStore.deleteBusy = false;
        const { requested, freed, removed, kept, failed_ports } = event.payload;
        // A row is only dropped when every SIM slot it occupies is confirmed
        // gone, so "kept" means the message is still on the card. Saying nothing
        // about it made a partial delete look exactly like a clean one.
        if (kept > 0 || failed_ports > 0) {
          const parts = [`Freed ${freed} of ${requested} SIM slot(s).`];
          if (removed > 0) parts.push(`${removed} message(s) removed.`);
          if (kept > 0) parts.push(`${kept} still on the SIM and kept in the list.`);
          if (failed_ports > 0) parts.push(`${failed_ports} port(s) failed.`);
          toast('Warning', 'Delete incomplete', parts.join(' '));
        } else if (removed > 0) {
          toast(
            'Success',
            'Delete complete',
            `${removed} message(s) removed, ${freed} SIM slot(s) freed.`
          );
        }
      });

      await listen<{ ports: PortInfo[] }>('ports:updated', (event) => {
        portsStore.set(event.payload.ports);
        // Ports can drop (serial failures) or come back (reconnects) behind
        // this snapshot — keep the "Live x/y" badge truthful by deriving the
        // ready set from the port list itself instead of optimistic local state.
        if (liveStore.on) {
          liveStore.readyPorts = event.payload.ports
            .filter((p) => p.live_ready)
            .map((p) => p.name);
        }
      });

      await listen<{ port: string; error: string }>('live:reconnecting', (event) => {
        liveStore.readyPorts = portsStore.items.filter((p) => p.live_ready).map((p) => p.name);
        // A port dropping out mid-shift is the event most worth interrupting for,
        // and console.warn is invisible in a packaged build with devtools closed.
        // Operational notices are deliberately not gated on notifications.enabled
        // (see applyRefreshedPorts) — that switch is for OTP alerts.
        toast(
          'Warning',
          'Port lost',
          `${portLabel(event.payload.port)} stopped answering — reconnecting. ${event.payload.error}`
        );
      });

      // A port live mode is holding open where no modem answers — an empty SIM
      // slot. Drop it from the ready set so the "Live x/y" badge counts only
      // ports that can actually deliver a message.
      await listen<{ port: string; error: string }>('live:offline', (event) => {
        liveStore.readyPorts = portsStore.items.filter((p) => p.live_ready).map((p) => p.name);
        console.info(`[api] Port ${event.payload.port} has no modem: ${event.payload.error}`);
      });

      await listen<{ found: number; total: number; unknown: number }>('detect:done', (event) => {
        liveStore.detectBusy = false;
        const { found, total, unknown } = event.payload;
        // Ports the probe could not reach keep their previous selection and
        // liveness, so they must not be described as deselected or as empty.
        const skipped = unknown > 0 ? ` ${unknown} port(s) could not be probed.` : '';
        if (found === 0) {
          toast('Warning', 'No modems found', `Probed ${total} port(s); none answered.${skipped}`);
        } else {
          toast(
            unknown > 0 ? 'Warning' : 'Success',
            'Detect complete',
            `${found} of ${total} port(s) have a modem. Empty slots were deselected.${skipped}`
          );
        }
      });

      await listen('live:stopped', () => {
        liveStore.on = false;
        liveStore.totalPorts = 0;
        liveStore.readyPorts = [];
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
          is_new: item.is_new ?? true,
        };
        // The backend allocates a fresh id for `sms:new` (it emits
        // `messages:updated` when a row is superseded), so a collision with a
        // buffered row should not happen — but if one ever did, patching the
        // buffer keeps a single ordered path for that id instead of racing the
        // flush.
        const patchedBuffer = applyBufferedUpdate(msgBuffer, smsItem);
        if (patchedBuffer) {
          msgBuffer = patchedBuffer;
        } else if (!messagesStore.items.some(m => m.id === smsItem.id)) {
          messagesStore.items = [...messagesStore.items, smsItem];
        }
        if (smsItem.otp) {
          notifyOtp('New OTP', smsItem.message.text ?? '', smsItem.otp);
        }
      });

      // A concatenated (long) message finished collecting all its parts — the
      // backend swapped the partial row for the complete text under the same
      // id, so we refresh that row in place instead of appending a duplicate.
      //
      // The id can be in the store or still in the add-buffer: the backend
      // supersedes a row it delivered through `messages:added`, and that burst
      // is held here for 50–200 ms. Patch wherever it is found — a store-only
      // lookup dropped the update, leaving the partial text and no OTP for a
      // message that has one.
      await listen<{ item: SmsItem }>('messages:updated', (event) => {
        const updated = event.payload.item;
        // Patch the buffer first and unconditionally: the buffered copy is
        // replaced, never merely shadowed, so the later flush can only append
        // this version. That is what stops the flush from re-staling the row.
        const patchedBuffer = applyBufferedUpdate(msgBuffer, updated);
        if (patchedBuffer) msgBuffer = patchedBuffer;

        const idx = messagesStore.items.findIndex((m) => m.id === updated.id);
        if (idx >= 0) {
          messagesStore.items = messagesStore.items.map((m, i) =>
            i === idx ? updated : m
          );
        }

        // An update for a row we have never seen is left alone on purpose:
        // re-adding it here would resurrect rows already deleted or purged.
        if ((idx >= 0 || patchedBuffer) && updated.otp) {
          notifyOtp('OTP detected', updated.message.text ?? '', updated.otp);
        }
      });

      await listen<{ port: string }>('sms:ready', (event) => {
        readyBuffer.push(event.payload.port);
        if (!readyTimer) {
          readyTimer = setTimeout(flushReadyBuffer, 50);
        }
      });

      await listen('scan:done', () => {
        flushMsgBuffer();
        liveStore.scanBusy = false;
      });

      await listen('ussd:done', () => {
        liveStore.ussdBusy = false;
      });

      await listen<{ error: string }>('export:failed', (event) => {
        toast('Danger', 'Export failed', event.payload.error);
      });

      // Without this the desktop app said nothing at all on success, so a
      // written file and a cancelled dialog looked the same. The browser
      // preview already toasted, so this is also a parity fix.
      await listen<{ path: string }>('export:saved', (event) => {
        toast('Success', 'Export complete', `Saved to ${event.payload.path}`);
      });

      // The idle SIM sweep runs on a timer with no operator involvement, so its
      // outcome only ever reached the Ports page footer.
      await listen<{ deleted: number; failed: number }>('sim_cleanup:done', (event) => {
        const { deleted, failed } = event.payload;
        if (failed > 0) {
          toast(
            'Warning',
            'SIM cleanup incomplete',
            `Freed ${deleted} expired SIM slot(s); ${failed} port(s) failed.`
          );
        } else if (deleted > 0) {
          toast('Success', 'SIM cleanup', `Freed ${deleted} expired SIM slot(s).`);
        }
      });

      if (settingsStore.general.autoStartLive && portsStore.items.some(p => p.checked)) {
        void api.startLive(true);
      }
    } catch (e) {
      console.warn('[api] Failed to setup Tauri event listeners:', e);
    }
  },

  async refreshPorts(source: 'manual' | 'auto' = 'manual') {
    // Two enumerations at once would only fight over the same shared port state:
    // the background timer can tick while the operator is on the Refresh button.
    if (portRefreshInFlight) return;
    portRefreshInFlight = true;
    try {
      if (!isTauri()) {
        const { generateSyntheticPorts } = await import('$lib/utils/synthetic');
        applyRefreshedPorts(generateSyntheticPorts(16));
        // Only the button press gets the "it did something" acknowledgement; the
        // timer stays quiet unless the port list actually changed.
        if (source === 'manual') toast('Success', 'Refreshed', 'Synthetic ports refreshed.');
        return;
      }
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const ports = await invoke<PortInfo[]>('refresh_ports');
        applyRefreshedPorts(ports);
      } catch (e) {
        toast('Danger', 'Refresh failed', String(e));
      }
    } finally {
      portRefreshInFlight = false;
    }
  },

  /**
   * Probe every port once and keep only the ones with a modem selected.
   *
   * Worth running before anything else on a partly-populated bank: an empty SIM
   * slot still exposes a serial device, and every other operation would
   * otherwise spend its full timeout chain (24 s for a scan, 35 s for a USSD
   * lookup) discovering that nothing is there.
   */
  async detectPorts() {
    if (!isTauri()) {
      liveStore.detectBusy = true;
      setTimeout(() => {
        // Three outcomes, matching the backend's verdict: a modem answered, the
        // slot proved empty, or the probe could not reach the port at all. The
        // last one keeps its previous selection and liveness and carries a
        // reason, so the preview shows that row the way the desktop app does.
        portsStore.set(
          portsStore.items.map((p, i) => {
            if (i % 7 === 3) {
              return { ...p, live_error: 'Cannot open port: Device or resource busy' };
            }
            const alive = i % 3 === 0;
            return { ...p, alive, checked: alive, live_error: null };
          })
        );
        liveStore.detectBusy = false;
        toast('Success', 'Detect complete', 'Simulated probe finished.');
      }, 600);
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      liveStore.detectBusy = true;
      await invoke('detect_ports');
    } catch (e) {
      liveStore.detectBusy = false;
      toast('Danger', 'Detect failed', String(e));
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
      // Live workers own their ports exclusively, so they do their own SIM
      // pruning — they need the retention window up front.
      await invoke('start_live', { retentionHours: settingsStore.general.retentionHours });
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
        // Preview parity: the desktop app reports the verdict through
        // `delete:done`, so the synthetic path has to say something too. There
        // is no SIM here, so every requested row is confirmed gone.
        setTimeout(() => {
          messagesStore.removeByIds(ids);
          messagesStore.deleteBusy = false;
          toast(
            'Success',
            'Delete complete',
            `${ids.length} message(s) removed, ${ids.length} SIM slot(s) freed.`
          );
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
   *
   * CSV cells go through `utils/csv`, which neutralizes formula-injection: the
   * sender, the SIM number and the body are all attacker-controlled. JSON needs
   * no such treatment — nothing evaluates a JSON string value.
   */
  async exportMessages(format: 'csv' | 'json') {
    const rows: CsvRow[] = messagesStore.visible.map((it) => ({
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

  async getLogs(limit?: number, minLevel?: string): Promise<LogEntry[]> {
    if (!isTauri()) {
      if (logsStore.items.length === 0) {
        // Populate synthetic logs for web preview
        const now = new Date().toISOString().replace('T', ' ').slice(0, 23);
        logsStore.setLogs([
          { id: 1, timestamp: now, level: 'INFO', target: 'core', message: 'SMS Reader initialized (Browser Simulation Mode)' },
          { id: 2, timestamp: now, level: 'INFO', target: 'ports', message: 'Discovered 16 simulated COM ports' },
          { id: 3, timestamp: now, level: 'DEBUG', target: 'modem', message: 'AT+CPMS? -> ("SM",0,50,"SM",0,50)' },
          { id: 4, timestamp: now, level: 'INFO', target: 'commands', message: 'Live monitoring engine idle and ready' },
          { id: 5, timestamp: now, level: 'WARN', target: 'ports', message: 'COM16 latency high (simulated warning)' },
        ]);
      }
      return logsStore.items;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const logs = await invoke<LogEntry[]>('get_logs', { limit, minLevel });
      logsStore.setLogs(logs);
      return logs;
    } catch (e) {
      console.warn('Failed to fetch logs:', e);
      return [];
    }
  },

  async clearLogs() {
    logsStore.clear();
    if (isTauri()) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('clear_logs');
      } catch (e) {
        console.warn('Failed to clear backend logs:', e);
      }
    }
  },

  async openLogFolder() {
    if (!isTauri()) {
      toast('Info', 'Log Directory', 'Log persistence is active in native application.');
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('open_log_folder');
      toast('Success', 'Log Directory', 'Opened log folder in file explorer.');
    } catch (e) {
      toast('Danger', 'Open Folder Failed', String(e));
    }
  },

  async purgeExpiredMessages(retentionHours: number) {
    if (!retentionHours || retentionHours <= 0) return 0;
    const purgedInStore = messagesStore.purgeExpired(retentionHours);
    if (isTauri()) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('purge_expired_messages', { maxAgeHours: retentionHours });
      } catch (e) {
        console.warn('Failed to purge expired messages from backend:', e);
      }
    }
    return purgedInStore;
  },

  /**
   * Prune expired messages out of SIM storage on the selected ports.
   *
   * Only meaningful while idle: a live worker holds its port open and prunes on
   * its own channel, and scan/USSD/delete need exclusive access, so the backend
   * answers "Busy" in those cases. That is expected for the background sweep,
   * hence the silent return.
   */
  async cleanupSimStorage(retentionHours: number) {
    if (!isTauri() || !retentionHours || retentionHours <= 0) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('cleanup_sim_storage', { retentionHours });
    } catch (e) {
      console.debug('[api] SIM cleanup skipped:', e);
    }
  },
};
