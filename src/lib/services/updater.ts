import type { DownloadEvent } from '@tauri-apps/plugin-updater';
import type { ToastData } from '$lib/types';
import { isTauri } from '$lib/utils/tauri';
import { liveStore } from '$lib/stores/live.svelte';
import { settingsStore } from '$lib/stores/settings.svelte';
import { updaterStore } from '$lib/stores/updater.svelte';
import { canCheck, cooldownRemaining, cooldownSeconds } from '$lib/utils/update-policy';

/**
 * Application auto-updater service.
 *
 * Every interaction with the Tauri updater plugin goes through here so the
 * manual "Check Now" button and the background timer share one state machine
 * (`updaterStore`) and one rate limit. The UI reads the store; nothing else
 * calls the plugin directly.
 *
 * The download and the install are issued separately — see `UpdateStage` — so
 * the user reviews the release notes, presses Update, and restarts on their own
 * schedule rather than having the app relaunch itself mid-shift.
 */

/**
 * The slice of the plugin's `Update` this service uses.
 *
 * Narrowing it to an interface lets the browser preview supply a stand-in
 * without the rest of the flow knowing which one it holds. `relaunch` is part
 * of it for the same reason: the real one restarts the process, the preview one
 * reloads the page.
 */
export interface UpdateSource {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
  download(onEvent?: (e: DownloadEvent) => void): Promise<void>;
  install(): Promise<void>;
  relaunch(): Promise<void>;
  close(): Promise<void>;
}

let nextToastId = 1000;
/** The checked update, held open until it is installed or superseded. */
let handle: UpdateSource | null = null;

function toast(kind: ToastData['kind'], title: string, body: string): void {
  liveStore.addToast({ id: nextToastId++, kind, title, body, otp: null });
}

async function currentAppVersion(): Promise<string | null> {
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    return await getVersion();
  } catch {
    return null;
  }
}

/** Ask the plugin, and wrap the result so `relaunch` travels with it. */
async function fetchUpdate(): Promise<UpdateSource | null> {
  const { check } = await import('@tauri-apps/plugin-updater');
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    currentVersion: update.currentVersion,
    date: update.date,
    body: update.body,
    download: (onEvent) => update.download(onEvent),
    install: () => update.install(),
    relaunch: async () => {
      const { relaunch } = await import('@tauri-apps/plugin-process');
      await relaunch();
    },
    close: () => update.close(),
  };
}

async function releaseHandle(): Promise<void> {
  const old = handle;
  handle = null;
  try {
    await old?.close();
  } catch {
    // A resource that is already gone is not worth reporting.
  }
}

function short(error: unknown, max = 200): string {
  return String(error).slice(0, max);
}

/**
 * Run one update check.
 *
 * @param interactive `true` for the "Check Now" button — refusals and results
 *        are surfaced. `false` for the background timer, which stays silent
 *        apart from announcing an update it found.
 */
export async function runUpdateCheck(interactive: boolean): Promise<void> {
  const preview = !isTauri();
  if (preview && !(import.meta as any).env?.DEV) {
    if (interactive) {
      toast(
        'Warning',
        'Update check unavailable',
        'Updates can only be checked inside the desktop application.',
      );
    }
    return;
  }
  // A browser dev session has no updater backend; the preview source stands in
  // so the panel can be worked on without cutting a release.
  if (preview && !interactive) return;

  // An update already waiting needs no further requests to the endpoint.
  if (updaterStore.pending) return;

  const now = Date.now();
  if (
    !canCheck({
      busy: updaterStore.stage === 'checking',
      lastCheckedAt: updaterStore.lastCheckedAt,
      now,
      interactive,
    })
  ) {
    if (interactive && updaterStore.stage !== 'checking') {
      const left = cooldownSeconds(cooldownRemaining(updaterStore.lastCheckedAt, now));
      toast(
        'Info',
        'Checked a moment ago',
        `The release endpoint was just queried — try again in ${left}s.`,
      );
    }
    return;
  }

  updaterStore.beginCheck();
  await releaseHandle();

  try {
    let update: UpdateSource | null;
    if (preview) {
      const { previewUpdate } = await import('./updater-preview');
      await new Promise((r) => setTimeout(r, 700));
      update = previewUpdate(updaterStore.currentVersion ?? '1.2.0');
    } else {
      update = await fetchUpdate();
    }
    updaterStore.lastCheckedAt = Date.now();

    if (!update) {
      updaterStore.foundNothing(await currentAppVersion());
      return;
    }

    handle = update;
    updaterStore.foundUpdate({
      version: update.version,
      currentVersion: update.currentVersion,
      date: update.date,
      body: update.body,
    });
    if (!interactive) {
      toast(
        'Info',
        `Version ${update.version} available`,
        'See what changed and install it from the card in the corner.',
      );
    }
  } catch (error) {
    // Endpoint unreachable, malformed latest.json, signature problems …
    console.error('[updater] check failed:', error);
    updaterStore.lastCheckedAt = Date.now();
    updaterStore.fail(short(error));
    if (interactive) toast('Danger', 'Update check failed', short(error));
  }
}

/** Download the pending update, leaving it staged for `restartNow()`. */
export async function downloadUpdate(): Promise<void> {
  if (!handle || updaterStore.stage === 'downloading') return;

  updaterStore.downloadedBytes = 0;
  updaterStore.totalBytes = 0;
  updaterStore.stage = 'downloading';

  try {
    await handle.download((event) => {
      if (event.event === 'Started') {
        updaterStore.totalBytes =
          typeof event.data.contentLength === 'number' ? event.data.contentLength : 0;
        updaterStore.downloadedBytes = 0;
      } else if (event.event === 'Progress') {
        updaterStore.downloadedBytes +=
          typeof event.data.chunkLength === 'number' ? event.data.chunkLength : 0;
      } else if (event.event === 'Finished') {
        if (updaterStore.totalBytes > 0) {
          updaterStore.downloadedBytes = updaterStore.totalBytes;
        }
      }
    });
    updaterStore.stage = 'ready';
  } catch (error) {
    console.error('[updater] download failed:', error);
    updaterStore.fail(short(error));
    toast('Danger', 'Download failed', `${short(error, 180)} — you can retry.`);
  }
}

/**
 * Install the staged package and restart.
 *
 * On Windows the installer takes over and ends the process itself, so the
 * `relaunch()` below is only reached on platforms where `install()` returns.
 */
export async function restartNow(): Promise<void> {
  if (!handle || updaterStore.stage !== 'ready') return;
  updaterStore.stage = 'installing';
  try {
    await handle.install();
    await handle.relaunch();
  } catch (error) {
    console.error('[updater] install failed:', error);
    updaterStore.fail(short(error));
    toast('Danger', 'Update installation failed', `${short(error, 180)} — please restart and retry.`);
  }
}

/** Put the update aside; the next check will find it again. */
export async function dismissUpdate(): Promise<void> {
  await releaseHandle();
  updaterStore.reset();
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
