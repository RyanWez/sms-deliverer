import type { SettingsState } from '$lib/types';
import { DEFAULT_SETTINGS } from '$lib/types';

const STORAGE_KEY = 'sms-reader-settings';

/**
 * A private copy of the defaults, safe to hand to `$state`.
 *
 * `DEFAULT_SETTINGS` is a module-level literal and the settings object is a deep
 * `$state` proxy the Settings page mutates in place, so a shallow spread would
 * let user edits write straight through into the shared literal — after a reset
 * (or on a fresh profile) the "defaults" would drift with whatever the user
 * last typed.
 */
function defaultsClone(): SettingsState {
  return structuredClone(DEFAULT_SETTINGS);
}

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
      return deepMerge(defaultsClone(), parsed);
    }
  } catch {
    // ignore parse errors
  }
  return defaultsClone();
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
      settings = defaultsClone();
      saveSettings(settings);
      applyTheme('system');
    },
  };
}

type Theme = SettingsState['appearance']['theme'];

const DARK_QUERY = '(prefers-color-scheme: dark)';

/**
 * The live `(prefers-color-scheme: dark)` subscription, held only while the
 * theme is "system".
 *
 * Kept in module scope (rather than added once at startup) so it can be torn
 * down when the user pins Dark or Light — otherwise the OS flipping at sunset
 * would repaint an explicitly pinned theme. Detaching before every (re)attach
 * also makes `applyTheme` idempotent: calling it twice cannot stack listeners.
 */
let systemQuery: MediaQueryList | null = null;
let systemListener: (() => void) | null = null;

function detachSystemListener() {
  if (systemQuery && systemListener) {
    systemQuery.removeEventListener('change', systemListener);
  }
  systemQuery = null;
  systemListener = null;
}

/**
 * Write the resolved theme onto <html>.
 *
 * Both classes are set explicitly instead of only adding/removing `dark`: the
 * palette in app.css is keyed on `.dark` / `.light`, and an element carrying
 * neither would fall back to the bare `:root` (dark) rule — which is what made
 * "Light" a no-op before. `colorScheme` is set alongside so the browser-native
 * chrome we cannot style (scrollbar gutters, <select> popups, number spinners,
 * form control defaults) follows the app instead of staying dark.
 */
function setResolved(resolved: 'dark' | 'light') {
  const root = document.documentElement;
  root.classList.toggle('dark', resolved === 'dark');
  root.classList.toggle('light', resolved === 'light');
  root.style.colorScheme = resolved;
}

function applyTheme(theme: Theme) {
  if (typeof document === 'undefined') return;

  detachSystemListener();

  if (theme !== 'system') {
    setResolved(theme);
    return;
  }

  // matchMedia is missing in some embedded webviews; dark is the shipped
  // default, so fall back to it rather than to the OS-less light branch.
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    setResolved('dark');
    return;
  }

  const query = window.matchMedia(DARK_QUERY);
  const listener = () => setResolved(query.matches ? 'dark' : 'light');
  query.addEventListener('change', listener);
  systemQuery = query;
  systemListener = listener;
  listener();
}

export const settingsStore = createSettingsStore();

if (typeof window !== 'undefined') {
  applyTheme(settingsStore.appearance.theme);
}
