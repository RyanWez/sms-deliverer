export type LogLevelFilter = 'ALL' | 'ERROR' | 'WARN' | 'INFO';

export interface LogEntry {
  id: number;
  timestamp: string;
  level: 'ERROR' | 'WARN' | 'INFO' | string;
  target: string;
  message: string;
}
