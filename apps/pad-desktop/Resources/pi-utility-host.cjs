'use strict';

const path = require('node:path');
const { PassThrough } = require('node:stream');
const { pathToFileURL } = require('node:url');
const { randomUUID } = require('node:crypto');

const [mode, piPackage, ...args] = process.argv.slice(2);
const parentPort = process.parentPort;

if (!parentPort || typeof parentPort.on !== 'function') {
  throw new Error('PAD Pi utility host requires an Electron utility process');
}
if (!path.isAbsolute(piPackage || '')) throw new Error('PAD Pi package path must be absolute');

const input = new PassThrough();
Object.defineProperty(process, 'stdin', { configurable: true, enumerable: true, value: input });
parentPort.on('message', (event) => {
  const message = event && typeof event === 'object' ? event.data : null;
  if (message && message.type === 'stdin' && typeof message.value === 'string') {
    input.write(message.value);
  }
});

const importPiFile = (relative) => import(pathToFileURL(path.join(piPackage, relative)).href);
const send = (value) => new Promise((resolve, reject) => {
  process.stdout.write(`${JSON.stringify(value)}\n`, (error) => error ? reject(error) : resolve());
});

async function runPiRpc() {
  const [{ main }, { configureHttpDispatcher }] = await Promise.all([
    importPiFile('dist/main.js'),
    importPiFile('dist/core/http-dispatcher.js'),
  ]);
  process.title = 'pi-rpc';
  process.env.PI_CODING_AGENT = 'true';
  process.env.AI_AGENT = 'pi';
  process.emitWarning = () => undefined;
  configureHttpDispatcher();
  await main(['--mode', 'rpc', ...args]);
}

function modelValue(model) {
  return {
    provider: typeof model.provider === 'string' ? model.provider : '',
    id: typeof model.id === 'string' ? model.id : '',
    name: typeof model.name === 'string' ? model.name : model.id,
    api: typeof model.api === 'string' ? model.api : '',
    reasoning: model.reasoning === true,
    reasoning_levels: model.thinkingLevelMap && typeof model.thinkingLevelMap === 'object'
      ? Object.keys(model.thinkingLevelMap) : [],
    input: Array.isArray(model.input) ? model.input.filter((item) => typeof item === 'string') : [],
    context_window: Number.isFinite(model.contextWindow) ? model.contextWindow : null,
    max_tokens: Number.isFinite(model.maxTokens) ? model.maxTokens : null,
  };
}

function uniqueModels(models) {
  const seen = new Set();
  return models.map(modelValue).filter((model) => {
    const key = `${model.provider}\0${model.id}`;
    if (!model.provider || !model.id || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

async function modelRuntime() {
  const { ModelRuntime } = await importPiFile('dist/index.js');
  const agentDir = process.env.PAD_MODEL_CATALOG_AGENT_DIR;
  const providers = JSON.parse(process.env.PAD_MODEL_CATALOG_AUTHENTICATED_PROVIDERS || '[]');
  const runtime = await ModelRuntime.create({
    authPath: path.join(agentDir, 'auth.json'),
    modelsPath: path.join(agentDir, 'models.json'),
    modelsStorePath: path.join(agentDir, 'models-store.json'),
    refreshOnCreate: false,
    allowModelNetwork: false,
  });
  if (process.env.PAD_MODEL_CATALOG_REFRESH === '1') await runtime.refresh({ allowNetwork: false });
  const allModels = uniqueModels(runtime.getModels());
  const availableModels = uniqueModels((await Promise.all(providers.map(async (provider) => {
    try { return await runtime.getAvailable(provider, { signal: AbortSignal.timeout(1200) }); }
    catch { return runtime.getModels(provider); }
  }))).flat());
  const grouped = new Map();
  for (const model of availableModels) {
    grouped.set(model.provider, [...(grouped.get(model.provider) || []), model]);
  }
  await send({
    status: 'ready',
    source: 'pi_model_runtime',
    models: availableModels,
    available_models: availableModels,
    all_models: allModels,
    providers: [...grouped.entries()].map(([id, models]) => ({
      id,
      name: runtime.getProvider(id)?.name || id,
      authenticated: true,
      models,
    })),
    counts: { all: allModels.length, available: availableModels.length },
    checked_at: Date.now(),
  });
}

async function authenticate() {
  const { ModelRuntime } = await importPiFile('dist/index.js');
  const pending = new Map();
  input.setEncoding('utf8');
  let buffer = '';
  input.on('data', (chunk) => {
    buffer += chunk;
    let newline = buffer.indexOf('\n');
    while (newline >= 0) {
      const frame = buffer.slice(0, newline);
      buffer = buffer.slice(newline + 1);
      try {
        const value = JSON.parse(frame);
        if (value.type === 'response' && pending.has(value.id)) {
          pending.get(value.id)(value);
          pending.delete(value.id);
        }
      } catch { /* ignore malformed input */ }
      newline = buffer.indexOf('\n');
    }
  });
  const interaction = {
    prompt: async (value) => {
      const id = randomUUID();
      await send({
        type: 'prompt', id, kind: value.type, message: value.message,
        placeholder: value.placeholder, options: value.options || [],
      });
      const response = await new Promise((resolve) => pending.set(id, resolve));
      if (response.cancelled) throw new Error('Authentication cancelled');
      return String(response.value || '');
    },
    notify: (event) => { void send({ type: 'event', event }); },
  };
  const agentDir = process.env.PAD_AUTH_AGENT_DIR;
  const runtime = await ModelRuntime.create({
    authPath: path.join(agentDir, 'auth.json'),
    modelsPath: path.join(agentDir, 'models.json'),
    refreshOnCreate: false,
  });
  const provider = process.env.PAD_AUTH_PROVIDER;
  if (process.env.PAD_AUTH_OPERATION === 'logout') await runtime.logout(provider);
  else await runtime.login(provider, process.env.PAD_AUTH_TYPE, interaction);
  await send({ type: 'success', provider });
}

async function main() {
  if (mode === 'pi-rpc') await runPiRpc();
  else if (mode === 'model-catalog') await modelRuntime();
  else if (mode === 'auth') await authenticate();
  else throw new Error(`Unsupported PAD utility mode: ${String(mode)}`);
}

function exitAfterFlush(code) {
  input.end();
  process.stdout.end(() => process.exit(code));
  setTimeout(() => process.exit(code), 1000).unref();
}

main().then(
  () => exitAfterFlush(0),
  async (error) => {
    if (mode === 'model-catalog') {
      await send({
        status: 'unavailable', source: 'pi_model_runtime', models: [],
        available_models: [], all_models: [], providers: [],
      }).catch(() => undefined);
    } else if (mode === 'auth') {
      await send({ type: 'error', message: error instanceof Error ? error.message : String(error) })
        .catch(() => undefined);
    } else {
      process.stderr.write(`${error instanceof Error ? error.stack : String(error)}\n`);
    }
    exitAfterFlush(1);
  },
);
