import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { WebSocket } from 'ws';
import type { DesktopBootstrapResult, TaskDto } from '../../shared/protocol';
import { LocalBackend } from './local-backend';

const roots: string[] = [];

async function nextFrame(socket: WebSocket, type: string): Promise<Record<string, unknown>> {
  for (;;) {
    const frame = await new Promise<Record<string, unknown>>((resolve, reject) => {
      socket.once('message', (data) => resolve(JSON.parse(data.toString()) as Record<string, unknown>));
      socket.once('error', reject);
    });
    if (frame.type === type) return frame;
  }
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe('Electron local backend', () => {
  it('runs the product path directly through Pi without a Rust sidecar', async () => {
    const root = mkdtempSync(path.join(os.tmpdir(), 'pad-electron-local-'));
    roots.push(root);
    const resources = path.join(root, 'resources');
    const bin = path.join(resources, 'bin');
    mkdirSync(bin, { recursive: true });
    const pi = path.join(bin, 'pi');
    writeFileSync(pi, `#!/bin/sh
set -eu
while IFS= read -r line; do
  case "$line" in
    *'"type":"get_state"'*) printf '%s\n' '{"type":"response","command":"get_state","success":true,"data":{"sessionId":"session-1"}}' ;;
    *'"type":"prompt"'*) printf '%s\n' '{"type":"response","command":"prompt","success":true}' ; printf '%s\n' '{"type":"agent_settled"}' ;;
    *'"type":"get_messages"'*) printf '%s\n' '{"type":"response","command":"get_messages","success":true,"data":{"messages":[{"role":"user","content":"你好"},{"role":"assistant","content":"你好呀"}]}}' ;;
    *'"type":"abort"'*) printf '%s\n' '{"type":"response","command":"abort","success":true}' ;;
  esac
done
`);
    chmodSync(pi, 0o700);

    const backend = new LocalBackend({ dataRoot: path.join(root, 'data'), resourcesPath: resources });
    const bootstrap = await backend.request<DesktopBootstrapResult>('bootstrap');
    expect(bootstrap.backend.status).toBe('ready');

    const created = await backend.request<{ task: TaskDto }>('create_task', {
      profile_id: bootstrap.profile.id,
      title: '本地 Pi',
      cwd: root,
    });
    await expect(backend.request('prompt', {
      task_id: created.task.id,
      prompt: '你好',
      fast_mode: true,
    })).resolves.toMatchObject({ accepted: true });
    await expect(backend.request('history', { task_id: created.task.id })).resolves.toMatchObject({
      messages: [{ role: 'user', content: '你好' }, { role: 'assistant', content: '你好呀' }],
    });

    const remote = await backend.request<{ remote: { enabled: boolean; state: string } }>('remote_set_enabled', { enabled: true });
    expect(remote.remote).toMatchObject({ enabled: true, state: 'ready' });
    const pairing = await backend.request<{ pairing: { qr_payload: string } }>('remote_pair_begin');
    expect(pairing.pairing.qr_payload).toMatch(/^pad:\/\/remote\/pair\?v=1&endpoint=wss/);
    const invitation = new URL(pairing.pairing.qr_payload);
    const socket = new WebSocket(invitation.searchParams.get('endpoint')!, 'pad.remote.v1', { rejectUnauthorized: false });
    await new Promise<void>((resolve, reject) => {
      socket.once('open', resolve);
      socket.once('error', reject);
    });
    socket.send(JSON.stringify({
      type: 'pair',
      pairing_id: invitation.searchParams.get('pairing_id'),
      secret: invitation.searchParams.get('secret'),
      device: { display_name: 'Test iPhone', platform: 'ios' },
    }));
    await expect(nextFrame(socket, 'paired')).resolves.toMatchObject({ type: 'paired', profile_available: true });
    socket.send(JSON.stringify({ type: 'command', command_id: 'bootstrap-1', action: 'bootstrap', params: {} }));
    await expect(nextFrame(socket, 'command_result')).resolves.toMatchObject({ type: 'command_result', command_id: 'bootstrap-1', ok: true });
    socket.close();
    await new Promise<void>((resolve) => socket.once('close', () => resolve()));
    await backend.stop();
  });
});
