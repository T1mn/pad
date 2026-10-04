import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { initializeHttpClient, loadInstalledUndici, proxyOptions, reportProxyWarning } from './http-client.mjs';

function fakeUndici({ hangClose = false, failInstall = false } = {}) {
  const calls = [];
  class Client extends EventEmitter { constructor(origin, options) { super(); this.options = options; } }
  class Pool extends Client {}
  class EnvHttpProxyAgent extends EventEmitter {
    constructor(options) { super(); this.options = options; calls.push('create'); }
    dispatch() { throw new Error('offline fake must not dispatch'); }
    close() { calls.push('close'); return hangClose ? new Promise(() => {}) : Promise.resolve(); }
    destroy() { calls.push('destroy'); return Promise.resolve(); }
  }
  let dispatcher;
  return { calls, get dispatcher() { return dispatcher; }, api: {
    Client, Pool, EnvHttpProxyAgent,
    setGlobalDispatcher(value) { dispatcher = value; calls.push('set'); },
    install() { calls.push('install'); if (failInstall) throw new Error('https://user:secret@proxy'); },
  } };
}

test('bootstrap before SDK-capable imports, Pi options and one owned close', async () => {
  const source = readFileSync(new URL('./workbench-host.mjs', import.meta.url), 'utf8');
  assert.ok(source.indexOf('sanitizeAmbientCredentials(process.env)') < source.indexOf('await initializeHttpClient('));
  assert.ok(source.indexOf('await initializeHttpClient(') < source.indexOf("await import('./workbench.mjs')"));
  assert.ok(source.indexOf('await host.close()') < source.indexOf('await httpClient.close()'));
  const fake = fakeUndici();
  const env = { HTTP_PROXY: 'http://proxy:8080', NO_PROXY: 'localhost' };
  const client = await initializeHttpClient({ env, loadUndici: () => fake.api });
  assert.deepEqual(fake.calls, ['create', 'set', 'install']);
  const opts = fake.dispatcher.options;
  assert.equal(opts.allowH2, false);
  assert.equal(opts.proxyTunnel, true);
  assert.equal(opts.bodyTimeout, 300_000);
  assert.equal(opts.headersTimeout, 300_000);
  assert.deepEqual(opts.connect, { autoSelectFamilyAttemptTimeout: 2_000 });
  assert.equal(env.http_proxy, 'http://proxy:8080');
  assert.equal(env.https_proxy, env.http_proxy);
  await Promise.all([client.close(), client.close()]);
  assert.equal(fake.calls.filter((call) => call === 'close').length, 1);
});

test('explicit lowercase/empty settings win; ALL only with no protocol settings', () => {
  assert.deepEqual(proxyOptions({ http_proxy: '', HTTP_PROXY: 'http://ignored',
    HTTPS_PROXY: 'https://proxy', all_proxy: 'socks5h://ignored', no_proxy: '', NO_PROXY: '*' }),
  { httpProxy: '', httpsProxy: 'https://proxy', noProxy: '' });
  assert.deepEqual(proxyOptions({ all_proxy: 'socks5://proxy:1080' }),
    { httpProxy: 'socks5://proxy:1080', httpsProxy: 'socks5://proxy:1080', noProxy: '' });
  assert.throws(() => proxyOptions({ ALL_PROXY: 'socks5h://user:secret@proxy' }),
    { message: 'Invalid proxy configuration; use an http, https or socks5 proxy URL' });
});

test('safe errors, internal error reporting, bounded destruction and failed install cleanup', async () => {
  const fake = fakeUndici({ hangClose: true });
  const diagnostics = [];
  const client = await initializeHttpClient({ env: {}, loadUndici: () => fake.api,
    diagnostic: (value) => diagnostics.push(value), closeTimeoutMs: 5 });
  fake.dispatcher.options.clientFactory('https://example.invalid', {}).emit('error', new Error('secret'));
  await client.close();
  assert.deepEqual(fake.calls, ['create', 'set', 'install', 'close', 'destroy']);
  assert.ok(diagnostics.some((value) => value.includes('connection error')));
  assert.ok(diagnostics.every((value) => !value.includes('secret')));
  const failed = fakeUndici({ failInstall: true });
  await assert.rejects(initializeHttpClient({ env: {}, loadUndici: () => failed.api }),
    { message: 'HTTP bootstrap failed; check the installed Pi Undici API and proxy configuration' });
  assert.equal(failed.calls.at(-1), 'close');
  await assert.rejects(initializeHttpClient({ env: {}, loadUndici: () => ({}) }),
    { message: 'HTTP bootstrap failed; check the installed Pi Undici API and proxy configuration' });
  const env = { PAD_PROXY_WARNING: 'unsupported-system' };
  const warnings = [];
  reportProxyWarning(env, (frame) => warnings.push(frame));
  assert.equal(env.PAD_PROXY_WARNING, undefined);
  assert.deepEqual(warnings, [{ type: 'event', event: 'host_warning',
    data: { code: 'system_proxy_not_adopted' } }]);
  reportProxyWarning({ PAD_PROXY_WARNING: 'https://user:secret@proxy' }, (frame) => warnings.push(frame));
  assert.equal(warnings.length, 1);
});

// Real public Dispatcher.request/dispatch contract; both underlying agents are
// Undici MockAgents with networking disabled. No global installation or sockets.
test('real Dispatcher routes explicit empty HTTPS directly with HTTP configured', async () => {
  const undici = loadInstalledUndici({});
  const agents = [];
  const lifecycle = [];
  function mockAgent(kind) {
    return class extends undici.MockAgent {
      constructor(options) {
        super();
        this.options = options;
        this.disableNetConnect();
        this.get(kind === 'proxy' ? 'http://example.invalid' : 'https://example.invalid')
          .intercept({ path: '/', method: 'GET' }).reply(200, kind);
        agents.push(this);
      }
      close() { lifecycle.push(`${kind}:close`); return super.close(); }
      destroy(error) { lifecycle.push(`${kind}:destroy`); return super.destroy(error); }
    };
  }
  let dispatcher;
  const client = await initializeHttpClient({
    env: { HTTP_PROXY: 'http://proxy.invalid:8080', HTTPS_PROXY: '' },
    loadUndici: () => ({ ...undici, EnvHttpProxyAgent: mockAgent('proxy'),
      Agent: mockAgent('direct'), setGlobalDispatcher(value) { dispatcher = value; }, install() {} }),
  });
  assert.ok(dispatcher instanceof undici.Dispatcher);
  assert.equal(agents[0].options.httpProxy, 'http://proxy.invalid:8080');
  assert.equal(agents[0].options.httpsProxy, '');
  try {
    for (const [origin, expected] of [['http://example.invalid', 'proxy'],
      ['https://example.invalid', 'direct']]) {
      const response = await dispatcher.request({ origin, path: '/', method: 'GET' });
      assert.equal(await response.body.text(), expected);
    }
    agents.forEach((agent) => agent.assertNoPendingInterceptors());
  } finally {
    await Promise.all([client.close(), client.close()]);
  }
  assert.deepEqual(lifecycle, ['proxy:close', 'direct:close']);
});
