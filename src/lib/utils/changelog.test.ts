// Zero-dependency unit tests for the CHANGELOG.md document parser behind the
// Changelog page. Run with `npm test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  bumpKind,
  countByFilter,
  filterReleases,
  normalizeVersion,
  parseChangelog,
} from './changelog.ts';

/**
 * A slice of the real file: a linked heading with both a pull-request and a
 * commit reference, a release with two sections, and the oldest entry, whose
 * heading carries no compare link and whose lines carry no references at all.
 */
const DOC = `# Changelog

## [1.6.2](https://github.com/RyanWez/sms-deliverer/compare/v1.6.1...v1.6.2) (2026-09-04)


### Bug Fixes

* report what the status lines and logs actually count ([#28](https://github.com/RyanWez/sms-deliverer/issues/28)) ([e7da48b](https://github.com/RyanWez/sms-deliverer/commit/e7da48bf84))

## [1.6.0](https://github.com/RyanWez/sms-deliverer/compare/v1.5.0...v1.6.0) (2026-09-03)


### Features

* forward live SMS and OTPs into the Telegram group ([88144a4](https://github.com/RyanWez/sms-deliverer/commit/88144a4992))


### Bug Fixes

* **sim:** key SIM numbers on ICCID, not on the serial port ([31d3e5b](https://github.com/RyanWez/sms-deliverer/commit/31d3e5bf83))

## 1.0.1 (2026-08-27)

### Bug Fixes

* restore updater ACL permissions (\`updater:default\`) so update checks execute

### Build

* reset all project versions to 1.0.1
`;

test('every release heading becomes one entry, newest first', () => {
  const out = parseChangelog(DOC);
  assert.deepEqual(
    out.map((r) => [r.version, r.date, r.changeCount]),
    [
      ['1.6.2', '2026-09-04', 1],
      ['1.6.0', '2026-09-03', 2],
      ['1.0.1', '2026-08-27', 2],
    ],
  );
});

test('the document title is not mistaken for a release', () => {
  assert.ok(!parseChangelog(DOC).some((r) => r.version === 'Changelog'));
});

test('a heading with no compare link still yields its version and date', () => {
  // The first entry was written by hand, before release-please took over.
  const oldest = parseChangelog(DOC).at(-1)!;
  assert.equal(oldest.version, '1.0.1');
  assert.equal(oldest.date, '2026-08-27');
});

test('pull-request and commit references never reach the change text', () => {
  // The whole point of parsing rather than rendering the markdown: neither
  // `#28` nor `e7da48b` leads anywhere from inside the app.
  const [latest] = parseChangelog(DOC);
  assert.equal(
    latest.sections[0].items[0].text,
    'report what the status lines and logs actually count',
  );
  const everyLine = parseChangelog(DOC)
    .flatMap((r) => r.sections)
    .flatMap((s) => s.items)
    .map((i) => i.text);
  assert.ok(!everyLine.some((t) => t.includes('#28')));
  assert.ok(!everyLine.some((t) => t.includes('e7da48b')));
  assert.ok(!everyLine.some((t) => t.includes('http')));
});

test('sections keep their kind and a scope is lifted out of the line', () => {
  const [, minor] = parseChangelog(DOC);
  assert.deepEqual(
    minor.sections.map((s) => [s.title, s.kind]),
    [
      ['Features', 'feature'],
      ['Bug Fixes', 'fix'],
    ],
  );
  assert.deepEqual(minor.sections[1].items[0], {
    scope: 'sim',
    text: 'key SIM numbers on ICCID, not on the serial port',
  });
});

test('a Build section is carried through as neither a feature nor a fix', () => {
  const oldest = parseChangelog(DOC).at(-1)!;
  assert.deepEqual(
    oldest.sections.map((s) => [s.title, s.kind]),
    [
      ['Bug Fixes', 'fix'],
      ['Build', 'other'],
    ],
  );
});

test('the bump is read off the numbers, against the entry below', () => {
  const out = parseChangelog(DOC);
  assert.equal(out[0].bump, 'patch'); // 1.6.0 → 1.6.2
  assert.equal(out[1].bump, 'minor'); // 1.0.1 → 1.6.0
  // Nothing sits below the oldest entry to compare it against, so the page has
  // nothing truthful to label it with.
  assert.equal(out[2].bump, null);
});

test('bumpKind names the field that moved', () => {
  assert.equal(bumpKind('2.0.0', '1.9.4'), 'major');
  assert.equal(bumpKind('1.7.0', '1.6.2'), 'minor');
  assert.equal(bumpKind('1.6.3', '1.6.2'), 'patch');
  // A version with a pre-release suffix must not read as a major bump just
  // because the suffix confuses the split.
  assert.equal(bumpKind('1.6.3-rc.1', '1.6.2'), 'patch');
});

test('a release whose body is empty still appears, with no changes', () => {
  const out = parseChangelog('# Changelog\n\n## [1.2.3](x) (2026-01-01)\n\n## 1.2.2 (2025-12-31)\n');
  assert.equal(out.length, 2);
  assert.deepEqual(out[0].sections, []);
  assert.equal(out[0].changeCount, 0);
});

test('an empty or missing file yields no releases', () => {
  assert.deepEqual(parseChangelog(''), []);
  assert.deepEqual(parseChangelog(null), []);
  assert.deepEqual(parseChangelog(undefined), []);
  assert.deepEqual(parseChangelog('   \n\n  '), []);
});

test('a filter drops the releases it would leave empty', () => {
  const out = parseChangelog(DOC);
  assert.deepEqual(
    filterReleases(out, 'feature').map((r) => r.version),
    ['1.6.0'],
  );
  assert.deepEqual(
    filterReleases(out, 'fix').map((r) => [r.version, r.changeCount]),
    [
      ['1.6.2', 1],
      ['1.6.0', 1],
      ['1.0.1', 1],
    ],
  );
  assert.deepEqual(
    filterReleases(out, 'other').map((r) => r.version),
    ['1.0.1'],
  );
  assert.equal(filterReleases(out, 'all'), out);
});

test('filter counts add up to the total number of change lines', () => {
  const counts = countByFilter(parseChangelog(DOC));
  assert.deepEqual(counts, { all: 5, feature: 1, fix: 3, other: 1 });
  assert.equal(counts.feature + counts.fix + counts.other, counts.all);
});

test('a version off the shell compares as written in the file', () => {
  assert.equal(normalizeVersion('v1.6.2'), '1.6.2');
  assert.equal(normalizeVersion(' 1.6.2 '), '1.6.2');
  assert.equal(normalizeVersion(null), '');
  assert.equal(normalizeVersion(undefined), '');
});
