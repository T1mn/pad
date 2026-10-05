// Bounded offline fixtures. Integration owner may run: node --test host/thinking-levels.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { THINKING_LEVELS, clampToThinkingLevels } from './thinking-levels.mjs';
import { PiSdk } from './pi-sdk.mjs';
import { validateState } from './workbench-store.mjs';
import { WorkbenchHost } from './workbench.mjs';

const seed = () => ({ version: 1, workspaces: [{ id: 'w', name: 'w', path: '/fake' }],
  profiles: [{ id: 'p', name: 'p' }], tasks: [{ id: 't', workspaceId: 'w', profileId: 'p',
    title: 't', status: 'idle', updatedAt: 'fixture', provider: 'openai', modelId: 'a' }] });
function fixture() {
  const store = { root: '/fake', state: seed(), mutate(fn) { const draft = structuredClone(this.state); const value = fn(draft);
    this.state = validateState(draft); return value; } };
  const host = new WorkbenchHost({ root: '/fake', store, emit() {}, diagnostic() {} });
  host.scheduleSnapshot = () => {};
  host.profileDirs = () => ({ agent: '/fake/agent', sessions: '/fake/sessions', cli: '/fake/pi', env: {} });
  host.taskSessionPath = () => null;
  host.sdk = { modelAvailable: async () => true, modelThinkingLevels: async () => [...THINKING_LEVELS] };
  let live = null;
  const requests = [];
  const process = { state: { model: { provider: 'openai', id: 'a' }, thinkingLevel: 'high' },
    async request(command) { requests.push(command); return command.type === 'get_state' ? structuredClone(this.state) : {}; } };
  host.runtime = { get: () => live, ensure: async () => { live = process; return process; }, stop: async () => { live = null; } };
  return { host, store, process, requests, activate: () => { live = process; } };
}

test('SDK configured metadata supplies capabilities without changing upstream identities', async () => {
  const models = [{ id: 'a', reasoning: true, thinkingLevelMap: { xhigh: 'xhigh', max: 'max' } },
    { id: 'plain', reasoning: false }, { id: 'only', reasoning: true,
      thinkingLevelMap: { off: null, minimal: null, low: null, medium: null } }];
  const sdk = new PiSdk({ packageRoot: '/fake', openaiCatalog: { get: async () => ({
    models: [...models.map((m) => ({ provider: 'openai', id: m.id, source: 'openai_account', selectable: true })),
      { provider: 'openai', id: 'unknown', source: 'openai_account', selectable: false }] }) } });
  sdk.runtimeFor = async () => ({ getProviders: () => [{ id: 'openai' }], getModels: () => models,
    listCredentials: async () => [{ providerId: 'openai', type: 'oauth' }] });
  sdk.configuredProviders = async () => new Set(['openai']);
  // Fake public utility's Pi 1.0.2 contract; asserts actual configured model input.
  sdk.getSupportedThinkingLevels = (model) => {
    assert.ok(models.includes(model));
    return !model.reasoning ? ['off'] : THINKING_LEVELS.filter((level) => model.thinkingLevelMap?.[level] !== null
      && (!['xhigh', 'max'].includes(level) || model.thinkingLevelMap?.[level] !== undefined));
  };
  const catalog = await sdk.catalog('/fake');
  assert.deepEqual(catalog.models.map((m) => m.thinkingLevels), [[...THINKING_LEVELS], ['off'], ['high'], []]);
  assert.equal(catalog.models[0].source, 'openai_account');
  assert.equal(clampToThinkingLevels('low', ['high']), 'high');
  assert.equal(clampToThinkingLevels('max', ['off', 'high']), 'high');
});

test('missing task level stays missing; explicit selection roundtrips and starts via argv without spawning', async () => {
  const { host, store } = fixture();
  assert.equal(Object.hasOwn(validateState(seed()).tasks[0], 'thinkingLevel'), false);
  assert.equal(host.createLaunch(store.state.tasks[0]).args.includes('--thinking'), false);
  const updated = await host.execute('set_thinking_level', { taskId: 't', level: 'max' });
  assert.equal(updated.thinkingLevel, 'max');
  assert.equal(host.runtime.get('t'), null);
  const args = host.createLaunch(validateState(JSON.parse(JSON.stringify(store.state))).tasks[0]).args;
  assert.deepEqual(args.slice(-2), ['--thinking', 'max']);
  assert.throws(() => validateState({ ...seed(), tasks: [{ ...seed().tasks[0], thinkingLevel: 'none' }] }));
});

test('live empty setter/no event reconciles actual getter; config events never leak to old model', async () => {
  const { host, store, process, requests, activate } = fixture(); activate();
  const updated = await host.setThinkingLevel('t', 'max');
  assert.equal(updated.thinkingLevel, 'high'); // Fake Pi clamped, no event.
  assert.deepEqual(requests.map((r) => r.type), ['set_thinking_level', 'get_state']);
  store.state.tasks[0].thinkingLevel = 'low';
  host.sdk.modelThinkingLevels = async () => ['high'];
  process.request = async (command) => {
    requests.push(command);
    if (command.type === 'set_model') {
      host.onPiEvent('t', { type: 'thinking_level_changed', level: 'off' }, process);
      assert.equal(store.state.tasks[0].modelId, 'a');
      assert.equal(store.state.tasks[0].thinkingLevel, 'low');
      process.state.model.id = 'b';
    }
    if (command.type === 'get_state') return structuredClone(process.state);
    return {};
  };
  assert.equal((await host.setModel('t', 'openai', 'b')).thinkingLevel, 'high');
  assert.ok(requests.some((r) => r.type === 'set_thinking_level' && r.level === 'high'));
  host.onPiEvent('t', { type: 'thinking_level_changed', level: 'off' }, {});
  assert.equal(store.state.tasks[0].thinkingLevel, 'high');
});

test('bootstrap inherits restored level and config reservation blocks sends, auth and active runs', async () => {
  const { host, store, process, requests } = fixture();
  await host.prompt('t', 'fake prompt'); // Only a fake process, no SDK session/inference.
  assert.equal(store.state.tasks[0].thinkingLevel, 'high');
  assert.deepEqual(requests.map((r) => r.type), ['get_state', 'prompt']);
  await assert.rejects(host.setThinkingLevel('t', 'low'), /running/);
  host.onPiEvent('t', { type: 'agent_settled' }, process);
  let release;
  host.sdk.modelThinkingLevels = () => new Promise((resolve) => { release = resolve; });
  const pending = host.setThinkingLevel('t', 'low');
  await assert.rejects(host.prompt('t', 'blocked'), /active prompt/);
  await assert.rejects(host.setModel('t', 'openai', 'b'), /running/);
  assert.equal(host.profileBusy('p'), true);
  release(['low']); await pending;
  host.auth.isAuthBusy = () => true;
  await assert.rejects(host.setThinkingLevel('t', 'low'), /authentication/);
});

test('failed config teardown blocks later configuration and prompt even when runtime hides the child', async () => {
  const { host, process, requests, activate } = fixture(); activate();
  process.request = async (command) => { requests.push(command); throw new Error('fake timeout'); };
  host.runtime.stop = async () => {
    process.closed = true;
    host.runtime.get = () => null; // PiTaskProcess.stop closes RPC before termination.
    throw new Error('private teardown details');
  };
  const blocked = /Pi teardown could not be confirmed; restart PAD/;
  await assert.rejects(host.setThinkingLevel('t', 'high'), blocked);
  assert.equal(host.taskOps.has('t'), false);
  await assert.rejects(host.setThinkingLevel('t', 'low'), blocked);
  await assert.rejects(host.setModel('t', 'openai', 'b'), blocked);
  await assert.rejects(host.prompt('t', 'never sent'), blocked);
  assert.deepEqual(requests.map((r) => r.type), ['set_thinking_level']);
});

test('deferred idle abort reserves the task until completion without releasing another abort', async () => {
  const { host, process, requests, activate } = fixture(); activate();
  let release;
  process.request = async (command) => {
    requests.push(command);
    if (command.type === 'abort') return new Promise((resolve) => { release = resolve; });
    return command.type === 'get_state' ? structuredClone(process.state) : {};
  };
  const pending = host.abort('t');
  await assert.rejects(host.prompt('t', 'never sent'), /abort is in progress/);
  await assert.rejects(host.setThinkingLevel('t', 'low'), /abort is in progress/);
  await assert.rejects(host.abort('t'), /abort is in progress/);
  assert.equal(host.taskAborts.has('t'), true);
  assert.deepEqual(requests.map((r) => r.type), ['clear_queue', 'abort']);
  release({}); await pending;
  assert.equal(host.taskAborts.has('t'), false);
  await host.setThinkingLevel('t', 'high'); // New operations are allowed only now.
  assert.equal(requests.some((r) => r.type === 'prompt'), false);
});

test('uncertain config stops runtime; cancelled startup never sends a future prompt', async () => {
  const { host, requests, process, activate } = fixture(); activate();
  process.request = async (command) => { requests.push(command); throw new Error('fake timeout'); };
  await assert.rejects(host.setThinkingLevel('t', 'high'), /fake timeout/);
  assert.equal(host.runtime.get('t'), null);
  assert.equal(process.configUnknown, true);
  const next = fixture();
  let release;
  next.host.sdk.modelAvailable = () => new Promise((resolve) => { release = resolve; });
  const pending = next.host.prompt('t', 'never sent');
  await next.host.abort('t');
  release(true);
  await assert.rejects(pending, /cancelled/);
  assert.deepEqual(next.requests, []);
});
