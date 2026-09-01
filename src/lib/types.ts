export interface PortInfo {
  name: string;
  /** Stable identity key (Linux by-path id, else the name). */
  path: string;
  checked: boolean;
  sim_number: string;
  /**
   * ICCID of the card in this slot, once a probe has read it. The phone number is
   * filed against this rather than against the port, because tty numbering is
   * reassigned on every hotplug.
   */
  iccid?: string | null;
  /**
   * Result of the last liveness probe: `true` a modem answered, `false` the
   * device node exists but nothing replied (empty SIM slot), `null` never
   * probed. A SIM bank publishes a port per channel whether or not a SIM is
   * inserted, so port count alone says nothing about reachable modems.
   */
  alive: boolean | null;
  live_ready: boolean;
  live_error: string | null;
}

export interface SmsMessage {
  port: string;
  index: number;
  from: string;
  received: string;
  status: string;
  text: string;
  /** SIM memory indices of every fragment a concatenated SMS was assembled from. */
  part_indices?: number[];
}

export interface SmsItem {
  id: number;
  message: SmsMessage;
  otp: string | null;
  is_new: boolean;
}

export type QuickFilter = 'All' | 'Otp' | 'Today';
export type ViewMode = 'Table' | 'Cards';

export interface ToastData {
  id: number;
  kind: 'Info' | 'Success' | 'Warning' | 'Danger' | 'Otp';
  title: string;
  body: string;
  otp: string | null;
  /**
   * How many identical notices this card stands for, set by
   * `utils/toast-queue.ts::pushToast` when repeats collapse. Absent means one.
   */
  count?: number;
}

export interface ScanStatus {
  busy: boolean;
  done: number;
  total: number;
}

export interface LiveStatus {
  on: boolean;
  ready: number;
  total: number;
}

export interface AppState {
  ports: PortInfo[];
  messages: SmsItem[];
  selected: number[];
  query: string;
  quick_filter: QuickFilter;
  port_filter: string | null;
  view_mode: ViewMode;
  scan: ScanStatus;
  live: LiveStatus;
  delete_busy: boolean;
  status_text: string;
  failed_notes: string[];
}

export type NavSection = 'inbox' | 'ports' | 'settings' | 'logs';

export interface SettingsState {
  general: {
    autoStartLive: boolean;
    confirmDelete: boolean;
    portRefreshInterval: number;
    /**
     * How long a received SMS is kept, in hours. `0` disables automatic
     * cleanup entirely — this single value drives both the in-app inbox and
     * SIM-storage pruning, replacing the old separate on/off toggle.
     */
    retentionHours: number;
  };
  notifications: {
    /** Mutes the in-app OTP toasts. Operational errors ignore this. */
    enabled: boolean;
  };
  appearance: {
    theme: 'system' | 'dark' | 'light';
    showSIMColumn: boolean;
    showPortColumn: boolean;
    /** Rows/messages per page. 'auto' fills the available viewport height. */
    pageSize: 'auto' | number;
  };
  updates: {
    autoCheck: boolean;
    checkInterval: number;
  };
  developer: {
    enabled: boolean;
    autoScroll: boolean;
  };
}

export const DEFAULT_SETTINGS: SettingsState = {
  general: {
    autoStartLive: false,
    confirmDelete: true,
    portRefreshInterval: 30,
    retentionHours: 2,
  },
  notifications: {
    enabled: true,
  },
  appearance: {
    theme: 'system',
    showSIMColumn: true,
    showPortColumn: true,
    pageSize: 'auto',
  },
  updates: {
    autoCheck: true,
    checkInterval: 24,
  },
  developer: {
    enabled: false,
    autoScroll: true,
  },
};

export * from './types/logs';
