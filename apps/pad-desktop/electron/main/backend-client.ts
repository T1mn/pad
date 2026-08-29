import { EventEmitter } from 'node:events';
import fs from 'node:fs';
import path from 'node:path';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import {
  DESKTOP_PROTOCOL_VERSION,
  DESKTOP_MAX_FRAME_BYTES,
  type DesktopEvent,
  type DesktopHelloResult,
  type DesktopHostRequest,
  type DesktopHostResponse,
} from '../../shared/protocol';

const MAX_STDERR_BYTES = 512 * 1024;
const REQUEST_TIMEOUT_MS = 30_000;
const SHUTDOWN_RESPONSE_WAIT_MS = 750;
const NATURAL_EXIT_GRACE_MS = 1_000;
const SIGTERM_EXIT_GRACE_MS = 1_000;

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timer: NodeJS.Timeout;
}

type BackendChildFactory = () => ChildProcessWithoutNullStreams;

export interface PadBackendClientOptions {
  /** Test seam for deterministic child-process lifecycle coverage. */
  spawnBackend?: BackendChildFactory;
  shutdownResponseWaitMs?: number;
  naturalExitGraceMs?: number;
  sigtermExitGraceMs?: number;
}

export class PadBackendClient extends EventEmitter {
  private child: ChildProcessWithoutNullStreams | null = null;
  private stdoutBuffer = Buffer.alloc(0);
  private stderrBuffer = '';
  private pending = new Map<string, PendingRequest>();
  private requestSequence = 0;
  private stopping = false;
  private stopPromise: Promise<void> | null = null;
  private protocolReady: Promise<void> | null = null;

  constructor(private readonly options: PadBackendClientOptions = {}) {
    super();
  }

  override on(event: 'event', listener: (event: DesktopEvent) => void): this {
    return super.on(event, listener);
  }

  start(): void {
    if (this.child) return;
    this.stopping = false;
    this.emitEvent({ type: 'host_status', status: 'starting' });

    const child = this.options.spawnBackend?.() ?? spawnBackend();
    this.child = child;
    child.stdout.on('data', (chunk: Buffer) => this.consumeStdout(chunk));
    child.stderr.on('data', (chunk: Buffer) => this.consumeStderr(chunk));
    child.once('spawn', () => this.emitEvent({ type: 'host_status', status: 'ready' }));
    child.once('error', (error) => this.failAll(error));
    child.once('exit', (code, signal) => {
      this.child = null;
      this.protocolReady = null;
      const message = `PAD desktop-server exited (code=${String(code)}, signal=${String(signal)})`;
      if (!this.stopping && code !== 0) {
        this.emitEvent({ type: 'host_status', status: 'failed', message });
      } else {
        this.emitEvent({ type: 'host_status', status: 'stopped', message });
      }
      this.failAll(new Error(message));
    });
  }

  async request<T>(action: string, fields: Record<string, unknown> = {}): Promise<T> {
    if (action !== 'hello' && action !== 'shutdown') await this.ensureProtocolV2();
    return this.requestRaw<T>(action, fields);
  }

  private async ensureProtocolV2(): Promise<void> {
    if (!this.protocolReady) {
      this.protocolReady = this.requestRaw<DesktopHelloResult>('hello')
        .then((hello) => {
          if (
            hello.protocol.current !== DESKTOP_PROTOCOL_VERSION ||
            !hello.protocol.supported.includes(DESKTOP_PROTOCOL_VERSION)
          ) {
            throw new Error('PAD desktop-server does not support the required protocol v2');
          }
        })
        .catch((error: unknown) => {
          this.protocolReady = null;
          throw error;
        });
    }
    await this.protocolReady;
  }

  private async requestRaw<T>(action: string, fields: Record<string, unknown> = {}): Promise<T> {
    this.start();
    const child = this.child;
    if (!child?.stdin.writable) {
      throw new Error('PAD desktop-server is unavailable');
    }
    const id = `electron-${process.pid}-${++this.requestSequence}`;
    const message: DesktopHostRequest = {
      id,
      action,
      protocol_version: DESKTOP_PROTOCOL_VERSION,
      ...fields,
    };
    const payload = JSON.stringify(message);
    if (Buffer.byteLength(payload, 'utf8') > DESKTOP_MAX_FRAME_BYTES) {
      throw new Error(`PAD desktop-server request is too large: ${action}`);
    }
    const frame = `${payload}\n`;
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`PAD desktop-server request timed out: ${action}`));
      }, REQUEST_TIMEOUT_MS);
      this.pending.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timer,
      });
      child.stdin.write(frame, (error) => {
        if (!error) return;
        const pending = this.pending.get(id);
        if (!pending) return;
        clearTimeout(pending.timer);
        this.pending.delete(id);
        pending.reject(error);
      });
    });
  }

  stop(): Promise<void> {
    if (this.stopPromise) return this.stopPromise;
    const child = this.child;
    if (!child) return Promise.resolve();
    this.stopping = true;
    const stopPromise = this.stopChild(child).finally(() => {
      if (this.stopPromise === stopPromise) this.stopPromise = null;
    });
    this.stopPromise = stopPromise;
    return stopPromise;
  }

  private async stopChild(child: ChildProcessWithoutNullStreams): Promise<void> {
    // A successful shutdown response means that Rust accepted the request, not
    // that its Drop-based cleanup has completed. Keep the process alive for a
    // bounded natural-exit window before escalating signals.
    const shutdownAttempt = this.requestRaw<unknown>('shutdown').catch(() => undefined);
    const exitedWhileAwaitingResponse = await Promise.race([
      shutdownAttempt.then(() => false),
      this.waitForChildExit(child, this.options.shutdownResponseWaitMs ?? SHUTDOWN_RESPONSE_WAIT_MS),
    ]);
    if (exitedWhileAwaitingResponse || this.hasChildExited(child)) return;

    if (
      await this.waitForChildExit(
        child,
        this.options.naturalExitGraceMs ?? NATURAL_EXIT_GRACE_MS,
      )
    ) {
      return;
    }

    this.killChildIfRunning(child, 'SIGTERM');
    if (
      await this.waitForChildExit(
        child,
        this.options.sigtermExitGraceMs ?? SIGTERM_EXIT_GRACE_MS,
      )
    ) {
      return;
    }

    this.killChildIfRunning(child, 'SIGKILL');
    await this.waitForChildExit(child);
  }

  private hasChildExited(child: ChildProcessWithoutNullStreams): boolean {
    return (
      this.child !== child ||
      child.exitCode !== null ||
      child.signalCode !== null
    );
  }

  private killChildIfRunning(
    child: ChildProcessWithoutNullStreams,
    signal: NodeJS.Signals,
  ): void {
    if (this.hasChildExited(child)) return;
    child.kill(signal);
  }

  private waitForChildExit(
    child: ChildProcessWithoutNullStreams,
    timeoutMs?: number,
  ): Promise<boolean> {
    if (this.hasChildExited(child)) return Promise.resolve(true);
    return new Promise<boolean>((resolve) => {
      let timer: NodeJS.Timeout | undefined;
      let settled = false;
      const finish = (exited: boolean) => {
        if (settled) return;
        settled = true;
        if (timer) clearTimeout(timer);
        child.removeListener('exit', onExit);
        resolve(exited);
      };
      const onExit = () => finish(true);
      child.once('exit', onExit);
      if (this.hasChildExited(child)) {
        finish(true);
        return;
      }
      if (timeoutMs !== undefined) {
        timer = setTimeout(() => finish(this.hasChildExited(child)), timeoutMs);
      }
    });
  }

  private consumeStdout(chunk: Buffer): void {
    this.stdoutBuffer = Buffer.concat([this.stdoutBuffer, chunk]);
    if (this.stdoutBuffer.length > DESKTOP_MAX_FRAME_BYTES && !this.stdoutBuffer.includes(0x0a)) {
      this.failAll(new Error('PAD desktop-server emitted an oversized JSONL frame'));
      this.child?.kill('SIGTERM');
      return;
    }
    let newline = this.stdoutBuffer.indexOf(0x0a);
    while (newline >= 0) {
      const frame = this.stdoutBuffer.subarray(0, newline);
      this.stdoutBuffer = this.stdoutBuffer.subarray(newline + 1);
      if (frame.length > DESKTOP_MAX_FRAME_BYTES) {
        this.failAll(new Error('PAD desktop-server emitted an oversized JSONL frame'));
        this.child?.kill('SIGTERM');
        return;
      }
      if (frame.length > 0) this.consumeFrame(frame);
      newline = this.stdoutBuffer.indexOf(0x0a);
    }
  }

  private consumeFrame(frame: Buffer): void {
    let value: DesktopHostResponse | Record<string, unknown>;
    try {
      value = JSON.parse(frame.toString('utf8')) as DesktopHostResponse | Record<string, unknown>;
    } catch (error) {
      this.emitEvent({
        type: 'host_status',
        status: 'failed',
        message: `Invalid PAD desktop-server JSON: ${String(error)}`,
      });
      return;
    }
    if ('id' in value && typeof value.id === 'string') {
      const pending = this.pending.get(value.id);
      if (!pending) return;
      clearTimeout(pending.timer);
      this.pending.delete(value.id);
      const response = value as DesktopHostResponse;
      if (response.ok) pending.resolve(response.result);
      else pending.reject(new Error(response.error?.message ?? 'PAD desktop-server request failed'));
      return;
    }
    this.emitEvent({ type: 'backend_event', payload: value });
  }

  private consumeStderr(chunk: Buffer): void {
    const remaining = Math.max(0, MAX_STDERR_BYTES - Buffer.byteLength(this.stderrBuffer));
    if (remaining === 0) return;
    this.stderrBuffer += chunk.subarray(0, remaining).toString('utf8');
  }

  private failAll(error: Error): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }

  private emitEvent(event: DesktopEvent): void {
    this.emit('event', event);
  }
}

function resolvePadBinary(): string {
  const bundledPad = process.resourcesPath
    ? path.join(process.resourcesPath, 'pad')
    : undefined;
  const candidates = [
    // A packaged app always prefers its signed, version-matched control plane.
    // PAD_BIN remains a development escape hatch only when no bundle member is
    // present; an inherited shell variable must not replace production code.
    bundledPad,
    process.env.PAD_BIN,
    path.resolve(process.cwd(), '../../rust-tui/target/debug/pad'),
    path.resolve(process.cwd(), '../../rust-tui/target/release/pad'),
    path.resolve(process.cwd(), 'rust-tui/target/debug/pad'),
    path.resolve(process.cwd(), 'rust-tui/target/release/pad'),
  ].filter((candidate): candidate is string => Boolean(candidate));
  const executable = candidates.find((candidate) => {
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      return true;
    } catch {
      return false;
    }
  });
  if (!executable) {
    throw new Error('PAD backend not found. Set PAD_BIN or build rust-tui/target/debug/pad.');
  }
  return executable;
}

function spawnBackend(): ChildProcessWithoutNullStreams {
  return spawn(resolvePadBinary(), ['__internal', 'desktop-server'], {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: hostEnvironment(),
  });
}

export function hostEnvironment(source: NodeJS.ProcessEnv = process.env): NodeJS.ProcessEnv {
  // Electron defines resourcesPath in production. Unit tests and a plain
  // Node-based development harness do not, so keep the fallback scoped to the
  // current checkout instead of constructing a path from `undefined`.
  const resourcesRoot = typeof process.resourcesPath === 'string' && process.resourcesPath.length > 0
    ? process.resourcesPath
    : process.cwd();
  const bundledBin = path.join(resourcesRoot, 'bin');
  const inheritedPath = source.PATH ?? '/usr/bin:/bin:/usr/sbin:/sbin';
  const environment: NodeJS.ProcessEnv = {
    PATH: [bundledBin, '/opt/homebrew/bin', '/usr/local/bin', inheritedPath].join(path.delimiter),
  };
  // Pass only operating-system context and explicit PAD test/development
  // controls. Provider keys and Codex/ChatGPT variables from the launching
  // shell must never bleed into a Desktop profile.
  for (const key of [
    'HOME',
    'USER',
    'LOGNAME',
    'SHELL',
    'TMPDIR',
    'LANG',
    'LC_ALL',
    'LC_CTYPE',
    'TERM',
    'COLORTERM',
    'PAD_DESKTOP_DATA_DIR',
    'RUST_BACKTRACE',
    'RUST_LOG',
  ]) {
    const value = source[key];
    if (value) environment[key] = value;
  }
  const codexHome = source.CODEX_HOME;
  if (codexHome && !codexHome.includes('\0')) {
    // Preserve a custom Codex directory only as a host-owned protection
    // boundary. Never pass CODEX_HOME itself to Rust/Pi as configuration.
    environment.PAD_PROTECTED_CODEX_HOME = path.resolve(codexHome);
  }
  return environment;
}
