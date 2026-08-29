import { parseReleaseNotes, type NoteSection } from '$lib/utils/release-notes';

/**
 * Where the update flow currently stands.
 *
 * The download and the install are deliberately separate stages: the user asked
 * to see the release notes, press Update, watch the download finish, and only
 * then choose to restart. `downloadAndInstall` would collapse the last three
 * into one and restart the app out from under them.
 */
export type UpdateStage =
  | 'idle'
  | 'checking'
  | 'uptodate'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'installing'
  | 'error';

export function createUpdaterStore() {
  let stage = $state<UpdateStage>('idle');
  let version = $state<string | null>(null);
  let currentVersion = $state<string | null>(null);
  let releaseDate = $state<string | null>(null);
  let notesBody = $state<string | null>(null);
  let downloadedBytes = $state(0);
  let totalBytes = $state(0);
  let error = $state<string | null>(null);
  let lastCheckedAt = $state<number | null>(null);
  /** Set while Settings → Updates is on screen, so the floating card can yield. */
  let panelOpen = $state(false);
  /**
   * The floating card was waved off.
   *
   * Deliberately not the same as dismissing: a staged download must survive it,
   * so this only hides the dock and leaves the update itself in place for the
   * Settings panel.
   */
  let snoozed = $state(false);

  const notes = $derived<NoteSection[]>(parseReleaseNotes(notesBody));

  /** True once a specific version is known and the flow has somewhere to go. */
  const pending = $derived(
    stage === 'available' ||
      stage === 'downloading' ||
      stage === 'ready' ||
      stage === 'installing',
  );

  function beginCheck() {
    stage = 'checking';
    error = null;
  }

  function foundUpdate(u: {
    version: string;
    currentVersion: string;
    date?: string;
    body?: string;
  }) {
    version = u.version;
    currentVersion = u.currentVersion;
    releaseDate = u.date ?? null;
    notesBody = u.body ?? null;
    downloadedBytes = 0;
    totalBytes = 0;
    error = null;
    snoozed = false;
    stage = 'available';
  }

  function foundNothing(current: string | null) {
    if (current) currentVersion = current;
    version = null;
    notesBody = null;
    releaseDate = null;
    error = null;
    stage = 'uptodate';
  }

  function fail(message: string) {
    error = message;
    // A failed download leaves a known version behind; keep it so the card can
    // offer a retry instead of dropping back to "no updates found".
    stage = 'error';
  }

  function reset() {
    stage = 'idle';
    version = null;
    releaseDate = null;
    notesBody = null;
    downloadedBytes = 0;
    totalBytes = 0;
    error = null;
    snoozed = false;
  }

  return {
    get stage() { return stage; },
    set stage(v: UpdateStage) { stage = v; },
    get version() { return version; },
    get currentVersion() { return currentVersion; },
    set currentVersion(v: string | null) { currentVersion = v; },
    get releaseDate() { return releaseDate; },
    get notesBody() { return notesBody; },
    get notes() { return notes; },
    get pending() { return pending; },
    get downloadedBytes() { return downloadedBytes; },
    set downloadedBytes(v: number) { downloadedBytes = v; },
    get totalBytes() { return totalBytes; },
    set totalBytes(v: number) { totalBytes = v; },
    get error() { return error; },
    get lastCheckedAt() { return lastCheckedAt; },
    set lastCheckedAt(v: number | null) { lastCheckedAt = v; },
    get panelOpen() { return panelOpen; },
    set panelOpen(v: boolean) { panelOpen = v; },
    get snoozed() { return snoozed; },
    set snoozed(v: boolean) { snoozed = v; },
    beginCheck,
    foundUpdate,
    foundNothing,
    fail,
    reset,
  };
}

export const updaterStore = createUpdaterStore();
