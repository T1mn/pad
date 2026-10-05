// Pi SDK adapter: dynamic, per-profile ModelRuntime for auth and catalog.
//
// The SDK is imported lazily (after the entry point sanitized process.env) and
// only through explicit per-profile auth/models paths. Credential files are
// never read by PAD; explicit OpenAI catalog refresh consumes SDK-resolved auth
// privately. No credentials are returned or serialized; rotation/login/logout
// persistence belongs to the SDK and login results are discarded.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { AuthError, callableAuthTypes, credentialAllowed } from './auth-policy.mjs';
import { OpenAIModelCatalog } from './openai-model-catalog.mjs';

export class PiSdkError extends Error {
  constructor(message) {
    super(message);
    this.name = 'PiSdkError';
  }
}

/** Auth methods a provider advertises, in stable order. */
export function authTypesOf(provider) {
  return callableAuthTypes(provider);
}

function messageEntriesToMessages(entries) {
  const messages = [];
  for (const entry of entries) {
    if (entry?.type === 'message' && entry.message !== undefined) messages.push(entry.message);
  }
  return messages;
}

/**
 * Read-only active-branch parse for one session file, without spawning Pi.
 * Walks parentId from the last appended entry (the live leaf) back to the
 * root, so abandoned branches are never surfaced. This is the fallback used
 * when the installed SDK's SessionManager is unavailable.
 */
export function readSessionMessages(sessionFile) {
  let text;
  try {
    text = fs.readFileSync(sessionFile, 'utf8');
  } catch (error) {
    throw new PiSdkError('Session file could not be read');
  }
  const byId = new Map();
  let leaf;
  for (const line of text.split('\n')) {
    if (line.length === 0) continue;
    let entry;
    try {
      entry = JSON.parse(line);
    } catch {
      continue;
    }
    if (!entry || typeof entry !== 'object' || typeof entry.id !== 'string') continue;
    byId.set(entry.id, entry);
    leaf = entry;
  }
  const branch = [];
  let current = leaf;
  while (current) {
    branch.push(current);
    current = typeof current.parentId === 'string' ? byId.get(current.parentId) : undefined;
  }
  branch.reverse();
  return messageEntriesToMessages(branch);
}

export class PiSdk {
  constructor({ packageRoot, getDeviceId, openaiCatalog = new OpenAIModelCatalog() }) {
    this.openaiCatalog = openaiCatalog;
    this.getDeviceId = getDeviceId;
    this.CredentialSynchronizationError = null;
    this.packageRoot = path.resolve(packageRoot);
    this.ModelRuntime = null;
    this.SessionManager = null;
    this.modulePromise = null;
    this.runtimes = new Map();
  }

  /** Load and cache the SDK module (ModelRuntime + SessionManager). */
  loadModule() {
    if (this.modulePromise) return this.modulePromise;
    const pending = (async () => {
      let module;
      try {
        module = await import(pathToFileURL(path.join(this.packageRoot, 'dist', 'index.js')).href);
      } catch (error) {
        throw new AuthError('sdk_load');
      }
      if (typeof module?.ModelRuntime !== 'function') {
        throw new PiSdkError('Installed Pi SDK does not export ModelRuntime');
      }
      this.CredentialSynchronizationError = typeof module.CredentialSynchronizationError === 'function'
        ? module.CredentialSynchronizationError : null;
      const require = createRequire(path.join(this.packageRoot, 'package.json'));
      const ai = await import(pathToFileURL(require.resolve('@earendil-works/pi-ai')).href);
      if (typeof ai.getSupportedThinkingLevels !== 'function') throw new PiSdkError('Installed Pi SDK lacks thinking capabilities');
      this.getSupportedThinkingLevels = ai.getSupportedThinkingLevels;
      this.ModelRuntime = module.ModelRuntime;
      this.SessionManager = typeof module.SessionManager === 'function' ? module.SessionManager : null;
      return this.ModelRuntime;
    })();
    this.modulePromise = pending;
    pending.catch(() => {
      if (this.modulePromise === pending) this.modulePromise = null;
    });
    return pending;
  }

  /**
   * Cached ModelRuntime per explicit profile agent directory. The promise is
   * cached before the first await so concurrent callers share one runtime
   * instead of each constructing a ModelRuntime.
   */
  runtimeFor(agentDir) {
    const key = path.resolve(agentDir);
    const cached = this.runtimes.get(key);
    if (cached) return cached;
    const pending = (async () => {
      const ModelRuntime = await this.loadModule();
      try {
        return await ModelRuntime.create({
          authPath: path.join(key, 'auth.json'),
          modelsPath: path.join(key, 'models.json'),
          modelsStorePath: path.join(key, 'models-store.json'),
          refreshOnCreate: false,
          allowModelNetwork: false,
        });
      } catch (error) {
        throw new AuthError('runtime_create');
      }
    })();
    this.runtimes.set(key, pending);
    pending.catch(() => {
      if (this.runtimes.get(key) === pending) this.runtimes.delete(key);
    });
    return pending;
  }

  /**
   * Read one session's active-branch messages without spawning a Pi process.
   * Prefers the SDK's SessionManager.open/getBranch and falls back to a
   * read-only parentId walk.
   */
  async sessionMessages(sessionFile) {
    try {
      await this.loadModule();
      if (this.SessionManager) {
        const manager = this.SessionManager.open(sessionFile);
        return messageEntriesToMessages(manager.getBranch());
      }
    } catch {
      // Fall through to the offline branch walk below.
    }
    return readSessionMessages(sessionFile);
  }

  /**
   * Providers this profile has credentials for. Uses only profile-scoped
   * signals: the explicit credential store (auth.json) and profile models.json
   * configuration. Never runs an availability refresh or touches ambient
   * host credentials.
   */
  async configuredProviders(agentDir) {
    const runtime = await this.runtimeFor(agentDir);
    const configured = new Set();
    const blocked = new Set();
    // SDK listCredentials exposes only providerId/type, never secret material.
    for (const info of await runtime.listCredentials()) {
      if (!credentialAllowed(info.providerId, info.type)) blocked.add(info.providerId);
      else if (runtime.getProvider(info.providerId)) configured.add(info.providerId);
    }
    for (const provider of runtime.getProviders()) {
      if (!credentialAllowed(provider.id, 'api_key') || blocked.has(provider.id)) continue;
      if (runtime.getProviderAuthStatus(provider.id)?.configured === true) configured.add(provider.id);
    }
    for (const providerId of blocked) configured.delete(providerId);
    return configured;
  }

  /**
   * Catalog for one profile. Only providers this profile has configured auth
   * for contribute models; ambient host credentials are absent by construction
   * and never queried.
   */
  async catalog(agentDir, { refreshOpenAI = false } = {}) {
    const runtime = await this.runtimeFor(agentDir);
    const configured = await this.configuredProviders(agentDir);
    const oauthOpenAI = (await runtime.listCredentials()).some((c) => c.providerId === 'openai' && c.type === 'oauth');
    const discovery = oauthOpenAI
      ? await this.openaiCatalog.get(agentDir, runtime, { refresh: refreshOpenAI }) : null;
    const providers = [];
    const models = [];
    const seen = new Set();
    for (const provider of runtime.getProviders()) {
      const authenticated = configured.has(provider.id);
      providers.push({
        id: provider.id,
        name: typeof provider.name === 'string' && provider.name.length > 0 ? provider.name : provider.id,
        authTypes: authTypesOf(provider),
        authenticated,
      });
      if (!authenticated) continue;
      if (provider.id === 'openai' && oauthOpenAI) {
        const configuredModels = runtime.getModels(provider.id);
        models.push(...discovery.models.map((entry) => {
          const model = configuredModels.find((candidate) => candidate.id === entry.id);
          return { ...entry, thinkingLevels: entry.selectable && model ? this.getSupportedThinkingLevels(model) : [] };
        }));
        continue;
      }
      for (const model of runtime.getModels(provider.id)) {
        const key = `${provider.id}\u0000${model.id}`;
        if (seen.has(key)) continue;
        seen.add(key);
        models.push({
          provider: provider.id,
          id: model.id,
          name: typeof model.name === 'string' && model.name.length > 0 ? model.name : model.id,
          source: 'sdk',
          selectable: true,
          thinkingLevels: this.getSupportedThinkingLevels(model),
        });
      }
    }
    return { providers, models, ...(discovery ? { openaiDiscovery: discovery.openaiDiscovery } : {}) };
  }

  /** {authTypes, authenticated} for one provider, or null when unknown. */
  async providerStatus(agentDir, providerId) {
    const runtime = await this.runtimeFor(agentDir);
    const provider = runtime.getProvider(providerId);
    if (!provider) return null;
    const configured = await this.configuredProviders(agentDir);
    return {
      authTypes: authTypesOf(provider),
      authenticated: configured.has(providerId),
    };
  }

  /** True when the model is in the profile's catalog and its provider is configured. */
  async modelAvailable(agentDir, providerId, modelId) {
    const runtime = await this.runtimeFor(agentDir);
    const provider = runtime.getProvider(providerId);
    if (!provider) return false;
    const configured = await this.configuredProviders(agentDir);
    if (!configured.has(providerId)) return false;
    if (providerId === 'openai' && (await runtime.listCredentials()).some((c) => c.providerId === 'openai' && c.type === 'oauth')) {
      const catalog = await this.openaiCatalog.get(agentDir, runtime);
      return catalog.models.some((model) => model.id === modelId && model.selectable);
    }
    return runtime.getModels(providerId).some((model) => model.id === modelId);
  }

  /** Offline, profile-policy-filtered capabilities for an existing upstream identity. */
  async modelThinkingLevels(agentDir, providerId, modelId) {
    const { models } = await this.catalog(agentDir);
    const model = models.find((entry) => entry.provider === providerId && entry.id === modelId && entry.selectable);
    return model?.thinkingLevels ?? [];
  }

  invalidateOpenAI(agentDir) { this.openaiCatalog.invalidate(agentDir); }

  close() { this.openaiCatalog.close(); }

  /** Runs the SDK login flow. The returned credential is intentionally discarded. */
  async login({ agentDir, providerId, method, interaction }) {
    const runtime = await this.runtimeFor(agentDir);
    const provider = runtime.getProvider(providerId);
    if (!provider || !authTypesOf(provider).includes(method)) throw new AuthError('unsupported');
    if (providerId === 'openai') this.invalidateOpenAI(agentDir);
    try {
      await runtime.login(providerId, method, interaction, {
        getDeviceId: () => {
          if (typeof this.getDeviceId !== 'function') throw new AuthError('identity_missing');
          try { return this.getDeviceId(); } catch (error) {
            if (error instanceof AuthError) throw error;
            throw new AuthError('identity_persistence');
          }
        },
      });
    } catch (error) {
      if (this.CredentialSynchronizationError && error instanceof this.CredentialSynchronizationError) {
        throw new AuthError('credential_sync');
      }
      throw error;
    }
  }

  async logout({ agentDir, providerId, signal }) {
    const runtime = await this.runtimeFor(agentDir);
    if (!runtime.getProvider(providerId)) throw new AuthError('unsupported');
    if (providerId === 'openai') this.invalidateOpenAI(agentDir);
    await runtime.logout(providerId, { signal });
  }
}
