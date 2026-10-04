import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { classifyAuthError, AuthError, AUTH_TOKEN_BODY_MAX_BYTES } from './auth-policy.mjs';
import { AuthManager } from './workbench-auth.mjs';
import { createAuthDiagnostics, AUTH_DIAGNOSTIC_MAX_BYTES, AUTH_DIAGNOSTIC_MAX_RECORDS } from './auth-diagnostics.mjs';

const context = { providerId: 'openai', method: 'oauth' };
const sentinel = 'SYNTHETIC_SECRET_https://example.invalid/callback?code=synthetic';

test('offline auth failure preserves safe OAuth category and status, not SDK payload or prompt cancellation', async () => {
  const root = await fs.mkdtemp(path.join(await fs.realpath(os.tmpdir()), 'pad-auth-diag-'));
  try {
    const write = createAuthDiagnostics({ dataRoot: root });
    const writes = [];
    const frames = [];
    let resolveTerminal;
    const terminal = new Promise((resolve) => { resolveTerminal = resolve; });
    const sdk = {
      providerStatus: async () => ({ authTypes: ['oauth'] }),
      login: async ({ interaction }) => {
        interaction.notify({ type: 'auth_url', url: `https://example.invalid/${sentinel}` });
        const manual = new AbortController();
        const prompt = interaction.prompt({ type: 'manual_code', message: 'Synthetic prompt', signal: manual.signal });
        const settled = prompt.catch(() => {});
        interaction.notify({ type: 'progress', message: 'Exchanging authorization code for tokens...' });
        // SDK-local prompt abort is not a user abort of the overall attempt.
        manual.abort();
        await settled;
        throw new Error(`OpenAI OAuth token request failed (403): ${JSON.stringify({ error: 'invalid_grant', error_description: sentinel, message: sentinel })}`);
      },
    };
    const manager = new AuthManager({
      requireProfile: () => ({ id: 'synthetic-profile' }), isClosed: () => false,
      profileBusy: () => false, agentDirFor: () => '/synthetic-not-read', sdkClient: () => sdk,
      emit: (frame) => { frames.push(frame); if (frame.data.phase === 'failed') resolveTerminal(frame.data); },
      diagnostic: () => {}, makeError: (message) => new Error(message), onCredentialsChanged: async () => {},
      recordAuthDiagnostic: (record) => { const promise = write(record); writes.push(promise); return promise; },
    });
    await manager.begin('synthetic-profile', 'openai', 'oauth');
    const state = await terminal;
    await Promise.all(writes);
    assert.match(state.message, /token exchange.*HTTP 403/);
    assert.equal(state.url, undefined);
    assert.equal(state.message.includes(sentinel), false);
    assert.equal(frames.some((frame) => frame.data.phase === 'cancelled'), false);
    const text = await fs.readFile(path.join(root, 'v1/auth-diagnostics.jsonl'), 'utf8');
    assert.equal(text.includes(sentinel), false);
    assert.equal(text.includes('synthetic-profile'), false);
    const failure = text.trim().split('\n').map(JSON.parse).at(-1);
    assert.equal(failure.httpStatus, 403);
    assert.equal(failure.phase, 'token_exchange');
    assert.equal(failure.category, 'token_oauth_invalid_grant');
    assert.match(state.message, /does not establish expiry or reuse/);
    assert.deepEqual(Object.keys(failure).sort(), ['category', 'httpStatus', 'method', 'outcome', 'phase', 'providerId', 'timestamp'].sort());
    const unknown = classifyAuthError(new Error(sentinel), context);
    assert.equal(unknown.category, 'unknown');
    assert.equal(unknown.message.includes(sentinel), false);
    const transport = classifyAuthError(new Error(sentinel, { cause: { code: 'ECONNRESET', message: sentinel } }), context);
    assert.equal(transport.errno, 'ECONNRESET');
    assert.equal(transport.phase, 'unknown');
    assert.equal(transport.message.includes(sentinel), false);
    assert.equal(classifyAuthError(new Error(`OpenAI OAuth token request failed (999): ${sentinel}`), context).category, 'unknown');
    assert.equal(classifyAuthError(new AuthError('credential_sync'), context).category, 'credential_sync');
  } finally { await fs.rm(root, { recursive: true, force: true }); }
});

test('bounded synthetic rejection bodies expose only fixed categories and approved status', async () => {
  const root = await fs.mkdtemp(path.join(await fs.realpath(os.tmpdir()), 'pad-auth-diag-'));
  const arbitraryCode = 'SYNTHETIC_ARBITRARY_CODE';
  const classify = (body, ctx = context, prefix = 'OpenAI OAuth token request failed (403): ') =>
    classifyAuthError(new Error(prefix + body), ctx);
  try {
    const write = createAuthDiagnostics({ dataRoot: root });
    const cases = [
      ...['invalid_request', 'invalid_client', 'invalid_grant', 'unauthorized_client', 'unsupported_grant_type', 'invalid_scope']
        .map((error) => [JSON.stringify({ error, error_description: sentinel, message: sentinel }), `token_oauth_${error}`]),
      [JSON.stringify({ error: arbitraryCode, error_description: sentinel }), 'token_json_unrecognized'],
      [JSON.stringify({ error: { code: 'invalid_grant', type: arbitraryCode, message: sentinel } }), 'token_json_unrecognized'],
      [` \n<!DOCTYPE html><html><body>${sentinel}</body></html>`, 'token_html_response'],
      [`<html lang="en"><body>${sentinel}</body></html>`, 'token_html_response'],
      [JSON.stringify({ error: 'invalid_grant', error_description: 'x'.repeat(AUTH_TOKEN_BODY_MAX_BYTES) }), 'token_body_unrecognized'],
      [JSON.stringify({ error: 'invalid_grant', error_description: 'é'.repeat(AUTH_TOKEN_BODY_MAX_BYTES / 2) }), 'token_body_unrecognized'],
      [`{"error":"invalid_grant","message":"${sentinel}"`, 'token_body_unrecognized'],
      ['["invalid_grant"]', 'token_body_unrecognized'],
      [`Forbidden ${sentinel}`, 'token_body_unrecognized'],
      [`prefix <html>${sentinel}</html>`, 'token_body_unrecognized'],
    ];
    for (const [body, category] of cases) {
      const safe = classify(body);
      assert.equal(safe.category, category);
      assert.equal(safe.httpStatus, 403);
      assert.match(safe.message, /HTTP 403/);
      for (const secret of [sentinel, 'https://', arbitraryCode]) assert.equal(JSON.stringify(safe).includes(secret), false);
      assert.equal(await write({ ...context, ...safe, outcome: 'failed', body, code: arbitraryCode, headers: sentinel, requestId: sentinel }), true);
    }
    const text = await fs.readFile(path.join(root, 'v1/auth-diagnostics.jsonl'), 'utf8');
    const records = text.trim().split('\n').map(JSON.parse);
    assert.deepEqual(records.map((record) => record.category), cases.map(([, category]) => category));
    for (const record of records) assert.deepEqual(Object.keys(record).sort(), ['category', 'httpStatus', 'method', 'outcome', 'phase', 'providerId', 'timestamp'].sort());
    for (const secret of [sentinel, 'https://', arbitraryCode]) assert.equal(text.includes(secret), false);

    const body = JSON.stringify({ error: 'invalid_grant', error_description: sentinel });
    const guards = [
      classify(body, { ...context, method: 'api_key' }),
      classify(body, { ...context, providerId: 'other' }),
      classify(body, context, 'OpenAI Codex token exchange failed (403): '),
      classify(body, { providerId: 'openai-codex', method: 'oauth' }),
      classify(body, context, 'prefix OpenAI OAuth token request failed (403): '),
      classify(body, context, 'OpenAI OAuth token request failed (600): '),
    ];
    for (const safe of guards) {
      assert.equal(safe.category, 'unknown');
      assert.equal(safe.httpStatus, undefined);
      assert.equal(safe.message.includes(sentinel), false);
    }
  } finally { await fs.rm(root, { recursive: true, force: true }); }
});

test('private bounded whitelist persistence refuses a symlink target', async () => {
  const root = await fs.mkdtemp(path.join(await fs.realpath(os.tmpdir()), 'pad-auth-diag-'));
  try {
    const write = createAuthDiagnostics({ dataRoot: root });
    const event = { ...context, phase: 'unknown', category: 'unknown', outcome: 'failed', message: sentinel, errno: sentinel };
    for (let i = 0; i < AUTH_DIAGNOSTIC_MAX_RECORDS + 2; i++) assert.equal(await write(event), true);
    const file = path.join(root, 'v1/auth-diagnostics.jsonl');
    const text = await fs.readFile(file, 'utf8');
    assert.equal(text.trim().split('\n').length, AUTH_DIAGNOSTIC_MAX_RECORDS);
    assert.ok(Buffer.byteLength(text) <= AUTH_DIAGNOSTIC_MAX_BYTES);
    assert.equal(text.includes(sentinel), false);
    assert.equal((await fs.stat(root)).mode & 0o777, 0o700);
    assert.equal((await fs.stat(path.join(root, 'v1'))).mode & 0o777, 0o700);
    assert.equal((await fs.stat(file)).mode & 0o777, 0o600);
    assert.equal(await write({ ...event, providerId: sentinel }), false);
    await fs.unlink(file);
    const target = path.join(root, 'sentinel-target');
    await fs.writeFile(target, sentinel);
    await fs.symlink(target, file);
    assert.equal(await write(event), false);
    assert.equal(await fs.readFile(target, 'utf8'), sentinel);
    const badRoot = path.join(root, 'linked-root');
    await fs.symlink(root, badRoot);
    assert.equal(await createAuthDiagnostics({ dataRoot: badRoot })(event), false);
  } finally { await fs.rm(root, { recursive: true, force: true }); }
});
