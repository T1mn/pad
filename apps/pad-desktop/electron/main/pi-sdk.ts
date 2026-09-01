import { randomUUID } from 'node:crypto';
import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import type { AuthSnapshotDto, AuthType } from '../../shared/protocol';
import type { StoredProfile } from './local-store';

const AUTH_SCRIPT = String.raw`
import { createInterface } from "node:readline";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
const send = (value) => process.stdout.write(JSON.stringify(value) + "\n");
const pending = new Map();
const input = createInterface({ input: process.stdin });
input.on("line", (line) => {
  try {
    const value = JSON.parse(line);
    if (value.type === "response" && pending.has(value.id)) {
      pending.get(value.id)(value);
      pending.delete(value.id);
    }
  } catch {}
});
const waitForResponse = (id) => new Promise((resolve) => pending.set(id, resolve));
const interaction = {
  prompt: async (value) => {
    const id = randomUUID();
    send({ type: "prompt", id, kind: value.type, message: value.message,
      placeholder: value.placeholder, options: value.options ?? [] });
    const response = await waitForResponse(id);
    if (response.cancelled) throw new Error("Authentication cancelled");
    return String(response.value ?? "");
  },
  notify: (event) => send({ type: "event", event }),
};
try {
  const agentDir = process.env.PAD_AUTH_AGENT_DIR;
  const runtime = await ModelRuntime.create({
    authPath: path.join(agentDir, "auth.json"),
    modelsPath: path.join(agentDir, "models.json"),
    refreshOnCreate: false,
  });
  const provider = process.env.PAD_AUTH_PROVIDER;
  if (process.env.PAD_AUTH_OPERATION === "logout") await runtime.logout(provider);
  else await runtime.login(provider, process.env.PAD_AUTH_TYPE, interaction);
  send({ type: "success", provider });
} catch (error) {
  send({ type: "error", message: error instanceof Error ? error.message : String(error) });
  process.exitCode = 1;
} finally { input.close(); }
`;

const MODEL_CATALOG_SCRIPT = String.raw`
import path from "node:path";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
const agentDir = process.env.PAD_MODEL_CATALOG_AGENT_DIR;
const providers = JSON.parse(process.env.PAD_MODEL_CATALOG_AUTHENTICATED_PROVIDERS ?? "[]");
const modelValue = (model) => ({
  provider: typeof model.provider === "string" ? model.provider : "",
  id: typeof model.id === "string" ? model.id : "",
  name: typeof model.name === "string" ? model.name : model.id,
  api: typeof model.api === "string" ? model.api : "",
  reasoning: model.reasoning === true,
  reasoning_levels: model.thinkingLevelMap && typeof model.thinkingLevelMap === "object" ? Object.keys(model.thinkingLevelMap) : [],
  input: Array.isArray(model.input) ? model.input.filter((item) => typeof item === "string") : [],
  context_window: Number.isFinite(model.contextWindow) ? model.contextWindow : null,
  max_tokens: Number.isFinite(model.maxTokens) ? model.maxTokens : null,
});
const unique = (models) => {
  const seen = new Set();
  return models.map(modelValue).filter((model) => {
    const key = model.provider + "\0" + model.id;
    if (!model.provider || !model.id || seen.has(key)) return false;
    seen.add(key); return true;
  });
};
try {
  const runtime = await ModelRuntime.create({
    authPath: path.join(agentDir, "auth.json"),
    modelsPath: path.join(agentDir, "models.json"),
    modelsStorePath: path.join(agentDir, "models-store.json"),
    refreshOnCreate: false,
    allowModelNetwork: false,
  });
  if (process.env.PAD_MODEL_CATALOG_REFRESH === "1") await runtime.refresh({ allowNetwork: false });
  const allModels = unique(runtime.getModels());
  const availableModels = unique((await Promise.all(providers.map(async (provider) => {
    try { return await runtime.getAvailable(provider, { signal: AbortSignal.timeout(1200) }); }
    catch { return runtime.getModels(provider); }
  }))).flat());
  const grouped = new Map();
  for (const model of availableModels) grouped.set(model.provider, [...(grouped.get(model.provider) ?? []), model]);
  process.stdout.write(JSON.stringify({
    status: "ready", source: "pi_model_runtime", models: availableModels,
    available_models: availableModels, all_models: allModels,
    providers: [...grouped.entries()].map(([id, models]) => ({ id, name: runtime.getProvider(id)?.name ?? id, authenticated: true, models })),
    counts: { all: allModels.length, available: availableModels.length }, checked_at: Date.now(),
  }));
} catch {
  process.stdout.write(JSON.stringify({ status: "unavailable", source: "pi_model_runtime", models: [], available_models: [], all_models: [], providers: [] }));
  process.exitCode = 1;
}
`;

function nodeProgram(resourcesPath: string): string {
  const candidates = [path.join(resourcesPath, 'bin', 'node'), '/opt/homebrew/bin/node', '/usr/local/bin/node', process.execPath];
  return candidates.find(existsSync) ?? process.execPath;
}

function piPackage(resourcesPath: string): string {
  const candidates = [
    path.join(resourcesPath, 'pi'),
    '/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent',
    '/usr/local/lib/node_modules/@earendil-works/pi-coding-agent',
  ];
  const selected = candidates.find((candidate) => existsSync(path.join(candidate, 'package.json')));
  if (!selected) throw new Error('Pi SDK is unavailable');
  return selected;
}

function sdkEnvironment(source: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  const result: NodeJS.ProcessEnv = { PATH: source.PATH ?? '/usr/bin:/bin:/usr/sbin:/sbin' };
  for (const key of [
    'HOME', 'USER', 'LOGNAME', 'SHELL', 'TMPDIR', 'LANG', 'LC_ALL', 'LC_CTYPE',
    'HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'NO_PROXY',
    'http_proxy', 'https_proxy', 'all_proxy', 'no_proxy',
  ]) if (source[key]) result[key] = source[key];
  return result;
}

export function authenticatedProviders(profile: StoredProfile): string[] {
  try {
    const value = JSON.parse(readFileSync(path.join(profile.agent_dir, 'auth.json'), 'utf8')) as unknown;
    if (!value || typeof value !== 'object' || Array.isArray(value)) return [];
    return Object.keys(value).filter((provider) => provider.length > 0).sort();
  } catch {
    return [];
  }
}

export function modelCatalog(
  resourcesPath: string,
  profile: StoredProfile,
  refresh: boolean,
  source: NodeJS.ProcessEnv = process.env,
): Record<string, unknown> {
  const providers = authenticatedProviders(profile);
  const output = spawnSync(nodeProgram(resourcesPath), ['--input-type=module', '-e', MODEL_CATALOG_SCRIPT], {
    cwd: piPackage(resourcesPath),
    encoding: 'utf8',
    timeout: 8_000,
    maxBuffer: 8 * 1024 * 1024,
    env: {
      ...sdkEnvironment(source),
      PAD_MODEL_CATALOG_AGENT_DIR: profile.agent_dir,
      PAD_MODEL_CATALOG_REFRESH: refresh ? '1' : '0',
      PAD_MODEL_CATALOG_AUTHENTICATED_PROVIDERS: JSON.stringify(providers),
      PI_CODING_AGENT_DIR: profile.agent_dir,
    },
  });
  if (output.error || !output.stdout) throw new Error('Pi model catalog is unavailable');
  const value = JSON.parse(output.stdout) as Record<string, unknown>;
  const available = Array.isArray(value.available_models) ? value.available_models : [];
  const first = available.find((item): item is Record<string, unknown> => !!item && typeof item === 'object' && !Array.isArray(item));
  return {
    ...value,
    profile_id: profile.id,
    authenticated_providers: providers,
    selected_provider: profile.default_provider ?? (typeof first?.provider === 'string' ? first.provider : null),
    selected_model: profile.default_model ?? (typeof first?.id === 'string' ? first.id : null),
  };
}

interface AuthAttempt {
  child: ChildProcessWithoutNullStreams;
  buffer: Buffer;
}

export class AuthCoordinator {
  private attempt: AuthAttempt | null = null;
  private snapshot: AuthSnapshotDto = { operation: 'login', phase: 'idle', notices: [], updated_at: Date.now() };

  constructor(
    private readonly resourcesPath: string,
    private readonly environment: NodeJS.ProcessEnv,
    private readonly changed: () => void,
  ) {}

  begin(profile: StoredProfile, provider: string, authType: AuthType, operation: 'login' | 'logout' = 'login'): AuthSnapshotDto {
    this.cancelCurrent(false);
    const attemptId = randomUUID();
    const child = spawn(nodeProgram(this.resourcesPath), ['--input-type=module', '-e', AUTH_SCRIPT], {
      cwd: piPackage(this.resourcesPath),
      stdio: ['pipe', 'pipe', 'pipe'],
      env: {
        ...sdkEnvironment(this.environment),
        PAD_AUTH_AGENT_DIR: profile.agent_dir,
        PAD_AUTH_PROVIDER: provider,
        PAD_AUTH_TYPE: authType,
        PAD_AUTH_OPERATION: operation,
        PI_CODING_AGENT_DIR: profile.agent_dir,
      },
    });
    this.snapshot = {
      attempt_id: attemptId,
      profile_id: profile.id,
      provider,
      auth_type: operation === 'login' ? authType : undefined,
      operation,
      phase: 'running',
      notices: [],
      updated_at: Date.now(),
    };
    this.attempt = { child, buffer: Buffer.alloc(0) };
    child.stdout.on('data', (chunk: Buffer) => this.consume(chunk));
    child.once('error', (error) => this.fail(error.message));
    child.once('exit', (code) => {
      if (this.snapshot.phase === 'running') this.fail(code === 0 ? 'Authentication ended before completion' : 'Authentication failed');
      this.attempt = null;
    });
    this.changed();
    return this.status();
  }

  status(): AuthSnapshotDto {
    return structuredClone(this.snapshot);
  }

  respond(attemptId: string, promptId: string, value: unknown, cancelled: boolean): AuthSnapshotDto {
    if (this.snapshot.attempt_id !== attemptId || !this.attempt) throw new Error('Authentication attempt is no longer active');
    this.attempt.child.stdin.write(`${JSON.stringify({ type: 'response', id: promptId, value, cancelled })}\n`);
    this.snapshot = { ...this.snapshot, prompt: undefined, updated_at: Date.now() };
    return this.status();
  }

  cancel(attemptId: string): AuthSnapshotDto {
    if (this.snapshot.attempt_id !== attemptId) throw new Error('Authentication attempt is no longer active');
    this.cancelCurrent(true);
    return this.status();
  }

  stop(): void {
    this.cancelCurrent(false);
  }

  private consume(chunk: Buffer): void {
    const attempt = this.attempt;
    if (!attempt) return;
    attempt.buffer = Buffer.concat([attempt.buffer, chunk]);
    let newline = attempt.buffer.indexOf(0x0a);
    while (newline >= 0) {
      const frame = attempt.buffer.subarray(0, newline);
      attempt.buffer = attempt.buffer.subarray(newline + 1);
      try {
        const message = JSON.parse(frame.toString('utf8')) as Record<string, unknown>;
        this.handleMessage(message);
      } catch { /* ignore malformed helper output */ }
      newline = attempt.buffer.indexOf(0x0a);
    }
  }

  private handleMessage(message: Record<string, unknown>): void {
    if (message.type === 'prompt') {
      const options = Array.isArray(message.options) ? message.options.map((option, index) => {
        if (typeof option === 'string') return { id: String(index), label: option };
        const item = option && typeof option === 'object' ? option as Record<string, unknown> : {};
        return { id: String(item.id ?? index), label: String(item.label ?? item.name ?? item.id ?? index), description: typeof item.description === 'string' ? item.description : undefined };
      }) : [];
      this.snapshot = {
        ...this.snapshot,
        prompt: {
          id: String(message.id ?? ''),
          kind: String(message.kind ?? 'input'),
          message: String(message.message ?? ''),
          placeholder: typeof message.placeholder === 'string' ? message.placeholder : undefined,
          options,
        },
        updated_at: Date.now(),
      };
    } else if (message.type === 'event') {
      const event = message.event && typeof message.event === 'object' ? message.event as Record<string, unknown> : {};
      this.snapshot = {
        ...this.snapshot,
        notices: [...this.snapshot.notices, {
          kind: String(event.type ?? event.kind ?? 'info'),
          message: String(event.message ?? ''),
          url: typeof event.url === 'string' ? event.url : undefined,
          user_code: typeof event.userCode === 'string' ? event.userCode : undefined,
        }].slice(-32),
        updated_at: Date.now(),
      };
    } else if (message.type === 'success') {
      this.snapshot = { ...this.snapshot, phase: 'succeeded', prompt: undefined, updated_at: Date.now() };
    } else if (message.type === 'error') {
      this.fail(String(message.message ?? 'Authentication failed'));
      return;
    }
    this.changed();
  }

  private fail(message: string): void {
    this.snapshot = { ...this.snapshot, phase: 'failed', prompt: undefined, error: message, updated_at: Date.now() };
    this.changed();
  }

  private cancelCurrent(markCancelled: boolean): void {
    if (this.attempt) {
      this.attempt.child.kill('SIGTERM');
      this.attempt = null;
    }
    if (markCancelled) {
      this.snapshot = { ...this.snapshot, phase: 'cancelled', prompt: undefined, updated_at: Date.now() };
      this.changed();
    }
  }
}
