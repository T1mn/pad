import test from 'node:test';
import assert from 'node:assert/strict';
import { PiSdk, authTypesOf } from './pi-sdk.mjs';
import { AuthError, safeAuthMessage } from './auth-policy.mjs';

const handler = { login() {} };
function fixture({ getDeviceId, login = async () => {}, credentials = [] } = {}) {
  const providers = [
    { id: 'openai', auth: { apiKey: handler, oauth: handler } },
    { id: 'anthropic', auth: { apiKey: handler, oauth: handler } },
    { id: 'google', auth: { apiKey: handler } },
    { id: 'google-vertex', auth: { apiKey: handler } },
    { id: 'google-gemini-cli', auth: { oauth: handler } },
    { id: 'google-antigravity', auth: { oauth: handler } },
    { id: 'metadata-only', auth: { apiKey: {}, oauth: { login: true } } },
  ];
  const runtime = {
    getProviders: () => providers,
    getProvider: (id) => providers.find((p) => p.id === id),
    listCredentials: async () => credentials,
    getProviderAuthStatus: (id) => ({ configured: id === 'google-vertex' || id === 'anthropic' }),
    getModels: () => [{ id: 'one' }], login,
  };
  const sdk = new PiSdk({ packageRoot: '/offline-fake', getDeviceId });
  sdk.ModelRuntime = class { static async create() { return runtime; } };
  sdk.loadModule = async () => sdk.ModelRuntime;
  return sdk;
}
test('callable methods and PAD policy retain API key/configured cloud models', async () => {
  assert.deepEqual(authTypesOf({ id: 'custom', auth: { apiKey: {}, oauth: handler } }), ['oauth']);
  const sdk = fixture({ credentials: [
    { providerId: 'anthropic', type: 'oauth' },
    { providerId: 'google', type: 'api_key' },
    { providerId: 'google-gemini-cli', type: 'oauth' },
  ] });
  const catalog = await sdk.catalog('/offline-profile');
  assert.deepEqual(catalog.providers.find((p) => p.id === 'anthropic').authTypes, ['api_key']);
  assert.deepEqual(catalog.providers.find((p) => p.id === 'metadata-only').authTypes, []);
  assert.deepEqual(catalog.models.map((m) => m.provider), ['google', 'google-vertex']);
  assert.equal((await sdk.providerStatus('/offline-profile', 'anthropic')).authenticated, false);
  assert.equal(await sdk.modelAvailable('/offline-profile', 'anthropic', 'one'), false);
  await assert.rejects(sdk.login({ agentDir: '/offline-profile', providerId: 'anthropic', method: 'oauth', interaction: {} }), AuthError);
  const apiSdk = fixture({ credentials: [{ providerId: 'anthropic', type: 'api_key' }] });
  assert.equal(await apiSdk.modelAvailable('/offline-profile', 'anthropic', 'one'), true);
});
test('runtime receives fourth options with a lazy synchronous callback', async () => {
  let requests = 0;
  const id = '12345678-1234-4234-8234-123456789abc';
  const sdk = fixture({ getDeviceId: () => { requests++; return id; }, login: async (provider, method, interaction, options) => {
    assert.equal(provider, 'openai');
    assert.equal(typeof options.getDeviceId, 'function');
    if (method === 'oauth') assert.equal(options.getDeviceId(), id);
  } });
  await sdk.catalog('/offline-profile');
  await sdk.login({ agentDir: '/offline-profile', providerId: 'openai', method: 'api_key', interaction: {} });
  assert.equal(requests, 0);
  await sdk.login({ agentDir: '/offline-profile', providerId: 'openai', method: 'oauth', interaction: {} });
  assert.equal(requests, 1);
  const missing = fixture({ login: async (_p, _m, _i, options) => options.getDeviceId() });
  await assert.rejects(missing.login({ agentDir: '/offline-profile', providerId: 'openai', method: 'oauth', interaction: {} }), (e) => e.code === 'identity_missing');
});
test('loaded synchronization class becomes an app-owned committed failure', async () => {
  class SyncError extends Error { constructor() { super('SENTINEL-SECRET'); this.credential = 'SENTINEL-SECRET'; } }
  const sdk = fixture({ login: async () => { throw new SyncError(); } });
  sdk.CredentialSynchronizationError = SyncError;
  await assert.rejects(sdk.login({ agentDir: '/offline-profile', providerId: 'openai', method: 'oauth', interaction: {} }), (e) => {
    assert.equal(e.code, 'credential_sync');
    assert.match(safeAuthMessage(e), /Credentials were saved/);
    assert.equal(JSON.stringify(e).includes('SENTINEL'), false);
    return true;
  });
});
