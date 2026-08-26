import type { SettingsState } from '$lib/types';
import { DEFAULT_SETTINGS } from '$lib/types';

const STORAGE_KEY = 'sms-reader-settings';

function deepMerge<T extends Record<string, any>>(target: T, source: Partial<T>): T {
  const result = { ...target };
  for (const key of Object.keys(source) as Array<keyof T>) {
    const sourceValue = source[key];
    const targetValue = target[key];
    if (
      sourceValue !== null &&
      typeof sourceValue === 'object' &&
      !Array.isArray(sourceValue) &&
      targetValue !== null &&
      typeof targetValue === 'object' &&
      !Array.isArray(targetValue)
    ) {
      (result as any)[key] = deepMerge(targetValue, sourceValue);
    } else if (sourceValue !== undefined) {
      (result as any)[key] = sourceValue;
    }
  }
  return result;
}

function loadSettings(): SettingsState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      return deepMerge(DEFAULT_SETTINGS, parsed);
    }
  } catch {
    // ignore parse errors
  }
  return { ...DEFAULT_SETTINGS };
}

function saveSettings(settings: SettingsState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // ignore storage errors
  }
}

export function createSettingsStore() {
  let settings = $state<SettingsState>(loadSettings());

  return {
    get settings() { return settings; },
    get general() { return settings.general; },
    get notifications() { return settings.notifications; },
    get otp() { return settings.otp; },
    get appearance() { return settings.appearance; },
    get updates() { return settings.updates; },

    setGeneral(v: Partial<SettingsState['general']>) {
      settings = { ...settings, general: { ...settings.general, ...v } };
      saveSettings(settings);
    },
    setNotifications(v: Partial<SettingsState['notifications']>) {
      settings = { ...settings, notifications: { ...settings.notifications, ...v } };
      saveSettings(settings);
    },
    setOtp(v: Partial<SettingsState['otp']>) {
      settings = { ...settings, otp: { ...settings.otp, ...v } };
      saveSettings(settings);
    },
    setAppearance(v: Partial<SettingsState['appearance']>) {
      settings = { ...settings, appearance: { ...settings.appearance, ...v } };
      saveSettings(settings);
      applyTheme(settings.appearance.theme);
    },
    setUpdates(v: Partial<SettingsState['updates']>) {
      settings = { ...settings, updates: { ...settings.updates, ...v } };
      saveSettings(settings);
    },

    resetToDefaults() {
      settings = { ...DEFAULT_SETTINGS };
      saveSettings(settings);
      applyTheme('system');
    },
  };
}

function applyTheme(theme: 'system' | 'dark' | 'light') {
  const root = document.documentElement;
  if (theme === 'dark') {
    root.classList.add('dark');
  } else if (theme === 'light') {
    root.classList.remove('dark');
  } else {
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  }
}

export const settingsStore = createSettingsStore();

if (typeof window !== 'undefined') {
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
  mediaQuery.addEventListener('change', () => {
    if (settingsStore.appearance.theme === 'system') {
      applyTheme('system');
    }
  });
  applyTheme(settingsStore.appearance.theme);
}