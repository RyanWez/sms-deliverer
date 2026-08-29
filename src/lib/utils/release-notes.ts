/**
 * Turn a release body into the small structure the update card renders.
 *
 * The text arrives from the release endpoint, so it is untrusted input and is
 * only ever rendered as plain text — never through `{@html}`. Everything here
 * is string slicing for that reason: no markdown renderer is involved.
 *
 * The shape it expects is what release-please writes:
 *
 *     ## [1.3.0](…compare/v1.2.0...v1.3.0) (2026-08-29)
 *
 *     ### Features
 *
 *     * **ports:** probe modems before work ([e50d0b0](…))
 *
 * A plain-prose body still comes through, as one unlabelled section.
 */

export type NoteKind = 'feature' | 'fix' | 'other';

export interface NoteItem {
  /** Conventional-commit scope (`**ports:**` → `ports`), when the line had one. */
  scope: string | null;
  text: string;
}

export interface NoteSection {
  title: string;
  kind: NoteKind;
  items: NoteItem[];
}

/** Trailing `([abc1234](url))` commit link that release-please appends. */
const COMMIT_LINK_RE = /\s*\(\[[0-9a-f]{6,40}\]\([^)]*\)\)\s*$/i;
/** Any leftover `[label](url)` — keep the label, drop the target. */
const MD_LINK_RE = /\[([^\]]*)\]\([^)]*\)/g;
const BULLET_RE = /^\s*[*+-]\s+/;
const HEADING_RE = /^\s*(#{1,6})\s+(.*)$/;
const SCOPE_RE = /^\*\*([^*:]{1,32}):\*\*\s*/;

function classify(title: string): NoteKind {
  if (/\b(feat|feature|added|new)/i.test(title)) return 'feature';
  if (/\b(fix|fixed|bug|patch)/i.test(title)) return 'fix';
  return 'other';
}

function cleanText(line: string): string {
  return line
    .replace(COMMIT_LINK_RE, '')
    .replace(MD_LINK_RE, '$1')
    .replace(/`/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

function toItem(line: string): NoteItem | null {
  let text = line.replace(BULLET_RE, '');
  const scoped = text.match(SCOPE_RE);
  const scope = scoped ? scoped[1].trim() : null;
  if (scoped) text = text.slice(scoped[0].length);
  text = cleanText(text).replace(/\*\*/g, '');
  if (!text) return null;
  return { scope, text };
}

/**
 * A version heading such as `## [1.3.0](…) (2026-08-29)`.
 *
 * Dropped because the card already shows the version and date in its own
 * header, and repeating them inside the notes reads like a stray line.
 */
function isVersionHeading(depth: number, title: string): boolean {
  return depth <= 2 && /^\[?\d+\.\d+\.\d+/.test(title.trim());
}

export function parseReleaseNotes(body: string | null | undefined): NoteSection[] {
  if (!body?.trim()) return [];

  const sections: NoteSection[] = [];
  let current: NoteSection | null = null;

  for (const raw of body.split(/\r?\n/)) {
    const line = raw.trimEnd();
    if (!line.trim()) continue;

    const heading = line.match(HEADING_RE);
    if (heading) {
      const title = cleanText(heading[2]).replace(/\*\*/g, '');
      if (!title || isVersionHeading(heading[1].length, heading[2])) continue;
      current = { title, kind: classify(title), items: [] };
      sections.push(current);
      continue;
    }

    // Bodies that open with prose have no heading to attach to yet.
    if (!current) {
      current = { title: "What's new", kind: 'other', items: [] };
      sections.push(current);
    }
    const item = toItem(line);
    if (item) current.items.push(item);
  }

  return sections.filter((s) => s.items.length > 0);
}

/** Total number of change lines across every section. */
export function countNoteItems(sections: NoteSection[]): number {
  return sections.reduce((n, s) => n + s.items.length, 0);
}

const MONTHS = [
  'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
];

/**
 * `29 Aug 2026` from a release timestamp, or `''` if there isn't one.
 *
 * The updater hands back whatever `pub_date` held, and Tauri's own form
 * (`2026-08-29 12:00:00.000 +00:00:00`) is not something `Date` will parse, so
 * the calendar date is read out of the string directly.
 */
export function formatReleaseDate(raw: string | null | undefined): string {
  const m = raw?.match(/(\d{4})-(\d{2})-(\d{2})/);
  if (!m) return '';
  const month = MONTHS[Number(m[2]) - 1];
  if (!month) return '';
  return `${Number(m[3])} ${month} ${m[1]}`;
}
