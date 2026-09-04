/**
 * Split CHANGELOG.md into the releases the Changelog page renders.
 *
 * The file is release-please's own output and is bundled into the build, so the
 * page never touches the network and the history it shows always belongs to the
 * binary that is running: `tauri-build.yml` builds from the release tag, and by
 * then the release PR has already written that version's section. The flip side
 * is that the newest entry a given install can ever show is its own version.
 *
 * Only the version headings are read here. Each release body is handed to
 * `parseReleaseNotes`, the same parser behind the update card, so one change
 * line looks identical wherever the app prints it — including the stripped
 * pull-request and commit references, which lead nowhere from inside the app.
 */

import {
  countNoteItems,
  parseReleaseNotes,
  type NoteKind,
  type NoteSection,
} from './release-notes.ts';

/** How much of the version number moved since the release below it. */
export type BumpKind = 'major' | 'minor' | 'patch';

export interface Release {
  /** `1.6.2`, without the `v`. */
  version: string;
  /** `2026-09-04`, or `''` when the heading carried no date. */
  date: string;
  /** `null` for the oldest entry, which has nothing to be compared against. */
  bump: BumpKind | null;
  sections: NoteSection[];
  changeCount: number;
}

/**
 * A release heading, in both forms the file contains: the linked
 * `## [1.6.2](…/compare/v1.6.1...v1.6.2)` release-please writes, and the bare
 * `## 1.0.1` of the first hand-written entry.
 */
const VERSION_HEADING_RE = /^##\s+\[?v?(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)/;

/**
 * The release date, read as the last `(YYYY-MM-DD)` on the heading line rather
 * than by position — the compare link that sits between the version and the
 * date is present on every entry but the oldest.
 */
const HEADING_DATE_RE = /\((\d{4}-\d{2}-\d{2})\)/g;

function headingDate(line: string): string {
  let last = '';
  for (const m of line.matchAll(HEADING_DATE_RE)) last = m[1];
  return last;
}

/** `[1, 6, 2]` from `1.6.2`; a non-numeric field reads as 0. */
function parts(version: string): [number, number, number] {
  const n = version.split('.').map((p) => Number.parseInt(p, 10) || 0);
  return [n[0] ?? 0, n[1] ?? 0, n[2] ?? 0];
}

/**
 * Which field of `version` moved relative to `previous`, the release directly
 * below it in the file.
 *
 * Read off the numbers rather than off the sections, because the two can
 * disagree: a release whose only commit is a `perf:` still bumps the patch
 * field, and release-please files that under a heading this module classifies
 * as neither a feature nor a fix.
 */
export function bumpKind(version: string, previous: string): BumpKind {
  const [ma, mi] = parts(version);
  const [pa, pi] = parts(previous);
  if (ma !== pa) return 'major';
  if (mi !== pi) return 'minor';
  return 'patch';
}

export function parseChangelog(source: string | null | undefined): Release[] {
  if (!source?.trim()) return [];

  const found: Array<Omit<Release, 'bump'>> = [];
  let version: string | null = null;
  let date = '';
  let body: string[] = [];

  function flush() {
    if (version === null) return;
    const sections = parseReleaseNotes(body.join('\n'));
    found.push({ version, date, sections, changeCount: countNoteItems(sections) });
  }

  for (const line of source.split(/\r?\n/)) {
    const heading = line.match(VERSION_HEADING_RE);
    if (heading) {
      flush();
      version = heading[1];
      date = headingDate(line);
      body = [];
      continue;
    }
    // Anything above the first release heading is the document's own title.
    if (version !== null) body.push(line);
  }
  flush();

  // The file is newest-first, so the entry that says how far a version moved is
  // the one after it.
  return found.map((r, i) => ({
    ...r,
    bump: i + 1 < found.length ? bumpKind(r.version, found[i + 1].version) : null,
  }));
}

/** `1.6.2` from `v1.6.2`, so a version off the shell compares as written here. */
export function normalizeVersion(version: string | null | undefined): string {
  return (version ?? '').trim().replace(/^v/i, '');
}

export type ChangeFilter = NoteKind | 'all';

/**
 * The same releases carrying only sections of one kind, with the ones that end
 * up empty dropped — an operator filtering to "Fixes" wants a list of fixes,
 * not ten headings above nothing.
 */
export function filterReleases(releases: Release[], filter: ChangeFilter): Release[] {
  if (filter === 'all') return releases;
  const out: Release[] = [];
  for (const r of releases) {
    const sections = r.sections.filter((s) => s.kind === filter);
    if (sections.length === 0) continue;
    out.push({ ...r, sections, changeCount: countNoteItems(sections) });
  }
  return out;
}

/** How many change lines each filter would show, for the filter labels. */
export function countByFilter(releases: Release[]): Record<ChangeFilter, number> {
  const counts: Record<ChangeFilter, number> = { all: 0, feature: 0, fix: 0, other: 0 };
  for (const r of releases) {
    for (const s of r.sections) {
      counts.all += s.items.length;
      counts[s.kind] += s.items.length;
    }
  }
  return counts;
}
