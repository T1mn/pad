// Public Undici bootstrap only. No Pi core imports, settings reads or requests.
import { EventEmitter } from 'node:events';
import { createRequire } from 'node:module';
import path from 'node:path';
import { resolvePiPackage } from './host-env.mjs';

const PROXY_KEYS = ['http_proxy', 'HTTP_PROXY', 'https_proxy', 'HTTPS_PROXY', 'all_proxy', 'ALL_PROXY'];

/** Presence (including empty values) is explicit; lowercase wins. */
export function proxyOptions(env) {
  const value = (lower, upper) => env[lower] ?? env[upper];
  let httpProxy = value('http_proxy', 'HTTP_PROXY');
  let httpsProxy = value('https_proxy', 'HTTPS_PROXY');
  if (httpProxy === undefined && httpsProxy === undefined) {
    httpProxy = httpsProxy = value('all_proxy', 'ALL_PROXY');
  }
  for (const proxy of [httpProxy, httpsProxy]) {
    if (proxy === undefined || proxy === '') continue;
    try {
      const url = new URL(proxy);
      if (!['http:', 'https:', 'socks5:'].includes(url.protocol) || !url.hostname
          || url.hash || url.search || (url.pathname && url.pathname !== '/')) throw new Error();
    } catch {
      throw new Error('Invalid proxy configuration; use an http, https or socks5 proxy URL');
    }
  }
  // Resolve absent HTTPS to HTTP, but preserve explicit empty for scheme routing.
  return { httpProxy: httpProxy ?? '', httpsProxy: httpsProxy ?? httpProxy ?? '',
    noProxy: value('no_proxy', 'NO_PROXY') ?? '' };
}

export function loadInstalledUndici(env) {
  try {
    const { packageRoot } = resolvePiPackage(env);
    return createRequire(path.join(packageRoot, 'package.json'))('undici');
  } catch {
    throw new Error('HTTP bootstrap unavailable; check the installed Pi package and its Undici dependency');
  }
}

/** One owned dispatcher per host. Call close after cancelling auth and tasks. */
export async function initializeHttpClient({ env = process.env, loadUndici = loadInstalledUndici,
  diagnostic = () => {}, closeTimeoutMs = 500 } = {}) {
  const options = proxyOptions(env);
  let dispatcher;
  const owned = [];
  let closing;
  let agentsClosing;
  let agentsDestroying;
  // Schedule every owned agent even if another throws synchronously.
  const closeAgents = () => agentsClosing ??= Promise.all(owned.map((agent) =>
    Promise.resolve().then(() => agent.close())));
  const destroyAgents = (error) => agentsDestroying ??= Promise.all(owned.map((agent) =>
    Promise.resolve().then(() => agent.destroy(error))));
  const bounded = async (operation) => {
    let timer;
    try {
      return await Promise.race([
        Promise.resolve().then(operation).then(() => true, () => false),
        new Promise((resolve) => { timer = setTimeout(() => resolve(false), closeTimeoutMs); }),
      ]);
    } finally { clearTimeout(timer); }
  };
  const close = () => closing ??= (async () => {
    if (!owned.length) return;
    if (!await bounded(closeAgents)) {
      diagnostic('HTTP dispatcher close incomplete; destroying owned connections');
      if (!await bounded(destroyAgents)) diagnostic('HTTP dispatcher destroy incomplete');
    }
  })();
  try {
    const undici = loadUndici(env);
    for (const name of ['EnvHttpProxyAgent', 'Client', 'Pool', 'setGlobalDispatcher', 'install']) {
      if (typeof undici?.[name] !== 'function') throw new Error();
    }
    // Internal Client error events may accompany a rejecting response body.
    // Report only a fixed message; request/body rejections are NOT intercepted.
    const listen = (client) => {
      if (client instanceof EventEmitter) EventEmitter.prototype.on.call(client, 'error', () => {
        diagnostic('HTTP dispatcher connection error; request or response body may fail');
      });
      return client;
    };
    const clientFactory = (origin, opts) => listen(new undici.Client(origin, opts));
    const factory = (origin, opts) => opts.connections === 1 ? clientFactory(origin, opts)
      : listen(new undici.Pool(origin, { ...opts, factory: clientFactory }));
    const agentOptions = { allowH2: false, proxyTunnel: true,
      bodyTimeout: 300_000, headersTimeout: 300_000,
      connect: { autoSelectFamilyAttemptTimeout: 2_000 }, clientFactory, factory };
    const own = (agent) => {
      owned.push(agent);
      if (typeof agent.dispatch !== 'function' || typeof agent.close !== 'function'
          || typeof agent.destroy !== 'function') throw new Error();
      return listen(agent);
    };
    const needsDirectHttps = options.httpProxy !== '' && options.httpsProxy === '';
    if (needsDirectHttps && (typeof undici.Dispatcher !== 'function' || typeof undici.Agent !== 'function')) {
      throw new Error();
    }
    const proxied = own(new undici.EnvHttpProxyAgent({ ...options, ...agentOptions }));
    dispatcher = proxied;
    if (needsDirectHttps) {
      const direct = own(new undici.Agent(agentOptions));
      // Undici 8 falls back from httpsProxy:'' to HTTP. Route that scheme
      // outside EnvHttpProxyAgent, retaining its NO_PROXY logic for HTTP.
      class SchemeDispatcher extends undici.Dispatcher {
        dispatch(opts, handler) {
          const target = new URL(opts.origin).protocol === 'https:' ? direct : proxied;
          return target.dispatch(opts, handler);
        }
        close(callback) {
          const result = closeAgents();
          if (typeof callback === 'function') { result.then(() => callback(null), callback); return; }
          return result;
        }
        destroy(error, callback) {
          if (typeof error === 'function') { callback = error; error = undefined; }
          const result = destroyAgents(error);
          if (typeof callback === 'function') { result.then(() => callback(null), callback); return; }
          return result;
        }
      }
      dispatcher = listen(new SchemeDispatcher());
    }
    undici.setGlobalDispatcher(dispatcher);
    undici.install();
    // Normalize for subsequent Pi task children; do not forward an unused ALL_PROXY.
    for (const key of PROXY_KEYS) delete env[key];
    env.http_proxy = options.httpProxy;
    env.https_proxy = options.httpsProxy;
    env.no_proxy = options.noProxy;
    return { close };
  } catch {
    await close();
    throw new Error('HTTP bootstrap failed; check the installed Pi Undici API and proxy configuration');
  }
}

export function reportProxyWarning(env, emit) {
  if (env.PAD_PROXY_WARNING === 'unsupported-system') {
    emit({ type: 'event', event: 'host_warning', data: { code: 'system_proxy_not_adopted' } });
  }
  delete env.PAD_PROXY_WARNING;
}
