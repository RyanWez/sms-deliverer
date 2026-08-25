export interface PortInfo {
  name: string;
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
