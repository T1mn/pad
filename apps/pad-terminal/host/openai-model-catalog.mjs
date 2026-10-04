// Official SIWC catalog only; never an inference or general SDK refresh.
// Contract: https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const ENDPOINT = 'https://api.openai.com/v1/models';
const MAX_BYTES = 1024 * 1024;
const SAFE_ERROR = 'OpenAI account models could not be refreshed.';
const CREDENTIALS_CHANGED = '凭据已更新，请再次同步模型';
const STALE = 'Cached OpenAI account models are stale; refresh to verify availability.';
const CACHE = 'pad-openai-models.json';

export function parseAccountModels(value) {
  if (!value || !Array.isArray(value.models)) throw new Error(SAFE_ERROR);
  const models = [], seen = new Set();
  let visible = 0;
  for (const entry of value.models) {
    if (entry?.visibility !== 'list') continue;
    if (++visible > 256) throw new Error(SAFE_ERROR);
    const id = typeof entry.slug === 'string' ? entry.slug.trim() : '';
    const name = typeof entry.display_name === 'string' ? entry.display_name.trim() : '';
    if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,199}$/.test(id)
      || !name || name.length > 200 || /[\u0000-\u001f\u007f]/.test(name)) throw new Error(SAFE_ERROR);
    if (seen.has(id)) continue;
    seen.add(id);
    models.push({ id, name });
  }
  return models;
}

// No credential contents: conservative private file-registration binding. A
// change, including during getAuth refresh, invalidates the catalog. The SDK
// exposes only providerId/type, not a stable OAuth registration identifier.
function binding(agentDir) {
  const dir = fs.lstatSync(agentDir);
  if (!dir.isDirectory() || dir.isSymbolicLink()) throw new Error(SAFE_ERROR);
  const stat = fs.lstatSync(path.join(agentDir, 'auth.json'), { bigint: true });
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(SAFE_ERROR);
  return crypto.createHash('sha256').update(`${stat.dev}:${stat.ino}:${stat.size}:${stat.mtimeNs}:${stat.ctimeNs}`).digest('hex');
}

function readCache(agentDir, identity) {
  let fd;
  try {
    fd = fs.openSync(path.join(agentDir, CACHE), fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
    const stat = fs.fstatSync(fd);
    if (!stat.isFile() || stat.size > MAX_BYTES || (stat.mode & 0o077)) return null;
    const data = JSON.parse(fs.readFileSync(fd, 'utf8'));
    if (data.version !== 1 || data.binding !== identity || typeof data.updatedAt !== 'string'
      || !Number.isFinite(Date.parse(data.updatedAt))) return null;
    const models = parseAccountModels({ models: data.models.map((m) => ({ slug: m.id, display_name: m.name, visibility: 'list' })) });
    return { models, updatedAt: data.updatedAt, state: 'stale' };
  } catch { return null; } finally { if (fd !== undefined) fs.closeSync(fd); }
}

function writeCache(agentDir, identity, entry) {
  const file = path.join(agentDir, CACHE);
  const temp = `${file}.${crypto.randomUUID()}.tmp`;
  let fd;
  try {
    try {
      const stat = fs.lstatSync(file);
      if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(SAFE_ERROR);
    } catch (error) { if (error.code !== 'ENOENT') throw error; }
    fd = fs.openSync(temp, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW, 0o600);
    fs.writeFileSync(fd, JSON.stringify({ version: 1, binding: identity, models: entry.models, updatedAt: entry.updatedAt }));
    fs.fsyncSync(fd);
    fs.closeSync(fd); fd = undefined;
    fs.renameSync(temp, file);
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
    try { fs.unlinkSync(temp); } catch { /* unpublished temp */ }
  }
}

async function responseJson(response) {
  if (!response.ok || !response.body || Number(response.headers.get('content-length')) > MAX_BYTES) throw new Error(SAFE_ERROR);
  const reader = response.body.getReader();
  const chunks = []; let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > MAX_BYTES) throw new Error(SAFE_ERROR);
      chunks.push(Buffer.from(value));
    }
    return JSON.parse(Buffer.concat(chunks).toString('utf8'));
  } finally { try { await reader.cancel(); } catch { /* no raw errors */ } }
}

export class OpenAIModelCatalog {
  constructor({ fetch = globalThis.fetch, registrationBinding = binding } = {}) {
    this.fetch = fetch;
    this.binding = registrationBinding;
    this.profiles = new Map();
  }

  invalidate(agentDir) {
    const previous = this.profiles.get(agentDir);
    previous?.controller?.abort();
    // Install the tombstone before touching disk: invalidation failure cannot
    // resurrect a previous-account entry during this host's lifetime.
    const tombstone = { blocked: true };
    this.profiles.set(agentDir, tombstone);
    try { fs.unlinkSync(path.join(agentDir, CACHE)); } catch (error) {
      if (error.code !== 'ENOENT') throw new Error('OpenAI account model cleanup failed. Restart PAD before using this profile.');
    }
    this.profiles.delete(agentDir);
  }

  close() { for (const state of this.profiles.values()) state.controller?.abort(); }

  async get(agentDir, runtime, { refresh = false } = {}) {
    let state;
    try {
      const credentials = await runtime.listCredentials();
      if (!credentials.some((c) => c.providerId === 'openai' && c.type === 'oauth')) {
        this.invalidate(agentDir);
        return { models: [], openaiDiscovery: { state: 'not_loaded' } };
      }
      const identity = this.binding(agentDir);
      state = this.profiles.get(agentDir);
      if (state?.blocked) throw new Error(SAFE_ERROR);
      if (state && state.identity !== identity) {
        this.invalidate(agentDir);
        state = null;
      }
      if (!state) {
        state = { identity, entry: readCache(agentDir, identity) };
        this.profiles.set(agentDir, state);
      }
      if (refresh) {
        if (!state.pending) {
          state.pending = this.refresh(agentDir, runtime, state).finally(() => { state.pending = null; });
        }
        await state.pending;
      }
      if (this.profiles.get(agentDir) !== state || this.binding(agentDir) !== state.identity) throw new Error(SAFE_ERROR);
      const supported = new Set(runtime.getModels('openai').map((m) => m.id));
      const entry = state.entry;
      return {
        models: (entry?.models ?? []).map((m) => ({ provider: 'openai', ...m, source: 'openai_account', selectable: supported.has(m.id) })),
        openaiDiscovery: entry
          ? { state: entry.state, updatedAt: entry.updatedAt, ...(entry.state === 'stale' ? { message: state.failed ? SAFE_ERROR + ' ' + STALE : STALE } : {}) }
          : { state: state.failed ? 'error' : 'not_loaded', ...(state.failed ? { message: SAFE_ERROR } : {}) },
      };
    } catch (error) {
      return { models: [], openaiDiscovery: { state: 'error', message: error.message === CREDENTIALS_CHANGED ? CREDENTIALS_CHANGED : SAFE_ERROR } };
    }
  }

  async refresh(agentDir, runtime, state) {
    const controller = new AbortController();
    state.controller = controller;
    let timer;
    const check = () => {
      if (controller.signal.aborted || this.profiles.get(agentDir) !== state) throw new Error(SAFE_ERROR);
    };
    const operation = async () => {
      check();
      const resolved = await runtime.getAuth('openai', { signal: controller.signal });
      check();
      if (resolved?.source !== 'OAuth' || typeof resolved.auth?.apiKey !== 'string'
        || !resolved.auth.apiKey || resolved.auth.apiKey.length > 16 * 1024 || /\s/.test(resolved.auth.apiKey)) throw new Error(SAFE_ERROR);
      // getAuth may persist rotation. With no public registration ID we cannot
      // prove that a changed file is the same account: discard the old cache
      // rather than rebind old choices to potentially different credentials.
      const checkBinding = () => {
        if (this.binding(agentDir) !== state.identity) {
          state.entry = null;
          this.invalidate(agentDir);
          throw new Error(CREDENTIALS_CHANGED);
        }
      };
      checkBinding();
      if (!(await runtime.listCredentials()).some((c) => c.providerId === 'openai' && c.type === 'oauth')) throw new Error(SAFE_ERROR);
      check();
      checkBinding();
      const response = await this.fetch(ENDPOINT, {
        method: 'GET', redirect: 'error', signal: controller.signal,
        headers: { Authorization: `Bearer ${resolved.auth.apiKey}`, Accept: 'application/json' },
      });
      const models = parseAccountModels(await responseJson(response));
      check();
      if (this.binding(agentDir) !== state.identity) throw new Error(SAFE_ERROR);
      const entry = { models, updatedAt: new Date().toISOString(), state: 'fresh' };
      writeCache(agentDir, state.identity, entry);
      state.entry = entry;
      state.failed = false;
    };
    try {
      // Also bound a misbehaving injected fetch/SDK that ignores cancellation.
      await Promise.race([operation(), new Promise((_, reject) => {
        timer = setTimeout(() => { controller.abort(); reject(new Error(SAFE_ERROR)); }, 15_000);
      })]);
    } catch (error) {
      state.failed = true;
      if (state.entry) state.entry.state = 'stale';
      if (error.message === CREDENTIALS_CHANGED) throw error;
    } finally { clearTimeout(timer); state.controller = null; }
  }
}
