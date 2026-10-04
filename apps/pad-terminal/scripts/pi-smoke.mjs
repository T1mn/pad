#!/usr/bin/env node
// One offline RPC path; deliberately no prompt/login/model request.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { launchSpec } from './pi-session.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
mkdirSync(path.join(root, '.cache'), { recursive: true });
const temporary = mkdtempSync(path.join(root, '.cache/pi-smoke-'));
const spec = launchSpec({ profile: 'smoke', dataRoot: temporary, cwd: temporary });
// RpcClient 1.0.2 merges the parent's environment. Spawn explicitly here so
// this smoke, like the TUI launcher, cannot inherit model keys or NODE_OPTIONS.
const child = spawn(process.execPath, [spec.cli, ...spec.args, '--mode', 'rpc', '--no-context-files'], {
  cwd: spec.cwd, env: spec.env, stdio: ['pipe', 'pipe', 'pipe'],
});
let stderr = '';
child.stderr.setEncoding('utf8').on('data', (chunk) => { stderr = (stderr + chunk).slice(-2000); });
child.stdin.on('error', () => {}); // The exit/deadline check reports broken pipes.
const exited = new Promise((resolve, reject) => {
  child.once('error', reject);
  child.once('close', (code) => resolve(code));
});
exited.catch(() => {});
const deadline = setTimeout(() => child.kill('SIGKILL'), 20_000);

async function* records() {
  let buffer = '';
  child.stdout.setEncoding('utf8');
  for await (const chunk of child.stdout) {
    buffer += chunk;
    if (buffer.length > 1024 * 1024) throw new Error('Unexpectedly large smoke response');
    let newline;
    while ((newline = buffer.indexOf('\n')) !== -1) {
      const line = buffer.slice(0, newline).replace(/\r$/, '');
      buffer = buffer.slice(newline + 1);
      if (line) yield JSON.parse(line);
    }
  }
}
const stream = records();
let id = 0;
async function request(type, fields = {}) {
  const requestId = String(++id);
  child.stdin.write(JSON.stringify({ id: requestId, type, ...fields }) + '\n');
  while (true) {
    const { value, done } = await stream.next();
    if (done) throw new Error(`Pi exited before ${type}: ${stderr}`);
    if (value.type !== 'response' || value.id !== requestId) continue;
    assert.equal(value.success, true, value.error);
    return value.data;
  }
}

try {
  const state = await request('get_state');
  assert.ok(state.sessionId);
  assert.equal(state.isStreaming, false);
  assert.ok(state.sessionFile.startsWith(spec.sessions + path.sep));
  const shell = await request('bash', { command: "printf 'PAD_PI_RPC_OK\\n'" });
  assert.equal(shell.exitCode, 0);
  assert.equal(shell.output.trim(), 'PAD_PI_RPC_OK');
  const stats = await request('get_session_stats');
  assert.equal(stats.cost, 0);
  assert.equal(stats.tokens.total, 0);
  const result = { pi: spec.version, sessionId: state.sessionId, sessionFile: state.sessionFile, shell: shell.output.trim(), modelRequests: 0, tokens: stats.tokens.total, cost: stats.cost };
  writeFileSync(path.join(temporary, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify(result, null, 2));
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  child.stdin.end();
  try { assert.equal(await exited, 0, 'Pi did not shut down cleanly after stdin EOF'); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
  clearTimeout(deadline);
}
