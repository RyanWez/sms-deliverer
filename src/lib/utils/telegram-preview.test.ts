import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  looksLikeBotToken,
  previewBotName,
  previewDetectGroup,
  previewSendTest,
  previewVerifyToken,
} from './telegram-preview.ts';

const TOKEN = '1234567890:AAF3xK-secret';

test('looksLikeBotToken accepts the BotFather shape', () => {
  assert.equal(looksLikeBotToken(TOKEN), true);
  assert.equal(looksLikeBotToken(`  ${TOKEN}  `), true);
});

test('looksLikeBotToken rejects a username, an empty half, and extra colons', () => {
  assert.equal(looksLikeBotToken('@my_otp_bot'), false);
  assert.equal(looksLikeBotToken(''), false);
  assert.equal(looksLikeBotToken('abcdef:AAF3xK'), false);
  assert.equal(looksLikeBotToken('1234567890:'), false);
  assert.equal(looksLikeBotToken(':AAF3xK'), false);
  assert.equal(looksLikeBotToken('123:AA:BB'), false);
});

test('previewBotName varies with the token so two tokens do not look alike', () => {
  assert.notEqual(previewBotName(TOKEN), previewBotName('9999999999:AAF3xK-secret'));
});

test('previewVerifyToken resolves a bot name for a well-formed token', async () => {
  assert.match(await previewVerifyToken(TOKEN), /^@preview_bot_/);
});

test('previewVerifyToken rejects with the same wording as the Rust command', async () => {
  await assert.rejects(
    () => previewVerifyToken('   '),
    (e) => e === 'Enter the bot token from @BotFather first',
  );
  await assert.rejects(
    () => previewVerifyToken('@my_otp_bot'),
    (e) => typeof e === 'string' && e.includes('does not look like a bot token'),
  );
});

test('previewDetectGroup returns a basic group, so the upgrade warning is reachable', async () => {
  const group = await previewDetectGroup(TOKEN);
  assert.equal(group.kind, 'group');
  assert.ok(!group.chatId.startsWith('-100'));
});

test('previewSendTest reports no migration for a supergroup id', async () => {
  const result = await previewSendTest(TOKEN, '-1001234567890');
  assert.equal(result.migratedChatId, null);
  assert.match(result.bot, /^@preview_bot_/);
});

test('previewSendTest simulates the supergroup migration for a basic group id', async () => {
  const result = await previewSendTest(TOKEN, '-4855120394');
  assert.equal(result.migratedChatId, '-1004855120394');
});

test('previewSendTest rejects an empty or non-group chat id', async () => {
  await assert.rejects(
    () => previewSendTest(TOKEN, '  '),
    (e) => typeof e === 'string' && e.includes('Detect Group ID'),
  );
  await assert.rejects(
    () => previewSendTest(TOKEN, '1234567'),
    (e) => typeof e === 'string' && e.includes('chat not found'),
  );
});
