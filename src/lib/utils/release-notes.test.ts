// Zero-dependency unit tests for the release-notes parser and the update
// check rate limit. Run with `npm test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  parseReleaseNotes,
  countNoteItems,
  formatReleaseDate,
} from './release-notes.ts';
import {
  cooldownRemaining,
  cooldownSeconds,
  canCheck,
  formatBytes,
  downloadPercent,
  MANUAL_COOLDOWN_MS,
  BACKGROUND_MIN_GAP_MS,
} from './update-policy.ts';

/** A body in exactly the shape release-please writes. */
const RELEASE_PLEASE_BODY = `## [1.3.0](https://github.com/RyanWez/sms-deliverer/compare/v1.2.0...v1.3.0) (2026-08-29)


### Features

* add expand/collapse toggle for long message text ([091c48d](https://github.com/RyanWez/sms-deliverer/commit/091c48d))
* **ports:** probe modems before work so empty slots cost ~1.6s not 24s ([e50d0b0](https://github.com/RyanWez/sms-deliverer/commit/e50d0b0))


### Bug Fixes

* **delete:** confirm a delete by re-reading the SIM ([712cbef](https://github.com/RyanWez/sms-deliverer/commit/712cbef))
`;

test('release-please sections are split by kind', () => {
  const out = parseReleaseNotes(RELEASE_PLEASE_BODY);
  assert.deepEqual(
    out.map((s) => [s.title, s.kind, s.items.length]),
    [
      ['Features', 'feature', 2],
      ['Bug Fixes', 'fix', 1],
    ],
  );
  assert.equal(countNoteItems(out), 3);
});

test('the version heading is dropped, not shown as a change', () => {
  // The card prints the version in its own header; leaving the `## [1.3.0](…)`
  // line in the list reads like a stray entry.
  const out = parseReleaseNotes(RELEASE_PLEASE_BODY);
  assert.ok(!out.some((s) => s.title.includes('1.3.0')));
});

test('commit links are stripped and the scope is lifted out', () => {
  const [features] = parseReleaseNotes(RELEASE_PLEASE_BODY);
  assert.deepEqual(features.items[1], {
    scope: 'ports',
    text: 'probe modems before work so empty slots cost ~1.6s not 24s',
  });
  assert.equal(features.items[0].scope, null);
  assert.ok(!features.items[0].text.includes('http'));
  assert.ok(!features.items[0].text.includes('091c48d'));
});

test('a link in the middle of a line keeps its label', () => {
  const out = parseReleaseNotes('### Notes\n\n* see [the docs](https://example.com/x) first');
  assert.equal(out[0].items[0].text, 'see the docs first');
});

test('a plain-prose body still renders as one section', () => {
  const out = parseReleaseNotes('Hotfix for the SIM cleanup sweep.');
  assert.equal(out.length, 1);
  assert.equal(out[0].title, "What's new");
  assert.equal(out[0].items[0].text, 'Hotfix for the SIM cleanup sweep.');
});

test('an empty or missing body yields no sections', () => {
  assert.deepEqual(parseReleaseNotes(''), []);
  assert.deepEqual(parseReleaseNotes(null), []);
  assert.deepEqual(parseReleaseNotes(undefined), []);
  assert.deepEqual(parseReleaseNotes('   \n\n  '), []);
});

test('a heading with no items under it is not rendered', () => {
  const out = parseReleaseNotes('### Features\n\n### Bug Fixes\n\n* fixed a thing');
  assert.deepEqual(out.map((s) => s.title), ['Bug Fixes']);
});

test('release dates are read out of the string, not parsed by Date', () => {
  // Tauri hands back its own format, which `new Date()` rejects outright.
  assert.equal(formatReleaseDate('2026-08-29 12:00:00.000 +00:00:00'), '29 Aug 2026');
  assert.equal(formatReleaseDate('2026-01-05T07:30:00Z'), '5 Jan 2026');
  assert.equal(formatReleaseDate(null), '');
  assert.equal(formatReleaseDate('not a date'), '');
});

/* ── rate limiting ── */

test('a fresh session may check immediately', () => {
  assert.equal(cooldownRemaining(null, 1_000_000), 0);
  assert.ok(canCheck({ busy: false, lastCheckedAt: null, now: 0, interactive: true }));
});

test('a manual check is refused for a minute after the last one', () => {
  const t = 5_000_000;
  assert.equal(cooldownRemaining(t, t + 10_000), MANUAL_COOLDOWN_MS - 10_000);
  assert.equal(cooldownSeconds(cooldownRemaining(t, t + 10_500)), 50);
  assert.ok(!canCheck({ busy: false, lastCheckedAt: t, now: t + 10_000, interactive: true }));
  assert.ok(canCheck({ busy: false, lastCheckedAt: t, now: t + MANUAL_COOLDOWN_MS, interactive: true }));
});

test('holding the button down cannot queue a second request', () => {
  assert.ok(!canCheck({ busy: true, lastCheckedAt: null, now: 0, interactive: true }));
});

test('background checks keep a much wider gap than the button', () => {
  const t = 1_000_000;
  const justUnder = t + BACKGROUND_MIN_GAP_MS - 1;
  assert.ok(!canCheck({ busy: false, lastCheckedAt: t, now: justUnder, interactive: false }));
  // The same instant is fine for a manual check, which only waits a minute.
  assert.ok(canCheck({ busy: false, lastCheckedAt: t, now: justUnder, interactive: true }));
});

test('a clock that jumps backwards does not strand the button', () => {
  assert.equal(cooldownRemaining(9_000_000, 1_000), 0);
});

test('byte and percentage labels', () => {
  assert.equal(formatBytes(0), '');
  assert.equal(formatBytes(-5), '');
  assert.equal(formatBytes(2048), '2 KB');
  assert.equal(formatBytes(5 * 1024 * 1024), '5.0 MB');
  // An endpoint that omits Content-Length leaves the bar indeterminate rather
  // than showing a made-up percentage.
  assert.equal(downloadPercent(1000, 0), null);
  assert.equal(downloadPercent(512, 1024), 50);
  assert.equal(downloadPercent(9999, 1024), 100);
});
