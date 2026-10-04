import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { PiTaskProcess } from './pi-task.mjs';

test('backpressure is accepted and an expired response cannot settle a newer request', () => {
  const peer = Object.create(PiTaskProcess.prototype);
  let written = false;
  peer.closed = false;
  peer.child = { stdin: { writable: true, write() { written = true; return false; } } };
  assert.equal(peer.write({ type: 'get_state' }), true);
  assert.equal(written, true);
  let settled = false;
  peer.pending = new Map([['new', { command: 'get_state', resolve() { settled = true; } }]]);
  peer.pendingOrder = ['new'];
  peer.resolveResponse({ id: 'expired', command: 'get_state', success: true, data: {} });
  assert.equal(settled, false);
  assert.equal(peer.pending.size, 1);
});

const FAKE_PI = `
import { writeFileSync } from 'node:fs';
const marker = process.env.FAKE_MARKER;
const sessionFile = process.env.FAKE_SESSION_FILE;
let buffer = '';
function send(value) { process.stdout.write(JSON.stringify(value) + '\\n'); }
function handle(message) {
  if (message.type === 'extension_ui_response') {
    if (message.cancelled && marker) writeFileSync(marker, 'cancelled\\n');
    return;
  }
  if (message.type === 'get_state') {
    send({ type: 'response', id: message.id, command: 'get_state', success: true, data: { sessionId: 'sess-1', sessionFile } });
    send({ type: 'extension_ui_request', id: 'ui-1', method: 'select', title: 'Allow?', options: ['Allow', 'Block'] });
    return;
  }
  if (message.type === 'prompt') {
    send({ type: 'response', id: message.id, command: 'prompt', success: true, data: { disposition: 'started' } });
    send({ type: 'message_start', message: { role: 'assistant', content: [] } });
    send({ type: 'agent_settled' });
    return;
  }
  send({ type: 'response', id: message.id, command: message.type, success: true, data: {} });
}
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  buffer += chunk;
  let newline = buffer.indexOf('\\n');
  while (newline >= 0) {
    const line = buffer.slice(0, newline).trim();
    buffer = buffer.slice(newline + 1);
    if (line) handle(JSON.parse(line));
    newline = buffer.indexOf('\\n');
  }
});
`;

async function waitFor(predicate, timeoutMs = 3_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) return true;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  return predicate();
}

test('runs a per-task Pi RPC child with correlation, fail-closed UI, and event passthrough', async () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), 'pad-pi-task-'));
  const marker = path.join(temporary, 'cancelled.marker');
  const sessionFile = path.join(temporary, 'session.jsonl');
  writeFileSync(sessionFile, '');
  const cli = path.join(temporary, 'fake-pi.mjs');
  writeFileSync(cli, FAKE_PI);
  const child = new PiTaskProcess({
    taskId: 'task-1',
    command: process.execPath,
    args: [cli],
    cwd: temporary,
    env: { PATH: process.env.PATH ?? '', FAKE_MARKER: marker, FAKE_SESSION_FILE: sessionFile },
  });
  const events = [];
  const diagnostics = [];
  child.on('pi', (message) => events.push(message));
  child.on('diagnostic', (message) => diagnostics.push(message));
  try {
    const state = await child.request({ type: 'get_state' });
    assert.equal(state.sessionFile, sessionFile);
    assert.ok(await waitFor(() => existsSync(marker)), 'unknown extension UI requests must be cancelled');
    assert.ok(diagnostics.some((line) => line.includes('cancelled extension UI request')));
    const prompt = await child.request({ type: 'prompt', message: 'hello' });
    assert.equal(prompt.disposition, 'started');
    assert.ok(await waitFor(() => events.some((event) => event.type === 'agent_settled')));
    assert.ok(events.some((event) => event.type === 'message_start'));
    assert.equal(events.some((event) => event.type === 'extension_ui_request'), false);
  } finally {
    await child.stop();
    rmSync(temporary, { recursive: true, force: true });
  }
});
