import { randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { EventEmitter } from 'node:events';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import type { StoredProfile, StoredTask } from './local-store';
import {
  nodeRuntimeProcess,
  type RuntimeProcess,
  type RuntimeProcessLauncher,
} from './runtime-process';

const MAX_FRAME_BYTES = 8 * 1024 * 1024;
const REQUEST_TIMEOUT_MS = 10_000;

const PAD_EXTENSION = `
import { Type } from "@earendil-works/pi-ai";
import { readFileSync } from "node:fs";
const modeFile = process.env.PAD_PI_FAST_MODE_FILE;
const collaborationEndpoint = process.env.PAD_COLLABORATION_ENDPOINT || "";
const collaborationToken = process.env.PAD_COLLABORATION_TOKEN || "";
const currentTaskId = process.env.PAD_TASK_ID || "";
const enabled = () => {
  if (!modeFile) return true;
  try { return readFileSync(modeFile, "utf8").trim().toLowerCase() !== "off"; }
  catch { return true; }
};

async function callPad(action, params, signal) {
  if (!collaborationEndpoint || !collaborationToken || !currentTaskId) throw new Error("PAD collaboration bridge is unavailable");
  const response = await fetch(collaborationEndpoint, {
    method: "POST",
    headers: { "content-type": "application/json", "authorization": "Bearer " + collaborationToken },
    body: JSON.stringify({ action, source_task_id: currentTaskId, params }),
    signal,
  });
  const payload = await response.json();
  if (!response.ok || !payload.ok) throw new Error(payload.error || "PAD collaboration request failed");
  return payload.result;
}

function result(value) {
  return { content: [{ type: "text", text: JSON.stringify(value, null, 2) }], details: { result: value } };
}

export default function padRuntime(pi) {
  pi.on("before_agent_start", (event, ctx) => {
    if (!ctx.model) return;
    const identity = "PAD runtime identity: current task id is " + currentTaskId + ", the active provider is " + ctx.model.provider + " and the active model id is " + ctx.model.id + ". Report this model id exactly when asked. PAD sessions may collaborate: use list_sessions/read_session to inspect another session; use rename_session whenever the user asks to rename this or another session; use spawn_agent for delegated work; use followup_task for an immediate message to another session or child agent; use send_message only when the message should wait until that session's next turn.";
    return { systemPrompt: event.systemPrompt + "\\n\\n" + identity };
  });
  pi.on("before_provider_request", (event, ctx) => {
    if (!enabled() || ctx.model?.provider !== "openai-codex") return;
    if (!event.payload || typeof event.payload !== "object" || Array.isArray(event.payload)) return;
    // Codex exposes this as the "Fast" UI tier, but the Responses API value is
    // "priority". Sending the UI label ("fast") is rejected by Codex.
    return { ...event.payload, service_tier: "priority" };
  });

  pi.registerTool({
    name: "list_sessions", label: "查询 Session", description: "List and search all local PAD sessions across accounts.",
    promptSnippet: "List or search PAD sessions",
    parameters: Type.Object({ query: Type.Optional(Type.String()), limit: Type.Optional(Type.Number({ minimum: 1, maximum: 50 })) }),
    async execute(_id, params, signal) { return result(await callPad("list_sessions", params, signal)); },
  });
  pi.registerTool({
    name: "read_session", label: "读取 Session", description: "Read recent messages from any local PAD session by task id.",
    promptSnippet: "Read another PAD session",
    parameters: Type.Object({ task_id: Type.String(), limit: Type.Optional(Type.Number({ minimum: 1, maximum: 50 })) }),
    async execute(_id, params, signal) { return result(await callPad("read_session", params, signal)); },
  });
  pi.registerTool({
    name: "rename_session", label: "重命名 Session", description: "Rename the current or another PAD session. Omit task_id to rename the current session.",
    promptSnippet: "Rename a PAD session",
    promptGuidelines: ["Use rename_session when the user asks to change a conversation, task, or session name."],
    parameters: Type.Object({ task_id: Type.Optional(Type.String()), name: Type.String({ minLength: 1, maxLength: 100 }) }),
    async execute(_id, params, signal) { return result(await callPad("rename_session", params, signal)); },
  });
  pi.registerTool({
    name: "spawn_agent", label: "创建子 Agent", description: "Create a persistent child Agent backed by its own Pi session and start its task.",
    promptSnippet: "Create a persistent child Agent",
    parameters: Type.Object({ task: Type.String({ minLength: 1 }), name: Type.Optional(Type.String({ maxLength: 100 })) }),
    async execute(_id, params, signal) { return result(await callPad("spawn_agent", params, signal)); },
  });
  pi.registerTool({
    name: "send_message", label: "发送 Session 消息", description: "Queue a message for another PAD session without starting a new turn.",
    promptSnippet: "Queue a message to another session",
    parameters: Type.Object({ task_id: Type.String(), message: Type.String({ minLength: 1 }) }),
    async execute(_id, params, signal) { return result(await callPad("send_message", params, signal)); },
  });
  pi.registerTool({
    name: "followup_task", label: "继续 Agent", description: "Send a message to another session or child Agent and immediately start or resume it.",
    promptSnippet: "Immediately continue another session or Agent",
    parameters: Type.Object({ task_id: Type.String(), message: Type.String({ minLength: 1 }) }),
    async execute(_id, params, signal) { return result(await callPad("followup_task", params, signal)); },
  });
  pi.registerTool({
    name: "wait_agent", label: "等待 Agent", description: "Wait briefly for a child Agent and return its latest state and messages.",
    promptSnippet: "Wait for a child Agent",
    parameters: Type.Object({ task_id: Type.String(), timeout_seconds: Type.Optional(Type.Number({ minimum: 0, maximum: 30 })) }),
    async execute(_id, params, signal) { return result(await callPad("wait_agent", params, signal)); },
  });
  pi.registerTool({
    name: "interrupt_agent", label: "中断 Agent", description: "Interrupt a running child Agent or PAD session.",
    promptSnippet: "Interrupt a child Agent",
    parameters: Type.Object({ task_id: Type.String() }),
    async execute(_id, params, signal) { return result(await callPad("interrupt_agent", params, signal)); },
  });
  pi.registerTool({
    name: "list_agents", label: "查看 Agent 树", description: "List the persistent child-Agent tree for the current or selected PAD session.",
    promptSnippet: "Inspect the child-Agent tree",
    parameters: Type.Object({ task_id: Type.Optional(Type.String()) }),
    async execute(_id, params, signal) { return result(await callPad("list_agents", params, signal)); },
  });
}
`;

export interface PiRuntimeOptions {
  resourcesPath: string;
  environment?: NodeJS.ProcessEnv;
  runtimeLauncher?: RuntimeProcessLauncher;
  onTaskChanged(taskId: string, patch: Partial<StoredTask>): void;
  onEvent(taskId: string): void;
  onCollaborationAction(sourceTaskId: string, action: string, params: Record<string, unknown>): Promise<unknown>;
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

const AUTO_CONFIRM_INTENT = /\b(?:permission|allow|approve|authorize|permit|consent|grant|access|execute|execution)\b|权限|允许|批准|授权|许可|同意|执行/i;
const PROTECTED_CONFIRM_TARGET = /(?:^|[/\\])\.(?:codex|pi|chatgpt)(?:[/\\]|$)|auth\.json|credential|api[ _-]?key|access[ _-]?token|secret|openai|Library[/\\]Application Support[/\\](?:Pi|Codex|ChatGPT)/i;

export function shouldAutoConfirmExtensionRequest(message: Record<string, unknown>): boolean {
  const text = JSON.stringify(message);
  return AUTO_CONFIRM_INTENT.test(text) && !PROTECTED_CONFIRM_TARGET.test(text);
}

function childEnvironment(
  profile: StoredProfile,
  source: NodeJS.ProcessEnv,
  fastModeFile: string,
  collaboration: { endpoint: string; token: string; taskId: string },
): NodeJS.ProcessEnv {
  const inheritedPath = source.PATH ?? '/usr/bin:/bin:/usr/sbin:/sbin';
  const environment: NodeJS.ProcessEnv = {
    PATH: inheritedPath,
    PI_CODING_AGENT_DIR: profile.agent_dir,
    PI_CODING_AGENT_SESSION_DIR: profile.session_dir,
    PAD_PI_FAST_MODE_FILE: fastModeFile,
    PAD_COLLABORATION_ENDPOINT: collaboration.endpoint,
    PAD_COLLABORATION_TOKEN: collaboration.token,
    PAD_TASK_ID: collaboration.taskId,
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
  const selected = [
    path.join(resourcesPath, 'bin', 'pi'),
    '/opt/homebrew/bin/pi',
    '/usr/local/bin/pi',
  ].find(existsSync);
  if (!selected) throw new Error('Pi runtime is unavailable');
  return { program: selected, prefix: [] };
}

function piPackagePath(resourcesPath: string): string {
  const selected = [
    path.join(resourcesPath, 'pi'),
    '/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent',
    '/usr/local/lib/node_modules/@earendil-works/pi-coding-agent',
  ].find((candidate) => existsSync(path.join(candidate, 'package.json')));
  if (!selected) throw new Error('Pi runtime package is unavailable');
  return selected;
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
  writeFileSync(extension, PAD_EXTENSION, { encoding: 'utf8', mode: 0o600 });
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
  private readonly child: RuntimeProcess;
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
    collaboration: { endpoint: string; token: string; taskId: string },
  ) {
    super();
    const fastModeFile = taskStatePath(profile, task.id);
    writeFastMode(fastModeFile, launch.fastMode);
    const args = ['--mode', 'rpc', '--extension', writeExtension(profile), '--approve'];
    if (task.session_file) args.push('--session', task.session_file);
    if (launch.provider && launch.model) args.push('--provider', launch.provider, '--model', launch.model);
    if (launch.thinkingLevel && launch.thinkingLevel !== 'default') args.push('--thinking', launch.thinkingLevel);
    const environment = childEnvironment(
      profile,
      options.environment ?? process.env,
      fastModeFile,
      collaboration,
    );
    if (options.runtimeLauncher) {
      this.child = options.runtimeLauncher({
        mode: 'pi-rpc',
        piPackage: piPackagePath(options.resourcesPath),
        args,
        cwd: task.cwd,
        env: environment,
        serviceName: `PAD Pi ${task.id.slice(0, 32)}`,
      });
    } else {
      const runtime = piCommand(options.resourcesPath);
      this.child = nodeRuntimeProcess(spawn(runtime.program, [...runtime.prefix, ...args], {
        cwd: task.cwd,
        stdio: ['pipe', 'pipe', 'pipe'],
        env: environment,
      }));
    }
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
    if (this.closed || !this.child.writable) return Promise.reject(new Error('Pi task is not running'));
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
      try {
        this.child.write(`${JSON.stringify(message)}\n`);
      } catch (error) {
        clearTimeout(timer);
        this.pending.delete(id);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
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
    this.child.write(`${JSON.stringify(response)}\n`);
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
      if (method === 'confirm' && fullAccess && shouldAutoConfirmExtensionRequest(message)) {
        this.child.write(`${JSON.stringify({ type: 'extension_ui_response', id, confirmed: true })}\n`);
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
  private readonly collaborationToken = randomUUID();
  private readonly collaborationServer = createServer((request, response) => {
    void this.handleCollaborationRequest(request, response);
  });
  private readonly collaborationEndpoint: Promise<string>;

  constructor(private readonly options: PiRuntimeOptions) {
    this.collaborationEndpoint = new Promise((resolve, reject) => {
      const onError = (error: Error) => reject(error);
      this.collaborationServer.once('error', onError);
      this.collaborationServer.listen(0, '127.0.0.1', () => {
        this.collaborationServer.off('error', onError);
        const address = this.collaborationServer.address();
        if (!address || typeof address === 'string') {
          reject(new Error('PAD collaboration bridge failed to bind'));
          return;
        }
        resolve(`http://127.0.0.1:${address.port}/collaboration`);
      });
    });
  }

  async start(task: StoredTask, profile: StoredProfile, launch: {
    provider?: string;
    model?: string;
    thinkingLevel?: string;
    fastMode: boolean;
  }): Promise<PiProcess> {
    const existing = this.processes.get(task.id);
    if (existing) return existing;
    const endpoint = await this.collaborationEndpoint;
    const process = new PiProcess(task, profile, this.options, launch, {
      endpoint,
      token: this.collaborationToken,
      taskId: task.id,
    });
    this.processes.set(task.id, process);
    process.once('exit', () => this.processes.delete(task.id));
    this.options.onTaskChanged(task.id, { status: 'starting' });
    try {
      await process.request({ type: 'get_state' });
      return process;
    } catch (error) {
      this.processes.delete(task.id);
      await process.stop().catch(() => undefined);
      throw error;
    }
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
    await this.collaborationEndpoint.catch(() => undefined);
    if (this.collaborationServer.listening) {
      await new Promise<void>((resolve) => this.collaborationServer.close(() => resolve()));
    }
  }

  history(task: StoredTask): unknown[] {
    return readSessionMessages(task.session_file);
  }

  private async handleCollaborationRequest(request: IncomingMessage, response: ServerResponse): Promise<void> {
    const send = (status: number, value: Record<string, unknown>) => {
      response.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' });
      response.end(JSON.stringify(value));
    };
    try {
      if (request.method !== 'POST' || request.url !== '/collaboration') {
        send(404, { ok: false, error: 'Not found' });
        return;
      }
      if (request.headers.authorization !== `Bearer ${this.collaborationToken}`) {
        send(401, { ok: false, error: 'Unauthorized' });
        return;
      }
      const chunks: Buffer[] = [];
      let bytes = 0;
      for await (const chunk of request) {
        const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
        bytes += buffer.length;
        if (bytes > 1024 * 1024) throw new Error('Collaboration request is too large');
        chunks.push(buffer);
      }
      const payload = record(JSON.parse(Buffer.concat(chunks).toString('utf8')));
      const sourceTaskId = typeof payload?.source_task_id === 'string' ? payload.source_task_id : '';
      const action = typeof payload?.action === 'string' ? payload.action : '';
      const params = record(payload?.params) ?? {};
      if (!sourceTaskId || !action) throw new Error('Invalid collaboration request');
      const result = await this.options.onCollaborationAction(sourceTaskId, action, params);
      send(200, { ok: true, result });
    } catch (error) {
      send(400, { ok: false, error: error instanceof Error ? error.message : String(error) });
    }
  }
}
