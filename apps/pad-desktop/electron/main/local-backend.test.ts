import { createHash } from 'node:crypto';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { WebSocket, type RawData } from 'ws';
import type { DesktopBootstrapResult, TaskDto } from '../../shared/protocol';
import { LocalBackend } from './local-backend';
import { shouldAutoConfirmExtensionRequest } from './pi-runtime';

const roots: string[] = [];

async function nextFrame(socket: WebSocket, type: string): Promise<Record<string, unknown>> {
  return new Promise<Record<string, unknown>>((resolve, reject) => {
    const onMessage = (data: RawData) => {
      const frame = JSON.parse(data.toString()) as Record<string, unknown>;
      if (frame.type !== type) return;
      socket.off('message', onMessage);
      socket.off('error', onError);
      resolve(frame);
    };
    const onError = (error: Error) => {
      socket.off('message', onMessage);
      reject(error);
    };
    socket.on('message', onMessage);
    socket.once('error', onError);
  });
}

function sendAndWait(socket: WebSocket, payload: Record<string, unknown>, type: string): Promise<Record<string, unknown>> {
  const frame = nextFrame(socket, type);
  socket.send(JSON.stringify(payload));
  return frame;
}

async function pairedSocket(backend: LocalBackend): Promise<{ socket: WebSocket; deviceToken: string; deviceId: string }> {
  const remote = await backend.request<{ remote: { enabled: boolean; state: string } }>('remote_set_enabled', { enabled: true });
  expect(remote.remote).toMatchObject({ enabled: true, state: 'ready' });
  const pairing = await backend.request<{ pairing: { qr_payload: string } }>('remote_pair_begin');
  const invitation = new URL(pairing.pairing.qr_payload);
  const socket = new WebSocket(invitation.searchParams.get('endpoint')!, 'pad.remote.v1', { rejectUnauthorized: false });
  socket.setMaxListeners(100);
  await new Promise<void>((resolve, reject) => {
    socket.once('open', resolve);
    socket.once('error', reject);
  });
  const paired = await sendAndWait(socket, {
    type: 'pair',
    pairing_id: invitation.searchParams.get('pairing_id'),
    secret: invitation.searchParams.get('secret'),
    device: { display_name: 'Scoped Test iPhone', platform: 'ios' },
  }, 'paired');
  return {
    socket,
    deviceToken: String(paired.device_token),
    deviceId: String(paired.device_id),
  };
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe('Electron local backend', () => {
  it('auto-confirms only explicit non-sensitive permission prompts in Full Access', () => {
    expect(shouldAutoConfirmExtensionRequest({
      method: 'confirm',
      title: 'Allow command execution?',
      message: 'Grant permission to run tests',
    })).toBe(true);
    expect(shouldAutoConfirmExtensionRequest({
      method: 'confirm',
      title: '是否允许执行命令？',
    })).toBe(true);
    expect(shouldAutoConfirmExtensionRequest({
      method: 'confirm',
      title: '继续下一步？',
    })).toBe(false);
    expect(shouldAutoConfirmExtensionRequest({
      method: 'confirm',
      title: 'Allow access',
      path: '/Users/example/.codex/auth.json',
    })).toBe(false);
  });

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
    expect(readFileSync(path.join(root, 'data', 'v1', 'profiles', 'default', 'pi-agent', 'pad-fast-mode.ts'), 'utf8'))
      .toContain('service_tier: "priority"');
    expect(readFileSync(path.join(root, 'data', 'v1', 'profiles', 'default', 'pi-agent', 'pad-fast-mode.ts'), 'utf8'))
      .toContain('active model id');
    expect(readFileSync(path.join(root, 'data', 'v1', 'profiles', 'default', 'pi-agent', 'pad-fast-mode.ts'), 'utf8'))
      .toContain('name: "spawn_agent"');
    await expect(backend.request('history', { task_id: created.task.id })).resolves.toMatchObject({
      messages: [{ role: 'user', content: '你好' }, { role: 'assistant', content: '你好呀' }],
    });
    await expect(backend.request('rename_session', { task_id: created.task.id, name: '主会话' }))
      .resolves.toMatchObject({ renamed: true, session: { title: '主会话' } });
    const spawned = await backend.request<{ agent: TaskDto }>('spawn_agent', {
      source_task_id: created.task.id,
      task: '检查跨 Session 协作',
      name: '协作 Agent',
    });
    expect(spawned.agent).toMatchObject({ title: '协作 Agent', parent_task_id: created.task.id });
    await expect(backend.request('followup_task', {
      source_task_id: created.task.id,
      task_id: spawned.agent.id,
      message: '继续检查',
    })).resolves.toMatchObject({ accepted: true });
    await expect(backend.request('list_agents', { source_task_id: created.task.id }))
      .resolves.toMatchObject({ root: { title: '主会话' }, agents: [{ id: spawned.agent.id }] });

    const remote = await backend.request<{ remote: { enabled: boolean; state: string } }>('remote_set_enabled', { enabled: true });
    expect(remote.remote).toMatchObject({ enabled: true, state: 'ready' });
    const pairing = await backend.request<{ pairing: { qr_payload: string } }>('remote_pair_begin');
    expect(pairing.pairing.qr_payload).toMatch(/^pad:\/\/remote\/pair\?v=1&endpoint=wss/);
    const invitation = new URL(pairing.pairing.qr_payload);
    const socket = new WebSocket(invitation.searchParams.get('endpoint')!, 'pad.remote.v1', { rejectUnauthorized: false });
    socket.setMaxListeners(100);
    await new Promise<void>((resolve, reject) => {
      socket.once('open', resolve);
      socket.once('error', reject);
    });
    await expect(sendAndWait(socket, {
      type: 'pair',
      pairing_id: invitation.searchParams.get('pairing_id'),
      secret: invitation.searchParams.get('secret'),
      device: { display_name: 'Test iPhone', platform: 'ios' },
    }, 'paired')).resolves.toMatchObject({ type: 'paired', profile_available: true });
    const status = await backend.request('remote_status');
    expect(JSON.stringify(status)).not.toContain('profile_id');
    expect(JSON.stringify(status)).not.toContain('token_hash');
    await expect(sendAndWait(socket, { type: 'command', command_id: 'bootstrap-1', action: 'bootstrap', params: {} }, 'command_result'))
      .resolves.toMatchObject({ type: 'command_result', command_id: 'bootstrap-1', ok: true });
    socket.close();
    await new Promise<void>((resolve) => socket.once('close', () => resolve()));
    await backend.stop();
  });

  it('keeps paired commands on their profile and deduplicates command receipts', async () => {
    const root = mkdtempSync(path.join(os.tmpdir(), 'pad-electron-remote-scope-'));
    roots.push(root);
    const resources = path.join(root, 'resources');
    const bin = path.join(resources, 'bin');
    mkdirSync(bin, { recursive: true });
    const promptLog = path.join(root, 'prompt.log');
    const pi = path.join(bin, 'pi');
    writeFileSync(pi, `#!/bin/sh
set -eu
while IFS= read -r line; do
  case "$line" in
    *'"type":"get_state"'*) printf '%s\\n' '{"type":"response","command":"get_state","success":true,"data":{"sessionId":"session-remote"}}' ;;
    *'"type":"prompt"'*) printf 'prompt\\n' >> '${promptLog}'; printf '%s\\n' '{"type":"response","command":"prompt","success":true}' ; printf '%s\\n' '{"type":"agent_settled"}' ;;
  esac
done
`);
    chmodSync(pi, 0o700);
    const backend = new LocalBackend({ dataRoot: path.join(root, 'data'), resourcesPath: resources });
    const initial = await backend.request<DesktopBootstrapResult>('bootstrap');
    const taskA = await backend.request<{ task: TaskDto }>('create_task', { profile_id: initial.profile.id, title: 'Profile A task', cwd: root });
    const { socket, deviceId } = await pairedSocket(backend);
    const profileB = await backend.request<{ profile: { id: string } }>('create_profile', { name: 'Profile B' });
    await backend.request('set_ui_state', { state: { ...initial.ui_state, active_profile_id: profileB.profile.id, selected_task_id: null } });
    const command = (commandId: string, action: string, params: Record<string, unknown>) => {
      return sendAndWait(socket, { type: 'command', command_id: commandId, action, params }, 'command_result');
    };
    await expect(command('scope-bootstrap', 'bootstrap', {})).resolves.toMatchObject({
      ok: true, result: { profile: { id: initial.profile.id }, records: { profiles: [{ id: initial.profile.id }] } },
    });
    const taskB = await backend.request<{ task: TaskDto }>('create_task', { profile_id: profileB.profile.id, title: 'Profile B task', cwd: root });
    await expect(command('cross-profile-task', 'prompt', { task_id: taskB.task.id, prompt: 'must reject' })).resolves.toMatchObject({ ok: false, error: { code: 'command_failed' } });
    await expect(command('cross-profile-create', 'create_task', { profile_id: profileB.profile.id, title: 'must reject', cwd: root })).resolves.toMatchObject({ ok: false, error: { code: 'command_failed' } });
    const scopedUpdate = await command('scoped-update', 'set_task', { task_id: taskA.task.id, pinned: true });
    expect(scopedUpdate).toMatchObject({
      ok: true,
      result: {
        records: {
          profiles: [expect.objectContaining({ id: initial.profile.id })],
          tasks: expect.arrayContaining([expect.objectContaining({ id: taskA.task.id })]),
        },
      },
    });
    expect(JSON.stringify(scopedUpdate)).not.toContain(taskB.task.id);
    expect(JSON.stringify(scopedUpdate)).not.toContain(profileB.profile.id);
    const firstPrompt = await command('same-prompt', 'prompt', { task_id: taskA.task.id, prompt: 'once' });
    const replayPrompt = await command('same-prompt', 'prompt', { task_id: taskA.task.id, prompt: 'twice' });
    expect(replayPrompt).toEqual(firstPrompt);
    expect(readFileSync(promptLog, 'utf8')).toBe('prompt\n');
    const firstCreate = await command('same-create', 'create_task', { title: 'first', cwd: root });
    const replayCreate = await command('same-create', 'create_task', { title: 'second', cwd: root });
    expect(replayCreate).toEqual(firstCreate);
    const createdTaskId = (firstCreate.result as { task: TaskDto }).task.id;
    expect((await backend.request<{ records: { tasks: TaskDto[] } }>('list_sidebar')).records.tasks.some((task) => task.id === createdTaskId)).toBe(true);
    const state = JSON.parse(readFileSync(path.join(root, 'data', 'v1', 'remote', 'state.json'), 'utf8')) as { devices: Array<{ id: string; receipts?: Array<{ command_id: string }> }> };
    expect(state.devices.find((device) => device.id === deviceId)?.receipts?.map((receipt) => receipt.command_id)).toEqual(expect.arrayContaining(['same-prompt', 'same-create']));
    socket.close();
    await new Promise<void>((resolve) => socket.once('close', () => resolve()));
    await backend.stop();
  }, 15_000);

  it('marks legacy devices without profile_id unavailable over a real websocket', async () => {
    const root = mkdtempSync(path.join(os.tmpdir(), 'pad-electron-remote-legacy-'));
    roots.push(root);
    const dataRoot = path.join(root, 'data');
    const remoteDir = path.join(dataRoot, 'v1', 'remote');
    mkdirSync(remoteDir, { recursive: true });
    const deviceToken = 'legacy-device-token';
    writeFileSync(path.join(remoteDir, 'state.json'), JSON.stringify({
      enabled: false,
      devices: [{
        id: 'legacy-device', display_name: 'Legacy', platform: 'ios', online: false,
        paired_at: Date.now(), last_seen_at: Date.now(),
        token_hash: createHash('sha256').update(deviceToken).digest('hex'),
      }],
    }));
    const resources = path.join(root, 'resources');
    mkdirSync(path.join(resources, 'bin'), { recursive: true });
    const backend = new LocalBackend({ dataRoot, resourcesPath: resources });
    await backend.request<DesktopBootstrapResult>('bootstrap');
    const remote = await backend.request<{ remote: { state: string } }>('remote_set_enabled', { enabled: true });
    expect(remote.remote.state).toBe('ready');
    const pairing = await backend.request<{ pairing: { qr_payload: string } }>('remote_pair_begin');
    const invitation = new URL(pairing.pairing.qr_payload);
    const socket = new WebSocket(invitation.searchParams.get('endpoint')!, 'pad.remote.v1', { rejectUnauthorized: false });
    socket.setMaxListeners(100);
    await new Promise<void>((resolve, reject) => {
      socket.once('open', resolve);
      socket.once('error', reject);
    });
    await expect(sendAndWait(socket, { type: 'resume', device_id: 'legacy-device', device_token: deviceToken }, 'welcome'))
      .resolves.toMatchObject({ profile_available: false });
    await expect(sendAndWait(socket, { type: 'command', command_id: 'legacy-bootstrap', action: 'bootstrap', params: {} }, 'command_result'))
      .resolves.toMatchObject({ ok: false, error: { code: 'profile_unavailable' } });
    socket.close();
    await new Promise<void>((resolve) => socket.once('close', () => resolve()));
    await backend.stop();
  }, 15_000);
});
