// Small offline suite: fake SDK, fetch, and private temp directories only.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { OpenAIModelCatalog, parseAccountModels } from './openai-model-catalog.mjs';
import { PiSdk } from './pi-sdk.mjs';

function fixture(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'pad-account-models-'));
  fs.writeFileSync(path.join(dir, 'auth.json'), '{}', { mode: 0o600 });
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  let calls = 0;
  const provider = { id: 'openai', name: 'OpenAI', auth: { oauth: { login() {} } } };
  const runtime = {
    listCredentials: async () => [{ providerId: 'openai', type: 'oauth' }],
    getAuth: async (id, { signal }) => {
      assert.equal(id, 'openai'); assert.ok(signal instanceof AbortSignal);
      calls++; return { source: 'OAuth', auth: { apiKey: 'fixture-secret' } };
    },
    getModels: () => [{ id: 'supported', name: 'Static name' }, { id: 'static-only', name: 'Not entitled' }],
    getProviders: () => [provider], getProvider: () => provider,
    getProviderAuthStatus: () => ({ configured: true }),
  };
  return { dir, runtime, authCalls: () => calls };
}
const response = (models) => new Response(JSON.stringify({ models }));
const listed = (slug, display_name = slug) => ({ slug, display_name, visibility: 'list' });

test('official schema, trimmed order, list visibility and deterministic dedupe', () => {
  assert.deepEqual(parseAccountModels({ models: [listed(' second ', ' Second '),
    { ...listed('hidden'), visibility: 'hide' }, listed('first'), listed('second', 'Duplicate')] }),
  [{ id: 'second', name: 'Second' }, { id: 'first', name: 'first' }]);
  assert.throws(() => parseAccountModels({ data: [{ id: 'supported' }] }));
  assert.throws(() => parseAccountModels({ models: [listed('bad/id')] }));
});

test('SDK OAuth choices are upstream-only; unknown model visible but unselectable; offline does not resolve auth', async (t) => {
  const f = fixture(t); let fetchCalls = 0;
  const discovery = new OpenAIModelCatalog({ fetch: async (url, options) => {
    fetchCalls++;
    assert.equal(url, 'https://api.openai.com/v1/models');
    assert.equal(options.redirect, 'error'); assert.equal(options.method, 'GET');
    assert.equal(options.headers.Authorization, 'Bearer fixture-secret');
    return response([listed('unknown', 'Upstream new'), listed('supported', 'Account name')]);
  } });
  const sdk = new PiSdk({ packageRoot: f.dir, openaiCatalog: discovery });
  sdk.runtimeFor = async () => f.runtime;
  assert.deepEqual((await sdk.catalog(f.dir)).models, []);
  assert.equal(f.authCalls(), 0); assert.equal(fetchCalls, 0);
  const catalog = await sdk.catalog(f.dir, { refreshOpenAI: true });
  assert.equal(catalog.openaiDiscovery.state, 'fresh');
  assert.deepEqual(catalog.models, [
    { provider: 'openai', id: 'unknown', name: 'Upstream new', source: 'openai_account', selectable: false },
    { provider: 'openai', id: 'supported', name: 'Account name', source: 'openai_account', selectable: true },
  ]);
  assert.equal(await sdk.modelAvailable(f.dir, 'openai', 'unknown'), false);
  assert.equal(await sdk.modelAvailable(f.dir, 'openai', 'static-only'), false);
  assert.equal(await sdk.modelAvailable(f.dir, 'openai', 'supported'), true);
  assert.equal(f.authCalls(), 1); assert.equal(fetchCalls, 1);
});

test('failure preserves verified same-registration cache as stale with fixed safe errors', async (t) => {
  const f = fixture(t); let fail = false;
  const discovery = new OpenAIModelCatalog({ fetch: async () => {
    if (fail) throw new Error('fixture-secret https://evil.example raw-auth-error');
    return response([listed('supported')]);
  } });
  await discovery.get(f.dir, f.runtime, { refresh: true });
  fail = true;
  const result = await discovery.get(f.dir, f.runtime, { refresh: true });
  assert.equal(result.openaiDiscovery.state, 'stale');
  assert.equal(result.models[0].id, 'supported');
  assert.doesNotMatch(JSON.stringify(result), /fixture-secret|evil|raw-auth-error/);
  const restored = await new OpenAIModelCatalog({ fetch: () => { throw new Error('offline'); } }).get(f.dir, f.runtime);
  assert.equal(restored.openaiDiscovery.state, 'stale');
  assert.equal(restored.models[0].id, 'supported');
  assert.equal(fs.statSync(path.join(f.dir, 'pad-openai-models.json')).mode & 0o777, 0o600);
});

test('inflight sharing, invalidation cancels late results, and profiles remain isolated', async (t) => {
  const a = fixture(t), b = fixture(t);
  let release, began; const started = new Promise((resolve) => { began = resolve; });
  const discovery = new OpenAIModelCatalog({ fetch: async (_url, options) => {
    began(options.signal);
    return new Promise((resolve) => { release = () => resolve(response([listed('supported')])); });
  } });
  const first = discovery.get(a.dir, a.runtime, { refresh: true });
  const signal = await started;
  const second = discovery.get(a.dir, a.runtime, { refresh: true });
  // Let the second metadata enumeration attach to the same operation.
  await Promise.resolve();
  discovery.invalidate(a.dir);
  assert.equal(signal.aborted, true);
  release();
  const results = await Promise.all([first, second]);
  assert.ok(results.every((r) => r.models.length === 0));
  assert.equal(a.authCalls(), 1);
  assert.equal(fs.existsSync(path.join(a.dir, 'pad-openai-models.json')), false);
  assert.deepEqual((await discovery.get(b.dir, b.runtime)).models, []);
  assert.equal(b.authCalls(), 0);
});

test('credential metadata changed after bearer resolution discards cache without fetching or publishing', async (t) => {
  const f = fixture(t); let fetchCalls = 0;
  const discovery = new OpenAIModelCatalog({ fetch: async () => { fetchCalls++; return response([listed('supported')]); } });
  await discovery.get(f.dir, f.runtime, { refresh: true });
  const resolveAuth = f.runtime.getAuth;
  f.runtime.getAuth = async (...args) => {
    const bearer = await resolveAuth(...args);
    fs.renameSync(path.join(f.dir, 'auth.json'), path.join(f.dir, 'old-fixture.json'));
    fs.writeFileSync(path.join(f.dir, 'auth.json'), '{}', { mode: 0o600 });
    return bearer;
  };
  const result = await discovery.get(f.dir, f.runtime, { refresh: true });
  assert.deepEqual(result, { models: [], openaiDiscovery: { state: 'error', message: '凭据已更新，请再次同步模型' } });
  assert.equal(fetchCalls, 1);
  assert.equal(fs.existsSync(path.join(f.dir, 'pad-openai-models.json')), false);
  assert.deepEqual((await discovery.get(f.dir, f.runtime)).models, []);
});

test('resolved API-key substitution rejected; external credential registration change clears cache', async (t) => {
  const f = fixture(t); let fetchCalls = 0;
  const discovery = new OpenAIModelCatalog({ fetch: async () => { fetchCalls++; return response([listed('supported')]); } });
  await discovery.get(f.dir, f.runtime, { refresh: true });
  fs.renameSync(path.join(f.dir, 'auth.json'), path.join(f.dir, 'old-fixture.json'));
  fs.writeFileSync(path.join(f.dir, 'auth.json'), '{}', { mode: 0o600 });
  assert.deepEqual((await discovery.get(f.dir, f.runtime)).models, []);
  f.runtime.getAuth = async () => ({ source: 'OPENAI_API_KEY', auth: { apiKey: 'fixture-secret' } });
  const result = await discovery.get(f.dir, f.runtime, { refresh: true });
  assert.equal(result.openaiDiscovery.state, 'error');
  assert.deepEqual(result.models, []); assert.equal(fetchCalls, 1);
  assert.doesNotMatch(JSON.stringify(result), /fixture-secret|OPENAI_API_KEY/);
});
