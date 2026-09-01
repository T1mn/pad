import { EventEmitter } from 'node:events';
import { existsSync } from 'node:fs';
import process from 'node:process';
import { utilityProcess, type UtilityProcess } from 'electron';
import type {
  RuntimeProcess,
  RuntimeProcessLauncher,
  RuntimeProcessRequest,
} from './runtime-process';

class ElectronRuntimeProcess extends EventEmitter implements RuntimeProcess {
  readonly stdout;
  readonly stderr;
  private spawned = false;
  private closed = false;
  private stdoutEnded = false;
  private stderrEnded = false;
  private exitCode: number | null | undefined;
  private readonly queuedInput: string[] = [];

  constructor(private readonly child: UtilityProcess) {
    super();
    if (!child.stdout || !child.stderr) {
      child.kill();
      throw new Error('Electron utility process did not expose diagnostic pipes');
    }
    this.stdout = child.stdout;
    this.stderr = child.stderr;
    this.stdout.once('end', () => {
      this.stdoutEnded = true;
      this.emitExitAfterPipesClose();
    });
    this.stderr.once('end', () => {
      this.stderrEnded = true;
      this.emitExitAfterPipesClose();
    });
    this.spawned = child.pid !== undefined;
    child.once('spawn', () => {
      this.spawned = true;
      for (const value of this.queuedInput.splice(0)) this.post(value);
    });
    child.once('error', (type, location) => {
      this.emit('error', new Error(`Electron utility process ${type} at ${location}`));
    });
    child.once('exit', (code) => {
      this.closed = true;
      this.exitCode = code;
      this.emitExitAfterPipesClose();
      setTimeout(() => this.emitExitAfterPipesClose(true), 100).unref();
    });
  }

  get writable(): boolean {
    return !this.closed;
  }

  write(value: string): void {
    if (this.closed) throw new Error('Electron utility process is not running');
    if (!this.spawned) {
      this.queuedInput.push(value);
      return;
    }
    this.post(value);
  }

  kill(signal: NodeJS.Signals = 'SIGTERM'): void {
    if (this.closed) return;
    if (signal === 'SIGKILL' && this.child.pid !== undefined) {
      try { process.kill(this.child.pid, 'SIGKILL'); } catch { /* process already exited */ }
      return;
    }
    this.child.kill();
  }

  private post(value: string): void {
    this.child.postMessage({ type: 'stdin', value });
  }

  private emitExitAfterPipesClose(force = false): void {
    if (
      this.exitCode === undefined
      || (!force && (!this.stdoutEnded || !this.stderrEnded))
    ) return;
    const code = this.exitCode;
    this.exitCode = undefined;
    this.emit('exit', code);
  }
}

export function createElectronRuntimeLauncher(hostPath: string): RuntimeProcessLauncher {
  if (!existsSync(hostPath)) throw new Error(`PAD utility host is missing: ${hostPath}`);
  return (request: RuntimeProcessRequest) => new ElectronRuntimeProcess(utilityProcess.fork(
    hostPath,
    [request.mode, request.piPackage, ...(request.args ?? [])],
    {
      cwd: request.cwd,
      env: request.env,
      serviceName: request.serviceName,
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  ));
}
