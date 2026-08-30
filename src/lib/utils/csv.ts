// CSV serialization for the inbox export.
//
// Pure string logic, deliberately free of runes and of `$lib` imports so
// `npm test` can exercise it directly (same rationale as port-refresh.ts).
//
// Two problems are solved here and they are independent:
//
//  1. RFC-4180 quoting, so an embedded quote/comma/newline round-trips.
//  2. Formula-injection neutralization, because every cell in this file is
//     attacker-controlled: anyone who can send this SIM bank an SMS controls
//     the sender number and the body, and the operator opens the export in
//     Excel / LibreOffice Calc / Sheets themselves.
//
// Quoting alone does NOT solve (2): the quotes are consumed by the CSV parser
// before the cell content is evaluated, so `"=cmd|'…'!A1"` is imported as a
// formula exactly like the unquoted form. The mitigation has to change the
// cell's first character, which is what `guardFormula` does.

/** Fixed column order of the CSV export; also used for the header row. */
export const CSV_COLUMNS = ['time', 'from', 'port', 'sim', 'text', 'otp', 'status'] as const;

export type CsvColumn = (typeof CSV_COLUMNS)[number];
export type CsvRow = Record<CsvColumn, string>;

/**
 * Leading characters a spreadsheet treats as the start of an expression.
 *
 * `=` and `+` start a formula; `-` starts a negated expression; `@` starts a
 * function/name reference in Excel; TAB and CR are here because they are
 * consumed as leading whitespace by some importers, which promotes the next
 * character to first position.
 */
const FORMULA_LEAD = /^[=+\-@\t\r]/;

/**
 * True when a spreadsheet could evaluate this cell instead of showing it.
 *
 * Checked both on the raw value and with leading whitespace stripped, so
 * `" =1+1"` cannot smuggle a formula past an importer that trims.
 */
export function isFormulaRisk(value: string): boolean {
  if (value.length === 0) return false;
  return FORMULA_LEAD.test(value) || FORMULA_LEAD.test(value.replace(/^\s+/, ''));
}

/**
 * Neutralize a formula-triggering cell by prefixing an apostrophe.
 *
 * The apostrophe is the conventional mitigation (OWASP) and is the choice here
 * over a leading TAB for one reason: the operator copies these cells (a sender
 * number, an OTP) out of the sheet by hand. A TAB prefix is invisible, so a
 * corrupted value would be pasted onwards unnoticed; an apostrophe is visible,
 * and Excel additionally hides it as its own "this is text" marker. A visible
 * apostrophe on the rare dangerous cell is a cosmetic cost the operator can
 * see and reason about.
 *
 * Note this fires on international sender numbers (`+8613800138000`), which is
 * intended: Excel evaluates a leading `+` as an expression and would silently
 * drop the plus even in the benign case.
 */
export function guardFormula(value: string): string {
  return isFormulaRisk(value) ? `'${value}` : value;
}

/**
 * Serialize one cell: neutralize first, then RFC-4180 quote.
 *
 * Neutralized cells are always quoted as well, even when the content would not
 * otherwise require it. Quoting is not what makes them safe, but it keeps the
 * apostrophe unambiguously part of the field for every parser.
 */
export function csvCell(value: string | null | undefined): string {
  const raw = value ?? '';
  const guarded = guardFormula(raw);
  if (guarded !== raw || /[",\n\r]/.test(guarded)) {
    return `"${guarded.replace(/"/g, '""')}"`;
  }
  return guarded;
}

/** Serialize export rows to CSV, header included, with a trailing newline. */
export function toCsv(rows: readonly Partial<CsvRow>[]): string {
  const header = CSV_COLUMNS.join(',');
  if (rows.length === 0) return `${header}\n`;
  const body = rows
    .map((row) => CSV_COLUMNS.map((col) => csvCell(row[col])).join(','))
    .join('\n');
  return `${header}\n${body}\n`;
}
