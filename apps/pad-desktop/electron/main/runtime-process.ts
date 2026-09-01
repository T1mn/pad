import type { ChildProcessWithoutNullStreams } from 'node:child_process';

export type RuntimeProcessMode = 'pi-rpc' | 'model-catalog' | 'auth';

export interface RuntimeProcessRequest {
  mode: RuntimeProcessMode;
  piPackage: string;
  args?: string[];
  cwd: string;
  env: NodeJS.ProcessEnv;
  serviceName: string;
}

export interface RuntimeProcess {
  readonly stdout: NodeJS.ReadableStream;
  readonly stderr: NodeJS.ReadableStream;
  readonly writable: boolean;
  write(value: string): void;
  kill(signal?: NodeJS.Signals): void;
  once(event: 'error', listener: (error: Error) => void): this;
  once(event: 'exit', listener: (code: number | null) => void): this;
}

export type RuntimeProcessLauncher = (request: RuntimeProcessRequest) => RuntimeProcess;

export function nodeRuntimeProcess(child: ChildProcessWithoutNullStreams): RuntimeProcess {
  return {
    stdout: child.stdout,
    stderr: child.stderr,
    get writable() { return child.stdin.writable && !child.killed; },
    write: (value) => { child.stdin.write(value); },
    kill: (signal = 'SIGTERM') => { child.kill(signal); },
    once(event: 'error' | 'exit', listener: ((value: Error) => void) | ((value: number | null) => void)) {
      child.once(event === 'exit' ? 'close' : event, listener as never);
      return this;
    },
  };
}
