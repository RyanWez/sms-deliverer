// Development-only synthetic data generator for performance profiling.
// Guarded: production builds tree-shake but also runtime guard via import.meta.env.DEV
// Do not import this module in production code paths without DEV guard.

import type { SmsItem, SmsMessage, PortInfo } from '$lib/types';

function randInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function randomPhone(): string {
  // E.164-ish
  return `+${randInt(1, 99)}${Array.from({ length: randInt(9, 11) }, () => randInt(0, 9)).join('')}`;
}

function randomText(long = false): string {
  const phrases = [
    'Your verification code is',
    'Use code',
    'OTP for login:',
    'Your order has shipped',
    'Delivery attempt failed, visit',
    'Welcome to our service!',
    'Payment received: $',
    'Account security alert',
    'Please confirm your email',
    'Your package will arrive tomorrow',
  ];
  const words = ['lorem', 'ipsum', 'dolor', 'sit', 'amet', 'consectetur', 'adipiscing', 'elit', 'sed', 'do', 'eiusmod', 'tempor', 'incididunt', 'labore', 'dolore', 'magna', 'aliqua', 'enim', 'ad', 'minim', 'veniam', 'quis', 'nostrud', 'exercitation'];
  const useOtp = Math.random() < 0.25;
  let base = phrases[randInt(0, phrases.length - 1)];
  if (useOtp) {
    const code = `${randInt(1000, 999999)}`.padStart(4, '0');
    base += ` ${code} valid for 5 minutes. Do not share.`;
  } else {
    base += ` ${Array.from({ length: randInt(6, 18) }, () => words[randInt(0, words.length - 1)]).join(' ')}.`;
  }
  if (long && Math.random() < 0.15) {
    base += ' ' + Array.from({ length: randInt(30, 80) }, () => words[randInt(0, words.length - 1)]).join(' ') + '.';
  }
  return base;
}

function randomDateWithinDays(days: number): string {
  const now = Date.now();
  const past = now - randInt(0, days * 86400000);
  // sprinkle some future? no
  return new Date(past).toISOString();
}

export function generateSyntheticPorts(count: number = 32): PortInfo[] {
  const out: PortInfo[] = [];
  for (let i = 0; i < count; i++) {
    const idx = i + 1;
    const name = idx <= 8 ? `COM${idx}` : `/dev/ttyUSB${idx - 1}`;
    const hasSim = Math.random() < 0.85;
    const sim = hasSim ? `+${Array.from({ length: 12 }, () => randInt(0, 9)).join('')}` : '';
    const isError = Math.random() < 0.08;
    out.push({
      name,
      path: name.includes('ttyUSB') ? `pci-0000:00:14.0-usb-0:1.${idx}:1.0-port0` : name,
      checked: Math.random() < 0.6,
      sim_number: sim,
      alive: null,
      live_ready: Math.random() < 0.3,
      live_error: isError ? 'AT+CMGF failed: timeout after 3000ms — device not responding' : null,
    });
  }
  return out;
}

export function generateSyntheticMessages(count: number, ports: PortInfo[]): SmsItem[] {
  const out: SmsItem[] = [];
  for (let i = 0; i < count; i++) {
    const port = ports[randInt(0, ports.length - 1)].name;
    const isOtp = Math.random() < 0.22;
    const otp = isOtp ? `${randInt(1000, 999999)}`.padStart(randInt(4, 6), '0') : null;
    const text = isOtp
      ? `Your OTP is ${otp}. Valid for 10 minutes. Do not share this code. Ref: ${randInt(100000, 999999)}`
      : randomText(true);
    const msg: SmsMessage = {
      port,
      index: i,
      from: randomPhone(),
      received: randomDateWithinDays(14),
      status: Math.random() < 0.9 ? 'REC UNREAD' : 'REC READ',
      text,
    };
    out.push({
      id: 100000 + i,
      message: msg,
      otp,
      is_new: Math.random() < 0.15,
    });
  }
  // sort time descending roughly but store will sort anyway
  return out;
}

// Helper to inject into stores — call from browser console in dev:
// import { injectSyntheticData } from '$lib/utils/synthetic'; injectSyntheticData(64, 5000)
export async function injectSyntheticData(portCount = 32, messageCount = 2000): Promise<void> {
  if (!(import.meta as any).env?.DEV) {
    console.warn('[synthetic] only available in DEV');
    return;
  }
  const { portsStore } = await import('$lib/stores/ports.svelte');
  const { messagesStore } = await import('$lib/stores/messages.svelte');
  const ports = generateSyntheticPorts(portCount);
  const msgs = generateSyntheticMessages(messageCount, ports);
  portsStore.set(ports);
  messagesStore.items = msgs;
  console.info(`[synthetic] injected ${ports.length} ports, ${msgs.length} messages`);
  // expose counts for quick profiling
  console.info(`[synthetic] otpCount=${msgs.filter(m=>m.otp).length}, visible would be ${msgs.length}`);
}

// Expose globally in dev for console access
if ((import.meta as any).env?.DEV && typeof window !== 'undefined') {
  (window as any).__injectSyntheticData = injectSyntheticData;
  (window as any).__genPorts = generateSyntheticPorts;
  (window as any).__genMessages = generateSyntheticMessages;
  console.info('[synthetic] dev helper: call __injectSyntheticData(64,5000) in console to load synthetic data');
}
