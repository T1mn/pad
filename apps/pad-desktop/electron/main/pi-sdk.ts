import { randomUUID } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import type { AuthSnapshotDto, AuthType } from '../../shared/protocol';
import type { StoredProfile } from './local-store';
import {
  type RuntimeProcess,
  type RuntimeProcessLauncher,
} from './runtime-process';

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

async function collectOutput(child: RuntimeProcess, timeoutMs: number): Promise<string> {
  return await new Promise((resolve, reject) => {
    let output = '';
    let settled = false;
    const finish = (error?: Error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) reject(error);
      else if (!output) reject(new Error('Pi helper returned no output'));
      else resolve(output);
    };
    const timer = setTimeout(() => {
      child.kill('SIGKILL');
      finish(new Error('Pi helper timed out'));
    }, timeoutMs);
    child.stdout.on('data', (chunk: Buffer | string) => {
      output += chunk.toString();
      if (Buffer.byteLength(output) > 8 * 1024 * 1024) {
        child.kill('SIGKILL');
        finish(new Error('Pi helper output is too large'));
      } else if (output.endsWith('\n')) {
        finish();
      }
    });
    child.once('error', (error) => finish(error));
    child.once('exit', () => finish());
  });
}

export async function modelCatalog(
  resourcesPath: string,
  profile: StoredProfile,
  refresh: boolean,
  source: NodeJS.ProcessEnv = process.env,
  runtimeLauncher?: RuntimeProcessLauncher,
): Promise<Record<string, unknown>> {
  const providers = authenticatedProviders(profile);
  const packagePath = piPackage(resourcesPath);
  const environment = {
    ...sdkEnvironment(source),
    PAD_MODEL_CATALOG_AGENT_DIR: profile.agent_dir,
    PAD_MODEL_CATALOG_REFRESH: refresh ? '1' : '0',
    PAD_MODEL_CATALOG_AUTHENTICATED_PROVIDERS: JSON.stringify(providers),
    PI_CODING_AGENT_DIR: profile.agent_dir,
  };
  if (!runtimeLauncher) throw new Error('Pi utility runtime is unavailable');
  const output = await collectOutput(runtimeLauncher({
    mode: 'model-catalog',
    piPackage: packagePath,
    cwd: packagePath,
    env: environment,
    serviceName: 'PAD Pi Model Catalog',
  }), 8_000);
  const value = JSON.parse(output) as Record<string, unknown>;
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
  child: RuntimeProcess;
  buffer: Buffer;
}

export class AuthCoordinator {
  private attempt: AuthAttempt | null = null;
  private snapshot: AuthSnapshotDto = { operation: 'login', phase: 'idle', notices: [], updated_at: Date.now() };

  constructor(
    private readonly resourcesPath: string,
    private readonly environment: NodeJS.ProcessEnv,
    private readonly changed: () => void,
    private readonly runtimeLauncher?: RuntimeProcessLauncher,
  ) {}

  begin(profile: StoredProfile, provider: string, authType: AuthType, operation: 'login' | 'logout' = 'login'): AuthSnapshotDto {
    this.cancelCurrent(false);
    const attemptId = randomUUID();
    const packagePath = piPackage(this.resourcesPath);
    const environment = {
      ...sdkEnvironment(this.environment),
      PAD_AUTH_AGENT_DIR: profile.agent_dir,
      PAD_AUTH_PROVIDER: provider,
      PAD_AUTH_TYPE: authType,
      PAD_AUTH_OPERATION: operation,
      PI_CODING_AGENT_DIR: profile.agent_dir,
    };
    if (!this.runtimeLauncher) throw new Error('Pi utility runtime is unavailable');
    const child = this.runtimeLauncher({
      mode: 'auth',
      piPackage: packagePath,
      cwd: packagePath,
      env: environment,
      serviceName: `PAD Pi Auth ${provider}`,
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
    this.attempt.child.write(`${JSON.stringify({ type: 'response', id: promptId, value, cancelled })}\n`);
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
