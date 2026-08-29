/**
 * Browser-preview stand-in for the updater plugin.
 *
 * `npm run dev` in a plain browser has no Tauri backend, so this module fakes
 * one release with realistic notes and a throttled byte stream — enough to
 * check the Settings → Updates layout without cutting a real release. It is
 * dynamically imported behind `import.meta.env.DEV`, so it never reaches a
 * production bundle.
 */

import type { UpdateSource } from './updater';

const PREVIEW_BODY = `## [9.9.9](https://example.invalid/compare/v1.2.0...v9.9.9) (2026-08-29)


### Features

* **updates:** review release notes in Settings before installing ([abc1234](https://example.invalid/commit/abc1234))
* add expand/collapse toggle for long message text ([091c48d](https://example.invalid/commit/091c48d))


### Bug Fixes

* **delete:** confirm a delete by re-reading the SIM, not by per-command replies ([712cbef](https://example.invalid/commit/712cbef))
* **ports:** probe modems before work so empty slots cost ~1.6s not 24s ([e50d0b0](https://example.invalid/commit/e50d0b0))
`;

const PREVIEW_TOTAL_BYTES = 8_400_000;
const CHUNK = 240_000;
const TICK_MS = 60;

export function previewUpdate(currentVersion: string): UpdateSource {
  return {
    version: '9.9.9',
    currentVersion,
    date: '2026-08-29 12:00:00.000 +00:00:00',
    body: PREVIEW_BODY,

    async download(onEvent) {
      onEvent?.({ event: 'Started', data: { contentLength: PREVIEW_TOTAL_BYTES } });
      let sent = 0;
      while (sent < PREVIEW_TOTAL_BYTES) {
        await new Promise((r) => setTimeout(r, TICK_MS));
        const chunkLength = Math.min(CHUNK, PREVIEW_TOTAL_BYTES - sent);
        sent += chunkLength;
        onEvent?.({ event: 'Progress', data: { chunkLength } });
      }
      onEvent?.({ event: 'Finished' });
    },

    async install() {
      await new Promise((r) => setTimeout(r, 900));
    },

    async relaunch() {
      // The closest a browser gets to an app restart.
      window.location.reload();
    },

    async close() {},
  };
}
