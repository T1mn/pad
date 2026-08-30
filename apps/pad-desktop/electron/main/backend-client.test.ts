import { PassThrough } from 'node:stream';
import { EventEmitter } from 'node:events';
import type { ChildProcessWithoutNullStreams } from 'node:child_process';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { hostEnvironment, PadBackendClient } from './backend-client';
import type { DesktopHostRequest } from '../../shared/protocol';

class FakeBackendChild extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  readonly killSignals: NodeJS.Signals[] = [];
  readonly requests: DesktopHostRequest[] = [];
  exitCode: number | null = null;
  signalCode: NodeJS.Signals | null = null;
  onKill?: (signal: NodeJS.Signals) => void;
  onRequest?: (request: DesktopHostRequest) => void;

  constructor() {
    super();
    let input = '';
    this.stdin.on('data', (chunk: Buffer) => {
      input += chunk.toString('utf8');
      let newline = input.indexOf('\n');
      while (newline >= 0) {
        const frame = input.slice(0, newline);
        input = input.slice(newline + 1);
        if (frame.length > 0) {
          const request = JSON.parse(frame) as DesktopHostRequest;
          this.requests.push(request);
          this.onRequest?.(request);
        }
        newline = input.indexOf('\n');
      }
    });
  }

  kill(signal: NodeJS.Signals = 'SIGTERM'): boolean {
    if (this.exitCode !== null || this.signalCode !== null) return false;
    this.killSignals.push(signal);
    this.onKill?.(signal);
    return true;
  }

  respondOk(request: DesktopHostRequest): void {
    this.stdout.write(`${JSON.stringify({ id: request.id, ok: true, result: {} })}\n`);
  }

  exit(code: number | null, signal: NodeJS.Signals | null): void {
    if (this.exitCode !== null || this.signalCode !== null) return;
    this.exitCode = code;
    this.signalCode = signal;
    this.emit('exit', code, signal);
  }

  asChildProcess(): ChildProcessWithoutNullStreams {
    return this as unknown as ChildProcessWithoutNullStreams;
  }
}

function makeClient(child: FakeBackendChild): PadBackendClient {
  return new PadBackendClient({
    spawnBackend: () => child.asChildProcess(),
    shutdownResponseWaitMs: 10,
    naturalExitGraceMs: 50,
    sigtermExitGraceMs: 40,
  });
}

async function flushPromises(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe('PadBackendClient sidecar shutdown', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('allows a sidecar that acknowledged shutdown to exit naturally without a signal', async () => {
    const child = new FakeBackendChild();
    const client = makeClient(child);
    child.onRequest = (request) => {
      if (request.action !== 'shutdown') return;
      child.respondOk(request);
      setTimeout(() => child.exit(0, null), 25);
    };
    client.start();

    const stopping = client.stop();
    await flushPromises();
    await vi.advanceTimersByTimeAsync(25);
    await stopping;

    expect(child.killSignals).toEqual([]);
    expect(child.requests.map((request) => request.action)).toEqual(['shutdown']);
  });

  it('escalates from SIGTERM to SIGKILL only after both grace periods and waits for exit', async () => {
    const child = new FakeBackendChild();
    const client = makeClient(child);
    child.onRequest = (request) => {
      if (request.action === 'shutdown') child.respondOk(request);
    };
    child.onKill = (signal) => {
      if (signal === 'SIGKILL') setTimeout(() => child.exit(null, 'SIGKILL'), 5);
    };
    client.start();

    const stopping = client.stop();
    await flushPromises();
    await vi.advanceTimersByTimeAsync(49);
    expect(child.killSignals).toEqual([]);

    await vi.advanceTimersByTimeAsync(1);
    expect(child.killSignals).toEqual(['SIGTERM']);
    await vi.advanceTimersByTimeAsync(39);
    expect(child.killSignals).toEqual(['SIGTERM']);

    await vi.advanceTimersByTimeAsync(1);
    expect(child.killSignals).toEqual(['SIGTERM', 'SIGKILL']);
    await vi.advanceTimersByTimeAsync(5);
    await stopping;
  });

  it('coalesces concurrent stop calls into one shutdown lifecycle', async () => {
    const child = new FakeBackendChild();
    const client = makeClient(child);
    child.onRequest = (request) => {
      if (request.action !== 'shutdown') return;
      child.respondOk(request);
      setTimeout(() => child.exit(0, null), 20);
    };
    client.start();

    const first = client.stop();
    const second = client.stop();
    expect(second).toBe(first);

    await flushPromises();
    await vi.advanceTimersByTimeAsync(20);
    await Promise.all([first, second]);
    expect(child.requests.filter((request) => request.action === 'shutdown')).toHaveLength(1);
    expect(child.killSignals).toEqual([]);
  });

  it('rejects pending work and reports stopped after a forced child exit', async () => {
    const child = new FakeBackendChild();
    const client = makeClient(child);
    const statuses: string[] = [];
    client.on('event', (event) => {
      if (event.type === 'host_status') statuses.push(event.status);
    });
    child.onRequest = (request) => {
      if (request.action === 'shutdown') child.respondOk(request);
    };
    child.onKill = (signal) => {
      if (signal === 'SIGKILL') setTimeout(() => child.exit(null, 'SIGKILL'), 1);
    };

    const pending = client.request('hello');
    const pendingRejection = expect(pending).rejects.toThrow(/exited.*SIGKILL/);
    const stopping = client.stop();
    await flushPromises();
    await vi.advanceTimersByTimeAsync(91);

    await Promise.all([pendingRejection, stopping]);
    expect(statuses).toEqual(['starting', 'stopped']);
  });
});

describe('PAD Desktop sidecar environment boundary', () => {
  it('maps custom CODEX_HOME to a protection-only variable and drops credentials and PAD_HOME', () => {
    const environment = hostEnvironment({
      HOME: '/Users/example',
      PATH: '/custom/bin',
      CODEX_HOME: '/Volumes/Private/Codex State',
      PAD_HOME: '/Users/example/.codex',
      PAD_DESKTOP_DATA_DIR: '/Users/example/Library/Application Support/PAD Desktop Test',
      OPENAI_API_KEY: 'must-not-leak',
      HTTPS_PROXY: 'http://127.0.0.1:7890',
      NO_PROXY: 'localhost,127.0.0.1',
    });

    expect(environment.HOME).toBe('/Users/example');
    expect(environment.PAD_DESKTOP_DATA_DIR).toBe('/Users/example/Library/Application Support/PAD Desktop Test');
    expect(environment.PAD_PROTECTED_CODEX_HOME).toBe('/Volumes/Private/Codex State');
    expect(environment.CODEX_HOME).toBeUndefined();
    expect(environment.PAD_HOME).toBeUndefined();
    expect(environment.OPENAI_API_KEY).toBeUndefined();
    expect(environment.HTTPS_PROXY).toBe('http://127.0.0.1:7890');
    expect(environment.NO_PROXY).toBe('localhost,127.0.0.1');
  });
});
