import { randomUUID } from 'node:crypto';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { EventEmitter } from 'node:events';
import type { StoredProfile, StoredTask } from './local-store';

const MAX_FRAME_BYTES = 8 * 1024 * 1024;
const REQUEST_TIMEOUT_MS = 10_000;

const FAST_EXTENSION = `
import { readFileSync } from "node:fs";
const modeFile = process.env.PAD_PI_FAST_MODE_FILE;
const enabled = () => {
  if (!modeFile) return true;
  try { return readFileSync(modeFile, "utf8").trim().toLowerCase() !== "off"; }
  catch { return true; }
};
export default function padFastMode(pi) {
  pi.on("before_provider_request", (event, ctx) => {
    if (!enabled() || ctx.model?.provider !== "openai-codex") return;
    if (!event.payload || typeof event.payload !== "object" || Array.isArray(event.payload)) return;
    // Codex exposes this as the "Fast" UI tier, but the Responses API value is
    // "priority". Sending the UI label ("fast") is rejected by Codex.
    return { ...event.payload, service_tier: "priority" };
  });
}
`;

export interface PiRuntimeOptions {
  resourcesPath: string;
  environment?: NodeJS.ProcessEnv;
  onTaskChanged(taskId: string, patch: Partial<StoredTask>): void;
  onEvent(taskId: string): void;
}

interface PendingCommand {
  command: string;
  resolve(value: Record<string, unknown>): void;
  reject(error: Error): void;
  timer: NodeJS.Timeout;
}

export interface PendingUiRequest {
  id: string;
  kind: string;
  title?: string;
  message?: string;
  options: string[];
  default_index?: number;
  default?: string;
  requires_response: boolean;
  response_action: 'respond_ui';
}

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function childEnvironment(profile: StoredProfile, source: NodeJS.ProcessEnv, fastModeFile: string): NodeJS.ProcessEnv {
  const inheritedPath = source.PATH ?? '/usr/bin:/bin:/usr/sbin:/sbin';
  const environment: NodeJS.ProcessEnv = {
    PATH: inheritedPath,
    PI_CODING_AGENT_DIR: profile.agent_dir,
    PI_CODING_AGENT_SESSION_DIR: profile.session_dir,
    PAD_PI_FAST_MODE_FILE: fastModeFile,
  };
  for (const key of [
    'HOME', 'USER', 'LOGNAME', 'SHELL', 'TMPDIR', 'LANG', 'LC_ALL', 'LC_CTYPE', 'TERM',
    'HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'NO_PROXY',
    'http_proxy', 'https_proxy', 'all_proxy', 'no_proxy',
  ]) {
    if (source[key]) environment[key] = source[key];
  }
  return environment;
}

function piCommand(resourcesPath: string): { program: string; prefix: string[] } {
  const bundledNode = path.join(resourcesPath, 'bin', 'node');
  const bundledNodePi = path.join(resourcesPath, 'pi', 'dist', 'bundle', 'cli.js');
  if (existsSync(bundledNode) && existsSync(bundledNodePi)) {
    return { program: bundledNode, prefix: [bundledNodePi] };
  }
  const bundledBun = path.join(resourcesPath, 'bin', 'bun');
  const bundledPi = path.join(resourcesPath, 'pi', 'dist', 'bun', 'cli.js');
  if (existsSync(bundledBun) && existsSync(bundledPi)) {
    return { program: bundledBun, prefix: [bundledPi] };
  }
  const selected = [
    path.join(resourcesPath, 'bin', 'pi'),
    '/opt/homebrew/bin/pi',
    '/usr/local/bin/pi',
  ].find(existsSync);
  if (!selected) throw new Error('Pi runtime is unavailable');
  return { program: selected, prefix: [] };
}

function taskStatePath(profile: StoredProfile, taskId: string): string {
  const directory = path.join(profile.agent_dir, 'pad-task-state');
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  const safeId = taskId.replace(/[^a-zA-Z0-9._-]+/g, '-').slice(0, 120) || randomUUID();
  return path.join(directory, `${safeId}.fast-mode`);
}

function writeFastMode(filePath: string, enabled: boolean): void {
  writeFileSync(filePath, enabled ? 'on\n' : 'off\n', { encoding: 'utf8', mode: 0o600 });
}

function writeExtension(profile: StoredProfile): string {
  mkdirSync(profile.agent_dir, { recursive: true, mode: 0o700 });
  const extension = path.join(profile.agent_dir, 'pad-fast-mode.ts');
  writeFileSync(extension, FAST_EXTENSION, { encoding: 'utf8', mode: 0o600 });
  return extension;
}

function readSessionMessages(sessionFile: string | null): unknown[] {
  if (!sessionFile) return [];
  try {
    return readFileSync(sessionFile, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        try { return JSON.parse(line) as unknown; } catch { return null; }
      })
      .map(record)
      .filter((entry): entry is Record<string, unknown> => entry?.type === 'message')
      .map((entry) => entry.message)
      .filter((message) => message !== undefined);
  } catch {
    return [];
  }
}

class PiProcess extends EventEmitter {
  private readonly child: ChildProcessWithoutNullStreams;
  private buffer = Buffer.alloc(0);
  private readonly pending = new Map<string, PendingCommand>();
  private readonly pendingOrder: string[] = [];
  private readonly events: Record<string, unknown>[] = [];
  private readonly pendingUi = new Map<string, PendingUiRequest>();
  private closed = false;
  status = 'starting';

  constructor(
    readonly task: StoredTask,
    readonly profile: StoredProfile,
    options: PiRuntimeOptions,
    launch: { provider?: string; model?: string; thinkingLevel?: string; fastMode: boolean },
  ) {
    super();
    const fastModeFile = taskStatePath(profile, task.id);
    writeFastMode(fastModeFile, launch.fastMode);
    const args = ['--mode', 'rpc', '--extension', writeExtension(profile), '--approve'];
    if (task.session_file) args.push('--session', task.session_file);
    if (launch.provider && launch.model) args.push('--provider', launch.provider, '--model', launch.model);
    if (launch.thinkingLevel && launch.thinkingLevel !== 'default') args.push('--thinking', launch.thinkingLevel);
    const runtime = piCommand(options.resourcesPath);
    this.child = spawn(runtime.program, [...runtime.prefix, ...args], {
      cwd: task.cwd,
      stdio: ['pipe', 'pipe', 'pipe'],
      env: childEnvironment(profile, options.environment ?? process.env, fastModeFile),
    });
    this.child.stdout.on('data', (chunk: Buffer) => this.consume(chunk, options));
    this.child.stderr.on('data', () => undefined);
    this.child.once('error', (error) => this.fail(error, options));
    this.child.once('exit', (code) => {
      this.closed = true;
      this.status = code === 0 ? 'disconnected' : 'failed';
      for (const pending of this.pending.values()) {
        clearTimeout(pending.timer);
        pending.reject(new Error(`Pi exited with code ${String(code)}`));
      }
      this.pending.clear();
      options.onTaskChanged(this.task.id, { status: this.status });
      options.onEvent(this.task.id);
      this.emit('exit');
    });
  }

  setFastMode(enabled: boolean): void {
    writeFastMode(taskStatePath(this.profile, this.task.id), enabled);
  }

  request(command: Record<string, unknown>, timeoutMs = REQUEST_TIMEOUT_MS): Promise<Record<string, unknown>> {
    if (this.closed || !this.child.stdin.writable) return Promise.reject(new Error('Pi task is not running'));
    const id = typeof command.id === 'string' ? command.id : `pad-${randomUUID()}`;
    const message = { ...command, id };
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        const index = this.pendingOrder.indexOf(id);
        if (index >= 0) this.pendingOrder.splice(index, 1);
        reject(new Error(`Pi ${String(command.type)} timed out`));
      }, timeoutMs);
      this.pending.set(id, { command: String(command.type ?? ''), resolve, reject, timer });
      this.pendingOrder.push(id);
      this.child.stdin.write(`${JSON.stringify(message)}\n`, (error) => {
        if (!error) return;
        clearTimeout(timer);
        this.pending.delete(id);
        reject(error);
      });
    });
  }

  poll(): { events: Record<string, unknown>[]; pending_ui_requests: PendingUiRequest[]; status: string } {
    const events = this.events.splice(0);
    return { events, pending_ui_requests: [...this.pendingUi.values()], status: this.status };
  }

  async respondUi(id: string, kind: string | undefined, value: unknown, cancelled: boolean): Promise<void> {
    const request = this.pendingUi.get(id);
    if (!request) throw new Error('Pi interaction is no longer pending');
    const response: Record<string, unknown> = { type: 'extension_ui_response', id };
    if (cancelled) response.cancelled = true;
    else if ((kind ?? request.kind) === 'confirm') response.confirmed = Boolean(value);
    else if ((kind ?? request.kind) === 'select' && typeof value === 'number') response.value = request.options[value];
    else response.value = value;
    this.pendingUi.delete(id);
    this.child.stdin.write(`${JSON.stringify(response)}\n`);
  }

  async stop(): Promise<void> {
    if (this.closed) return;
    try { await this.request({ type: 'abort' }, 1_000); } catch { /* process may already be idle */ }
    this.child.kill('SIGTERM');
    await Promise.race([
      new Promise<void>((resolve) => this.child.once('exit', () => resolve())),
      new Promise<void>((resolve) => setTimeout(resolve, 1_000)),
    ]);
    if (!this.closed) this.child.kill('SIGKILL');
  }

  private consume(chunk: Buffer, options: PiRuntimeOptions): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    if (this.buffer.length > MAX_FRAME_BYTES && !this.buffer.includes(0x0a)) {
      this.fail(new Error('Pi emitted an oversized frame'), options);
      return;
    }
    let newline = this.buffer.indexOf(0x0a);
    while (newline >= 0) {
      const frame = this.buffer.subarray(0, newline);
      this.buffer = this.buffer.subarray(newline + 1);
      if (frame.length) {
        try {
          const value = JSON.parse(frame.toString('utf8')) as unknown;
          const message = record(value);
          if (message) this.handleMessage(message, options);
        } catch {
          // Pi stderr remains diagnostic-only; a malformed line must not stop later frames.
        }
      }
      newline = this.buffer.indexOf(0x0a);
    }
  }

  private handleMessage(message: Record<string, unknown>, options: PiRuntimeOptions): void {
    if (message.type === 'response') {
      const responseId = typeof message.id === 'string' ? message.id : undefined;
      let pendingId = responseId && this.pending.has(responseId) ? responseId : undefined;
      if (!pendingId) {
        const command = typeof message.command === 'string' ? message.command : '';
        pendingId = this.pendingOrder.find((id) => this.pending.get(id)?.command === command);
      }
      if (pendingId) {
        const pending = this.pending.get(pendingId);
        if (pending) {
          clearTimeout(pending.timer);
          this.pending.delete(pendingId);
          const index = this.pendingOrder.indexOf(pendingId);
          if (index >= 0) this.pendingOrder.splice(index, 1);
          if (message.success === false) pending.reject(new Error(String(message.error ?? `Pi ${pending.command} failed`)));
          else pending.resolve(message);
        }
      }
      if (message.command === 'get_state') this.captureState(message, options);
      return;
    }

    const type = typeof message.type === 'string' ? message.type : '';
    if (type === 'extension_ui_request') {
      const method = typeof message.method === 'string' ? message.method : 'unknown';
      const id = typeof message.id === 'string' ? message.id : randomUUID();
      const permissionMode = this.task.policy?.mode ?? this.profile.policy?.mode;
      const unattended = this.task.policy?.unattended ?? this.profile.policy?.unattended;
      const fullAccess = permissionMode === 'system_full' && unattended === true;
      if (method === 'confirm' && fullAccess) {
        this.child.stdin.write(`${JSON.stringify({ type: 'extension_ui_response', id, confirmed: true })}\n`);
      } else if (['confirm', 'select', 'input', 'editor'].includes(method)) {
        const optionsValue = Array.isArray(message.options)
          ? message.options.filter((item): item is string => typeof item === 'string')
          : [];
        this.pendingUi.set(id, {
          id,
          kind: method,
          title: typeof message.title === 'string' ? message.title : undefined,
          message: typeof message.message === 'string' ? message.message : undefined,
          options: optionsValue,
          default_index: typeof message.defaultIndex === 'number' ? message.defaultIndex : undefined,
          default: typeof message.prefill === 'string' ? message.prefill : undefined,
          requires_response: true,
          response_action: 'respond_ui',
        });
        this.status = method === 'confirm' || method === 'select' ? 'needs_approval' : 'needs_input';
      }
    } else {
      this.events.push(message);
      this.status = statusFromEvent(message, this.status);
    }
    options.onTaskChanged(this.task.id, { status: this.status });
    options.onEvent(this.task.id);
  }

  private captureState(response: Record<string, unknown>, options: PiRuntimeOptions): void {
    const data = record(response.data);
    if (!data) return;
    options.onTaskChanged(this.task.id, {
      pi_session_id: typeof data.sessionId === 'string' ? data.sessionId : this.task.pi_session_id,
      session_file: typeof data.sessionFile === 'string' ? data.sessionFile : this.task.session_file,
    });
  }

  private fail(error: Error, options: PiRuntimeOptions): void {
    this.status = 'failed';
    options.onTaskChanged(this.task.id, { status: 'failed', summary: error.message });
    options.onEvent(this.task.id);
    this.child.kill('SIGTERM');
  }
}

function statusFromEvent(message: Record<string, unknown>, current: string): string {
  switch (message.type) {
    case 'agent_start':
    case 'turn_start':
    case 'turn_end':
    case 'tool_execution_end': return 'running';
    case 'message_start':
    case 'message_update': return 'streaming';
    case 'tool_execution_start':
    case 'tool_execution_update': return 'tool_running';
    case 'compaction_start': return 'compacting';
    case 'auto_retry_start': return 'retrying';
    case 'agent_settled': return current === 'failed' ? current : 'idle';
    case 'message_end': {
      const messageValue = record(message.message);
      const failed = messageValue?.stopReason === 'error' || typeof messageValue?.errorMessage === 'string';
      return failed ? 'failed' : 'running';
    }
    default: return current;
  }
}

export class PiRuntime {
  private readonly processes = new Map<string, PiProcess>();

  constructor(private readonly options: PiRuntimeOptions) {}

  isRunning(taskId: string): boolean {
    return this.processes.has(taskId);
  }

  async start(task: StoredTask, profile: StoredProfile, launch: {
    provider?: string;
    model?: string;
    thinkingLevel?: string;
    fastMode: boolean;
  }): Promise<PiProcess> {
    const existing = this.processes.get(task.id);
    if (existing) return existing;
    const process = new PiProcess(task, profile, this.options, launch);
    this.processes.set(task.id, process);
    process.once('exit', () => this.processes.delete(task.id));
    this.options.onTaskChanged(task.id, { status: 'starting' });
    await process.request({ type: 'get_state' });
    return process;
  }

  get(taskId: string): PiProcess | null {
    return this.processes.get(taskId) ?? null;
  }

  async stop(taskId: string): Promise<void> {
    const process = this.processes.get(taskId);
    if (!process) return;
    this.processes.delete(taskId);
    await process.stop();
  }

  async stopAll(): Promise<void> {
    await Promise.all([...this.processes.keys()].map((taskId) => this.stop(taskId)));
  }

  history(task: StoredTask): unknown[] {
    return readSessionMessages(task.session_file);
  }
}
