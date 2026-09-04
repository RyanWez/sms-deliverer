import type { SettingsState } from '$lib/types';
import { DEFAULT_SETTINGS } from '$lib/types';
import { normalizeRetentionHours } from '$lib/utils/retention';

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
 * with no single number to prune SIM storage by. They became one value where `0`
 * meant "keep everything", so a profile with auto-delete switched off migrated
 * to 0 rather than silently turning cleanup on.
 *
 * Settings now offers one window and 0 is not it, so that value — and the 2, 4,
 * 8, 24 and 168 the page used to offer — is folded on to the default as well.
 * Skipping this leaves a `<select>` whose value matches no option: it renders
 * blank, and the number the page is not showing is the one still in force.
 */
function migrate(parsed: any): any {
  let out = parsed;
  if (out?.general && 'autoDeleteExpired' in out.general) {
    const { autoDeleteExpired, ...general } = out.general;
    if (autoDeleteExpired === false) general.retentionHours = 0;
    out = { ...out, general };
  }
  if (out?.general && 'retentionHours' in out.general) {
    out = {
      ...out,
      general: {
        ...out.general,
        retentionHours: normalizeRetentionHours(out.general.retentionHours),
      },
    };
  }
  return out;
}

function loadSettings(): SettingsState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = migrate(JSON.parse(stored));
      const merged = deepMerge(defaultsClone(), parsed);
      // Write the migrated shape straight back, so what is on disk matches what
      // is in memory. Migration otherwise only ever corrects the copy the app
      // runs on: a legacy `retentionHours` of 2 stays in `localStorage` being
      // re-coerced on every launch, and anything that reads the raw profile — a
      // later migration keyed on the stored value, or somebody debugging a bank
      // in the field — is looking at a window Settings has no option for.
      saveSettings(merged);
      return merged;
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
    get forwarding() { return settings.forwarding; },

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
    setForwarding(v: Partial<SettingsState['forwarding']>) {
      settings = { ...settings, forwarding: { ...settings.forwarding, ...v } };
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
