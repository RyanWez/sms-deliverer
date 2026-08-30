// Unit tests for the CSV export serializer.
//
// Plain functions, Node's built-in runner — no DOM, no rune stores (same
// rationale as port-refresh.test.ts).

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { CSV_COLUMNS, csvCell, guardFormula, isFormulaRisk, toCsv } from './csv.ts';

test('an ordinary cell is emitted verbatim', () => {
  assert.equal(csvCell('Your code is 123456'), 'Your code is 123456');
  assert.equal(csvCell('8613800138000'), '8613800138000');
  assert.equal(csvCell('COM7'), 'COM7');
});

test('an empty or missing cell stays empty', () => {
  assert.equal(csvCell(''), '');
  assert.equal(csvCell(null), '');
  assert.equal(csvCell(undefined), '');
});

test('every formula-triggering leading character is neutralized', () => {
  // The five OWASP leads plus CR, each one attacker-reachable through the SMS
  // body or the sender field.
  assert.equal(csvCell('=1+1'), '"\'=1+1"');
  assert.equal(csvCell('+8613800138000'), '"\'+8613800138000"');
  assert.equal(csvCell('-4917612345'), '"\'-4917612345"');
  assert.equal(csvCell('@SUM(A1:A9)'), '"\'@SUM(A1:A9)"');
  assert.equal(csvCell('\t=1+1'), '"\'\t=1+1"');
  assert.equal(csvCell('\r=1+1'), '"\'\r=1+1"');
});

test('the DDE payload shape cannot survive as a formula', () => {
  const payload = '=cmd|\' /C calc\'!A0';
  const cell = csvCell(payload);
  // Quoting alone would leave the cell starting with '=' once the CSV parser
  // strips the quotes; the apostrophe is what actually defuses it.
  assert.ok(cell.startsWith('"\''));
  assert.ok(!cell.startsWith('"='));
});

test('leading whitespace cannot smuggle a formula past a trimming importer', () => {
  assert.ok(isFormulaRisk(' =1+1'));
  assert.ok(isFormulaRisk('   @A1'));
  assert.equal(csvCell(' =1+1'), '"\' =1+1"');
});

test('a normal cell is not marked as a risk', () => {
  for (const safe of ['123456', 'OTP: 4021', 'a=b', 'x+y', 'user@example.com', '', ' hello']) {
    assert.equal(isFormulaRisk(safe), false, safe);
    assert.equal(guardFormula(safe), safe);
  }
});

test('embedded quotes are doubled and the cell is quoted', () => {
  assert.equal(csvCell('say "hi"'), '"say ""hi"""');
});

test('an embedded comma quotes the cell', () => {
  assert.equal(csvCell('Hello, world'), '"Hello, world"');
});

test('an embedded newline quotes the cell and keeps the break', () => {
  assert.equal(csvCell('line1\nline2'), '"line1\nline2"');
  assert.equal(csvCell('line1\r\nline2'), '"line1\r\nline2"');
});

test('a dangerous cell that also needs quoting gets both, with quotes doubled', () => {
  assert.equal(csvCell('=HYPERLINK("http://x","go"),now'), '"\'=HYPERLINK(""http://x"",""go""),now"');
});

test('an empty export still carries the header', () => {
  assert.equal(toCsv([]), `${CSV_COLUMNS.join(',')}\n`);
});

test('a row is serialized in the fixed column order', () => {
  const csv = toCsv([
    {
      time: '2026-08-30 12:00:00',
      from: '+8613800138000',
      port: 'COM7',
      sim: '-4917612345',
      text: 'Your code is 123456',
      otp: '123456',
      status: 'REC READ',
    },
  ]);
  const [header, row] = csv.trimEnd().split('\n');
  assert.equal(header, 'time,from,port,sim,text,otp,status');
  assert.equal(
    row,
    '2026-08-30 12:00:00,"\'+8613800138000",COM7,"\'-4917612345",Your code is 123456,123456,REC READ',
  );
});

test('a body with a comma and a newline round-trips as one field', () => {
  const csv = toCsv([{ text: 'part one, part two\npart three', otp: '999111' }]);
  const lines = csv.split('\n');
  // header, then a field that legitimately spans two physical lines.
  assert.equal(lines[0], 'time,from,port,sim,text,otp,status');
  assert.equal(lines[1], ',,,,"part one, part two');
  assert.equal(lines[2], 'part three",999111,');
});

test('missing keys become empty cells rather than "undefined"', () => {
  assert.equal(toCsv([{ otp: '4021' }]), 'time,from,port,sim,text,otp,status\n,,,,,4021,\n');
});
