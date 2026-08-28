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

/**
 * Fold legacy shapes into the current one.
 *
 * Retention used to be two settings — an `autoDeleteExpired` toggle plus a
 * `retentionHours` value — which could disagree (off + 2h) and left the backend
 * with no single number to prune SIM storage by. They are now one value where
 * `0` means "keep everything", so an existing profile that had auto-delete
 * switched off has to migrate to 0 instead of silently turning cleanup on.
 */
function migrate(parsed: any): any {
  if (parsed?.general && 'autoDeleteExpired' in parsed.general) {
    const { autoDeleteExpired, ...general } = parsed.general;
    if (autoDeleteExpired === false) general.retentionHours = 0;
    return { ...parsed, general };
  }
  return parsed;
}

function loadSettings(): SettingsState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = migrate(JSON.parse(stored));
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
    get developer() { return settings.developer; },

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
    setDeveloper(v: Partial<SettingsState['developer']>) {
      settings = { ...settings, developer: { ...settings.developer, ...v } };
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