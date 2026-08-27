import type { ToastData } from '$lib/types';
import { isTauri } from '$lib/utils/tauri';
import { liveStore } from '$lib/stores/live.svelte';
import { settingsStore } from '$lib/stores/settings.svelte';

/**
 * Application auto-updater service.
 *
 * Centralises every interaction with the Tauri updater plugin so both the
 * manual "Check Now" button and background automatic checks share identical
 * behaviour. All outcomes are reported through the in-app toast system because
 * browser alert()/confirm() dialogs are unreliable inside webviews.
 */

let nextToastId = 1000;
let checking = false;

function toast(kind: ToastData['kind'], title: string, body: string): void {
  liveStore.addToast({ id: nextToastId++, kind, title, body, otp: null });
}

async function currentAppVersion(): Promise<string | undefined> {
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    return await getVersion();
  } catch {
    return undefined;
  }
}

/** Download the pending update, install it and relaunch the application. */
async function applyUpdate(update: NonNullable<Awaited<ReturnType<typeof checkForUpdate>>>): Promise<void> {
  const version = update.version;
  let downloaded = 0;
  let total = 0;
  try {
    await update.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        const mb =
          typeof event.data.contentLength === 'number'
            ? `${(event.data.contentLength / 1024 / 1024).toFixed(1)} MB`
            : '';
        toast('Info', 'Downloading update', `Version ${version} ${mb ? `(${mb}) ` : ''}is downloading…`);
        total = typeof event.data.contentLength === 'number' ? event.data.contentLength : 0;
        downloaded = 0;
        liveStore.updateProgress = { version, downloadedBytes: 0, totalBytes: total, phase: 'download' };
      } else if (event.event === 'Progress') {
        downloaded += typeof event.data.chunkLength === 'number' ? event.data.chunkLength : 0;
        liveStore.updateProgress = { version, downloadedBytes: downloaded, totalBytes: total, phase: 'download' };
      } else if (event.event === 'Finished') {
        liveStore.updateProgress = { version, downloadedBytes: total, totalBytes: total, phase: 'install' };
      }
    });
    liveStore.updateProgress = null;
    toast('Success', 'Update installed', `Version ${version} installed. Restarting the app…`);
    const { relaunch } = await import('@tauri-apps/plugin-process');
    await relaunch();
  } catch (error) {
    liveStore.updateProgress = null;
    console.error('[updater] install failed:', error);
    toast(
      'Danger',
      'Update installation failed',
      `${String(error).slice(0, 180)} — please restart and retry.`,
    );
  }
}

async function checkForUpdate(): Promise<Awaited<ReturnType<typeof import('@tauri-apps/plugin-updater').check>>> {
  const { check } = await import('@tauri-apps/plugin-updater');
  return check();
}

/**
 * Run one update check.
 *
 * @param interactive  `true`  → user-initiated ("Check Now"): every outcome is
 *                               surfaced via toast or native dialog.
 *                     `false` → background check: stays completely silent
 *                               unless an update (or an unexpected error) is found.
 */
export async function runUpdateCheck(interactive: boolean): Promise<void> {
  if (!isTauri()) {
    if (interactive) {
      toast(
        'Warning',
        'Update check unavailable',
        'Updates can only be checked inside the desktop application.',
      );
    }
    return;
  }

  if (checking) return;
  checking = true;

  try {
    const update = await checkForUpdate();

    if (!update) {
      if (interactive) {
        const v = await currentAppVersion();
        toast(
          'Info',
          "You're up to date",
          `No updates found${v ? ` — you are running the latest version (v${v}).` : '.'}`,
        );
      }
      return;
    }

    if (interactive) {
      const ok = await confirmDialog(
        `Version ${update.version} is available.${
          update.body?.trim() ? `\n\n${update.body.trim().slice(0, 500)}` : ''
        }\n\nDownload and install now?`,
      );
      if (ok) {
        await applyUpdate(update);
      } else {
        toast(
          'Info',
          'Update postponed',
          `Version ${update.version} remains available — press Check Now any time.`,
        );
      }
    } else {
      toast(
        'Info',
        'Update available',
        `Version ${update.version} is ready — open Settings → Updates and press Check Now.`,
      );
    }
  } catch (error) {
    // Endpoint unreachable, malformed latest.json, signature problems …
    console.error('[updater] check failed:', error);
    if (interactive) {
      toast('Danger', 'Update check failed', String(error).slice(0, 200));
    }
  } finally {
    checking = false;
  }
}

async function confirmDialog(text: string): Promise<boolean> {
  const { confirm } = await import('@tauri-apps/plugin-dialog');
  return confirm(text, {
    title: 'Update available',
    kind: 'info',
    okLabel: 'Install update',
    cancelLabel: 'Not now',
  });
}

/* ------------------------------------------------------------------ */
/* Background automatic checks                                         */
/* ------------------------------------------------------------------ */

const STARTUP_DELAY_MS = 30_000; // first silent check shortly after launch
const MIN_INTERVAL_HOURS = 1;
const MAX_INTERVAL_HOURS = 168;

let startupTimer: ReturnType<typeof setTimeout> | undefined;
let intervalTimer: ReturnType<typeof setInterval> | undefined;

function stopTimers(): void {
  clearTimeout(startupTimer);
  clearInterval(intervalTimer);
  startupTimer = undefined;
  intervalTimer = undefined;
}

/** Restart the scheduled silent update checks based on current settings. */
export function restartAutoUpdater(): void {
  stopTimers();

  const { autoCheck, checkInterval } = settingsStore.updates;
  if (!autoCheck || !isTauri()) return;

  const hours = Math.min(MAX_INTERVAL_HOURS, Math.max(MIN_INTERVAL_HOURS, checkInterval || 24));
  startupTimer = setTimeout(() => void runUpdateCheck(false), STARTUP_DELAY_MS);
  intervalTimer = setInterval(() => void runUpdateCheck(false), hours * 3_600_000);
}
