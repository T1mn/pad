// Per-task Pi RPC child: spawn, LF-only framing, request correlation,
// event passthrough, fail-closed extension UI, and lifecycle dedup.
import { spawn } from 'node:child_process';
import { EventEmitter } from 'node:events';
import { randomUUID } from 'node:crypto';
import { createFrameDecoder, isRecord, parseJson, writeFrame } from './jsonl.mjs';

const DEFAULT_REQUEST_TIMEOUT_MS = 15_000;

function errorMessage(value) {
  if (typeof value === 'string' && value.length > 0) return value;
  if (value instanceof Error) return value.message;
  return 'Pi command failed';
}

/** One live Pi RPC process for one task. */
export class PiTaskProcess extends EventEmitter {
  constructor({ taskId, command, args, cwd, env }) {
    super();
    this.taskId = taskId;
    this.state = null;
    this.closed = false;
    this.currentProvider = undefined;
    this.currentModel = undefined;
    this.pending = new Map();
    this.pendingOrder = [];
    this.diagnosticTail = [];
    this.decoder = createFrameDecoder({
      onFrame: (text) => this.consumeFrame(text),
      onOversize: (bytes) => this.emit('diagnostic', `dropped an oversized Pi frame (${bytes} bytes)`),
    });
    this.child = spawn(command, args, { cwd, env, stdio: ['pipe', 'pipe', 'pipe'] });
    this.child.stdout?.on('data', (chunk) => this.decoder.feed(chunk));
    this.child.stderr?.on('data', (chunk) => this.consumeStderr(chunk));
    this.child.stdin?.on('error', (error) => this.fail(error));
    this.child.once('error', (error) => this.fail(error));
    this.child.once('exit', (code, signal) => this.finish(code, signal));
  }

  get writable() {
    return !this.closed && this.child.stdin?.writable === true;
  }

  consumeStderr(chunk) {
    const text = Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk);
    for (const line of text.split('\n')) {
      if (line.trim().length === 0) continue;
      this.diagnosticTail.push(line.slice(0, 500));
      if (this.diagnosticTail.length > 20) this.diagnosticTail.shift();
    }
  }

  consumeFrame(text) {
    const parsed = parseJson(text);
    if (!parsed.ok || !isRecord(parsed.value)) return; // stderr is diagnostics; a bad frame never stops later frames.
    this.handleMessage(parsed.value);
  }

  handleMessage(message) {
    if (message.type === 'response') {
      this.resolveResponse(message);
      return;
    }
    if (message.type === 'extension_ui_request') {
      // Fail closed: unknown or unattended extension interactions are cancelled,
      // never silently approved. The protocol stream never sees this record.
      const id = typeof message.id === 'string' && message.id.length > 0 ? message.id : randomUUID();
      const method = typeof message.method === 'string' ? message.method : 'unknown';
      this.emit('diagnostic', `cancelled extension UI request (${method})`);
      this.write({ type: 'extension_ui_response', id, cancelled: true });
      return;
    }
    this.emit('pi', message);
  }

  resolveResponse(message) {
    const responseId = typeof message.id === 'string' ? message.id : undefined;
    let pendingId = responseId !== undefined && this.pending.has(responseId) ? responseId : undefined;
    if (pendingId === undefined && responseId === undefined) {
      const command = typeof message.command === 'string' ? message.command : '';
      pendingId = this.pendingOrder.find((id) => this.pending.get(id)?.command === command);
    }
    if (pendingId === undefined) return;
    const pending = this.pending.get(pendingId);
    this.pending.delete(pendingId);
    const index = this.pendingOrder.indexOf(pendingId);
    if (index >= 0) this.pendingOrder.splice(index, 1);
    clearTimeout(pending.timer);
    if (message.success === false) pending.reject(new Error(errorMessage(message.error)));
    else pending.resolve(isRecord(message.data) ? message.data : {});
  }

  write(value) {
    if (!this.writable) return false;
    try {
      // false is stream backpressure, not rejection: the frame is already queued.
      writeFrame(this.child.stdin, value);
      return true;
    } catch (error) {
      this.emit('diagnostic', `failed to write to Pi: ${error instanceof Error ? error.message : String(error)}`);
      return false;
    }
  }

  request(command, timeoutMs = DEFAULT_REQUEST_TIMEOUT_MS) {
    if (!this.writable) return Promise.reject(new Error('Pi task is not running'));
    const id = typeof command.id === 'string' && command.id.length > 0 ? command.id : `pad-${randomUUID()}`;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        const index = this.pendingOrder.indexOf(id);
        if (index >= 0) this.pendingOrder.splice(index, 1);
        reject(new Error(`Pi ${String(command.type ?? 'command')} timed out`));
      }, timeoutMs);
      timer.unref?.();
      this.pending.set(id, { command: String(command.type ?? ''), resolve, reject, timer });
      this.pendingOrder.push(id);
      if (!this.write({ ...command, id })) {
        clearTimeout(timer);
        this.pending.delete(id);
        const index = this.pendingOrder.indexOf(id);
        if (index >= 0) this.pendingOrder.splice(index, 1);
        reject(new Error('Pi task is not running'));
      }
    });
  }

  rejectPending(error) {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
    this.pendingOrder.length = 0;
  }

  fail(error) {
    if (this.closed) return;
    this.closed = true;
    this.child.kill('SIGTERM');
    this.rejectPending(error);
    this.emit('exit', { code: null, signal: null, error });
  }

  finish(code, signal) {
    if (this.closed) return;
    this.closed = true;
    this.rejectPending(new Error(`Pi exited (code ${String(code)}, signal ${String(signal)})`));
    this.emit('exit', { code, signal, error: null });
  }

  async stop() {
    if (this.closed) return;
    try {
      await this.request({ type: 'clear_queue' }, 2_000);
    } catch {
      // The process may already be idle or gone.
    }
    try {
      await this.request({ type: 'abort' }, 3_000);
    } catch {
      // Abort is best effort during teardown.
    }
    this.closed = true;
    this.rejectPending(new Error('Pi task was stopped'));
    this.child.kill('SIGTERM');
    const exited = await Promise.race([
      new Promise((resolve) => this.child.once('exit', () => resolve(true))),
      new Promise((resolve) => setTimeout(() => resolve(false), 1_500)),
    ]);
    if (!exited) {
      this.child.kill('SIGKILL');
      await new Promise((resolve) => this.child.once('exit', resolve));
    }
  }
}

/** Owns the per-task process map and de-duplicates concurrent starts. */
export class PiTaskRuntime {
  constructor({ createLaunch, onPiEvent, onExit, onDiagnostic }) {
    this.createLaunch = createLaunch;
    this.onPiEvent = onPiEvent;
    this.onExit = onExit;
    this.onDiagnostic = onDiagnostic;
    this.processes = new Map();
    this.starting = new Map();
    this.stopping = new Map();
    this.shuttingDown = false;
  }

  get(taskId) {
    const process = this.processes.get(taskId);
    return process && !process.closed ? process : null;
  }

  async ensure(task) {
    if (this.shuttingDown) throw new Error('Pi runtime is shutting down');
    const stopping = this.stopping.get(task.id);
    if (stopping) {
      await stopping;
      return this.ensure(task);
    }
    const existing = this.get(task.id);
    if (existing) return existing;
    const starting = this.starting.get(task.id);
    if (starting) return starting;
    const pending = this.launch(task);
    this.starting.set(task.id, pending);
    try {
      return await pending;
    } finally {
      if (this.starting.get(task.id) === pending) this.starting.delete(task.id);
    }
  }

  async launch(task) {
    const spec = await this.createLaunch(task);
    const process = new PiTaskProcess({
      taskId: task.id,
      command: spec.command,
      args: spec.args,
      cwd: spec.cwd,
      env: spec.env,
    });
    process.currentProvider = task.provider;
    process.currentModel = task.modelId;
    this.processes.set(task.id, process);
    process.on('pi', (message) => this.onPiEvent(task.id, message, process));
    process.on('diagnostic', (message) => this.onDiagnostic(task.id, message));
    process.on('exit', (info) => {
      if (this.processes.get(task.id) === process) this.processes.delete(task.id);
      this.onExit(task.id, info, process);
    });
    try {
      process.state = await process.request({ type: 'get_state' });
      return process;
    } catch (error) {
      if (this.processes.get(task.id) === process) this.processes.delete(task.id);
      await process.stop().catch(() => undefined);
      throw error;
    }
  }

  async stop(taskId) {
    const existing = this.stopping.get(taskId);
    if (existing) return existing;
    const pending = (async () => {
      await this.starting.get(taskId)?.catch(() => undefined);
      const process = this.processes.get(taskId);
      if (!process) return;
      await process.stop();
      if (this.processes.get(taskId) === process) this.processes.delete(taskId);
    })();
    this.stopping.set(taskId, pending);
    try {
      await pending;
    } finally {
      if (this.stopping.get(taskId) === pending) this.stopping.delete(taskId);
    }
  }

  async stopAll() {
    this.shuttingDown = true;
    const taskIds = new Set([...this.processes.keys(), ...this.starting.keys(), ...this.stopping.keys()]);
    await Promise.all([...taskIds].map((taskId) => this.stop(taskId)));
  }
}
