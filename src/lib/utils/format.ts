const pad = (n: number) => n.toString().padStart(2, '0');

function toDate(received: string): Date | null {
  if (!received || received === '1970-01-01T00:00:00Z') return null;
  const d = new Date(received);
  return Number.isNaN(d.getTime()) ? null : d;
}

export function fmtDate(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function fmtTime(d: Date, withSeconds = false): string {
  const base = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  return withSeconds ? `${base}:${pad(d.getSeconds())}` : base;
}

export function fmtDateTime(received: string): string {
  const d = toDate(received);
  if (!d) return '';
  return `${fmtDate(d)} ${fmtTime(d)}`;
}

export function fmtFullDateTime(received: string): string {
  const d = toDate(received);
  if (!d) return 'Unknown time';
  return `${fmtDate(d)} ${fmtTime(d, true)}`;
}
