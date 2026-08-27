export interface PortInfo {
  name: string;
  /** Stable identity key (Linux by-path id, else the name). */
  path: string;
  checked: boolean;
  sim_number: string;
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
    minimizeToTray: boolean;
    confirmDelete: boolean;
    portRefreshInterval: number;
    autoDeleteExpired: boolean;
    retentionHours: number;
  };
  notifications: {
    enabled: boolean;
    soundEnabled: boolean;
    desktopNotifications: boolean;
    otpOnlyNotifications: boolean;
  };
  otp: {
    autoCopy: boolean;
    showInTable: boolean;
    highlightNewOtp: boolean;
    otpPattern: string;
  };
  appearance: {
    theme: 'system' | 'dark' | 'light';
    compactMode: boolean;
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
    logLevel: 'ALL' | 'ERROR' | 'WARN' | 'INFO';
    autoScroll: boolean;
    maxLogs: number;
  };
}

export const DEFAULT_SETTINGS: SettingsState = {
  general: {
    autoStartLive: false,
    minimizeToTray: false,
    confirmDelete: true,
    portRefreshInterval: 30,
    autoDeleteExpired: true,
    retentionHours: 2,
  },
  notifications: {
    enabled: true,
    soundEnabled: true,
    desktopNotifications: true,
    otpOnlyNotifications: false,
  },
  otp: {
    autoCopy: false,
    showInTable: true,
    highlightNewOtp: true,
    otpPattern: '\\b(\\d{4,8})\\b',
  },
  appearance: {
    theme: 'system',
    compactMode: false,
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
    logLevel: 'ALL',
    autoScroll: true,
    maxLogs: 1000,
  },
};

export * from './types/logs';
