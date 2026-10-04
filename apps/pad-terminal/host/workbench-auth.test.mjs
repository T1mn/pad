import test from 'node:test';
import assert from 'node:assert/strict';
import { AuthManager } from './workbench-auth.mjs';
import { AuthError, safeAuthMessage } from './auth-policy.mjs';
import { WorkbenchHost } from './workbench.mjs';

function fixture(login, cleanup = () => {}) {
  const frames = [];
  const diagnostics = [];
  let changed = 0;
  let calls = 0;
  const manager = new AuthManager({
    requireProfile: () => ({ id: 'p' }), isClosed: () => false, profileBusy: () => false,
    agentDirFor: () => '/offline-profile',
    sdkClient: () => ({ providerStatus: async () => ({ authTypes: ['oauth', 'api_key'] }),
      login: async (options) => { calls++; return login(options); } }),
    emit: (frame) => frames.push(frame), diagnostic: (message) => diagnostics.push(message),
    makeError: (message) => new Error(message),
    onCredentialsChanged: () => { changed++; return cleanup(); },
  });
  return { manager, frames, diagnostics, changed: () => changed, calls: () => calls };
}
const settle = () => new Promise((resolve) => setImmediate(resolve));
test('SDK failures redact sentinel secrets and preserve committed distinction without retry', async () => {
  const secret = 'SENTINEL-SECRET https://secret.invalid/?token=secret';
  const ordinary = fixture(async () => { throw new Error(secret, { cause: { body: secret } }); });
  await ordinary.manager.begin('p', 'openai', 'oauth', 'attempt');
  await settle();
  assert.equal(ordinary.manager.states.get('p').phase, 'failed');
  assert.equal(JSON.stringify([ordinary.frames, ordinary.diagnostics]).includes('SENTINEL'), false);
  assert.equal(ordinary.changed(), 0);
  assert.doesNotMatch(ordinary.manager.states.get('p').message, /network/i);
  const committed = fixture(async () => { throw new AuthError('credential_sync'); });
  await committed.manager.begin('p', 'openai', 'oauth', 'attempt');
  await settle();
  assert.match(committed.manager.states.get('p').message, /Credentials were saved/);
  assert.equal(committed.changed(), 1);
  assert.equal(committed.calls(), 1);
  assert.match(safeAuthMessage(new Error('Sign in with ChatGPT requires a device ID (UUID) for this installation')), /Installation identity/);
  assert.equal(safeAuthMessage(new Error(`Sign in with ChatGPT requires a device ID (UUID) for this installation ${secret}`)), 'Authentication failed. Please try again.');
});
test('authorization notices survive the following manual-code prompt until completion', async () => {
  for (const notice of [
    { type: 'auth_url', url: 'https://offline.invalid/authorize' },
    { type: 'device_code', verificationUri: 'https://offline.invalid/device', userCode: 'OFFLINE' },
  ]) {
    const f = fixture(async ({ interaction }) => {
      interaction.notify(notice);
      await interaction.prompt({ type: 'manual_code', message: 'Paste the code' });
    });
    await f.manager.begin('p', 'openai', 'oauth', 'notice-attempt');
    const state = f.manager.states.get('p');
    assert.equal(state.promptKind, 'manual_code');
    assert.equal(state.url, notice.url ?? notice.verificationUri);
    assert.equal(state.userCode, notice.userCode);
    f.manager.respond('p', state.promptId, 'offline-code');
    assert.equal(f.manager.states.get('p').url, state.url);
    await settle();
    assert.equal(f.manager.states.get('p').phase, 'succeeded');
    assert.equal(f.manager.states.get('p').url, undefined);
    assert.equal(f.manager.states.get('p').userCode, undefined);
  }
});
test('credential cleanup retains the auth reservation and defers terminal state even on cancel', async () => {
  for (const cancel of [false, true]) {
    let completeCleanup;
    const cleanup = new Promise((resolve) => { completeCleanup = resolve; });
    const f = fixture(async () => {}, () => cleanup);
    await f.manager.begin('p', 'openai', 'oauth', 'cleanup-attempt');
    await settle();
    assert.equal(f.changed(), 1);
    assert.equal(f.manager.isAuthBusy('p'), true);
    assert.equal(f.manager.states.get('p').phase, 'running');
    await assert.rejects(f.manager.begin('p', 'openai', 'oauth', 'next'), /already in progress/);
    assert.throws(() => f.manager.reserveLogout('p'), /authentication is in progress/);
    if (cancel) f.manager.cancel('p', 'cleanup-attempt');
    assert.equal(f.manager.isAuthBusy('p'), true);
    assert.equal(f.frames.some((frame) => ['succeeded', 'failed', 'cancelled'].includes(frame.data.phase)), false);
    completeCleanup();
    await settle();
    assert.equal(f.manager.isAuthBusy('p'), false);
    assert.equal(f.manager.states.get('p').phase, cancel ? 'cancelled' : 'succeeded');
  }
});
test('cancelled attempts cannot publish a later success or refresh peers', async () => {
  let complete;
  const f = fixture(() => new Promise((resolve) => { complete = resolve; }));
  await f.manager.begin('p', 'openai', 'oauth', 'first');
  f.manager.cancel('p', 'stale');
  assert.equal(f.manager.isAuthBusy('p'), true);
  f.manager.cancel('p', 'first');
  complete();
  await settle();
  assert.equal(f.manager.states.get('p').phase, 'cancelled');
  assert.equal(f.changed(), 0);
});
test('model availability blocks setModel and prompt before any Pi RPC and preserves binding', async () => {
  const task = { id: 't', profileId: 'p', provider: 'anthropic', modelId: 'one', status: 'idle' };
  const host = Object.create(WorkbenchHost.prototype);
  Object.assign(host, {
    store: { state: { tasks: [task] } }, activeRuns: new Set(), taskOps: new Map(), closed: false,
    auth: { isAuthBusy: () => false }, profileDirs: () => ({ agent: '/offline-profile' }),
    sdkClient: () => ({ modelAvailable: async () => false }),
    runtime: { get: () => { throw new Error('must not reach RPC'); } },
  });
  await assert.rejects(host.setModel('t', 'anthropic', 'one'), /not available/);
  await assert.rejects(host.prompt('t', 'offline message'), /no credentials/);
  assert.equal(task.provider, 'anthropic');
  assert.equal(task.modelId, 'one');
  assert.equal(host.taskOps.size, 0);
});
