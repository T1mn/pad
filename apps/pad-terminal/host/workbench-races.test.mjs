import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { WorkbenchHost } from './workbench.mjs';
import { AuthManager } from './workbench-auth.mjs';

test('auth attempts reserve synchronously, cancel during provider validation, and ignore stale attempt ids', async () => {
  const gates = [];
  let loginCalls = 0;
  const emitted = [];
  const manager = new AuthManager({
    requireProfile: (id) => ({ id, name: 'P' }),
    isClosed: () => false,
    profileBusy: () => false,
    agentDirFor: () => '/tmp/agent',
    sdkClient: () => ({
      providerStatus: () => new Promise((resolve) => gates.push(resolve)),
      login: async () => { loginCalls += 1; },
    }),
    emit: (frame) => emitted.push(frame.data),
    diagnostic: () => {},
    makeError: (message) => new Error(message),
    onCredentialsChanged: () => {},
  });

  const first = manager.begin('p1', 'openai', 'api_key', 'attempt-a');
  // The reservation must already be held before provider validation resolves.
  await assert.rejects(manager.begin('p1', 'openai', 'api_key', 'attempt-dup'), /already in progress/);
  // A stale cancel for a different attempt must not touch the active one.
  manager.cancel('p1', 'attempt-other');
  assert.equal(manager.isAuthBusy('p1'), true);

  manager.cancel('p1', 'attempt-a');
  gates[0]({ authTypes: ['api_key'], authenticated: false });
  const firstState = await first;
  assert.equal(firstState.attemptId, 'attempt-a');
  assert.equal(firstState.phase, 'cancelled');
  await new Promise((resolve) => setTimeout(resolve, 5));
  assert.equal(loginCalls, 0, 'login must not start after a cancel during validation');

  // A new attempt starts after the old one finished; an old cancel is a no-op.
  const second = manager.begin('p1', 'openai', 'oauth', 'attempt-b');
  manager.cancel('p1', 'attempt-a');
  assert.equal(manager.isAuthBusy('p1'), true, 'stale cancel must not tear down the newer attempt');
  manager.cancel('p1', 'attempt-b');
  assert.equal(manager.isAuthBusy('p1'), false);
  gates[1]({ authTypes: ['oauth'], authenticated: false });
  const secondState = await second;
  assert.equal(secondState.attemptId, 'attempt-b');
  assert.equal(secondState.phase, 'cancelled');
  assert.equal(loginCalls, 0);
  assert.ok(emitted.length > 0);
  assert.ok(emitted.every((state) => typeof state.attemptId === 'string' && state.attemptId.length > 0), 'every auth event carries attemptId');
});

test('abort cancels a prompt before it reaches the paid Pi RPC', async () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), 'pad-workbench-races-'));
  const sessions = path.join(temporary, 'sessions');
  mkdirSync(sessions, { recursive: true });
  const state = {
    version: 1,
    workspaces: [{ id: 'ws-1', name: 'w', path: temporary }],
    profiles: [{ id: 'profile-abc', name: 'P' }],
    tasks: [{
      id: 'task-1',
      workspaceId: 'ws-1',
      profileId: 'profile-abc',
      title: 't',
      status: 'idle',
      provider: 'openai',
      modelId: 'gpt',
      updatedAt: new Date().toISOString(),
    }],
  };
  const store = {
    state,
    mutate(fn) {
      const draft = structuredClone(state);
      const result = fn(draft);
      state.tasks = draft.tasks;
      state.workspaces = draft.workspaces;
      state.profiles = draft.profiles;
      return result;
    },
    releaseLock() {},
  };
  const host = new WorkbenchHost({ root: temporary, store, emit() {}, diagnostic() {} });
  host.profileDirs = () => ({ agent: path.join(temporary, 'agent'), sessions, env: {} });
  let resolveAvailable;
  const availability = new Promise((resolve) => { resolveAvailable = resolve; });
  let ensureCalls = 0;
  host.sdk = { modelAvailable: () => availability, sessionMessages: async () => [] };
  host.runtime = {
    get: () => null,
    ensure: async () => { ensureCalls += 1; return { request: async () => ({}) }; },
  };
  try {
    const pending = host.prompt('task-1', 'hello');
    await Promise.resolve();
    assert.equal(host.taskOps.has('task-1'), true, 'prompt must reserve before its first await');
    const aborted = await host.abort('task-1');
    assert.deepEqual(aborted, {});
    resolveAvailable(true);
    await assert.rejects(pending, /cancelled/i);
    assert.equal(ensureCalls, 0, 'abort must prevent the send before the paid RPC');
    assert.equal(host.taskOps.has('task-1'), false);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});
