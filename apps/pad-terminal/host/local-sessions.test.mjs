import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { LocalSessions } from './local-sessions.mjs';
import { WorkbenchStore } from './workbench-store.mjs';
import { WorkbenchHost } from './workbench.mjs';
import { sessionMessages, contentText } from './local-session-formats.mjs';

test('history excerpts include shell/custom tools and label long text clipping', () => {
  assert.equal(sessionMessages('pi', [{ type: 'message', message: { role: 'bashExecution', command: 'pwd', output: '/tmp' } }])[0].text, '$ pwd\n/tmp');
  assert.equal(sessionMessages('codex', [{ type: 'response_item', payload: { type: 'custom_tool_call', name: 'apply_patch', input: 'patch text' } }])[0].role, 'tool');
  assert.equal(sessionMessages('claude', [{ type: 'user', message: { role: 'user', content: [{ type: 'tool_result', content: 'done' }] } }])[0].role, 'tool');
  assert.match(contentText('x'.repeat(13_000)), /预览已截断/);
});

const uuid = '12345678-abcd-abcd-abcd-123456789012';
const jsonl = (entries) => entries.map((e) => JSON.stringify(e)).join('\n') + '\n';
test('missing project restoration refreshes canonical cwd without a session-file change', async () => {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), 'pad-restored-project-'));
  try {
    // The alias makes missing and restored cwd spellings differ on every platform.
    const alias = path.join(home, 'alias'); fs.symlinkSync(home, alias);
    const workspace = path.join(alias, 'project');
    const file = path.join(home, '.pi/agent/sessions/project/pi.jsonl');
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, jsonl([{ type: 'session', version: 3, id: uuid, cwd: workspace }]));
    const bin = path.join(home, '.local/bin'); fs.mkdirSync(bin, { recursive: true });
    fs.writeFileSync(path.join(bin, 'pi'), '#!/bin/sh\nexit 99\n', { mode: 0o700 });
    const local = new LocalSessions({ home });
    const first = (await local.list()).sessions[0];
    assert.equal(first.cwd, workspace); // Missing projects remain browsable.
    assert.equal(Object.hasOwn(first, 'rawCwd'), false);
    const fingerprint = local.require(first.id).fingerprint;
    await assert.rejects(local.resume(first.id));
    fs.mkdirSync(workspace);
    const restored = (await local.list()).sessions[0];
    assert.equal(local.require(restored.id).fingerprint, fingerprint);
    assert.equal(restored.cwd, fs.realpathSync(workspace));
    assert.notEqual(restored.cwd, first.cwd);
    assert.equal(Object.hasOwn(restored, 'rawCwd'), false);
    const launch = await local.resume(restored.id); // Prepare only; never execute CLI.
    assert.equal(launch.cwd, fs.realpathSync(workspace));
    assert.deepEqual(launch.args, ['--session', fs.realpathSync(file)]);
  } finally { fs.rmSync(home, { recursive: true, force: true }); }
});

test('local sessions: three formats, live refresh, branch-safe history and isolated Pi fork', async () => {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), 'pad-local-sessions-'));
  let host;
  try {
    const workspace = path.join(home, 'project'); fs.mkdirSync(workspace);
    const piFile = path.join(home, '.pi/agent/sessions/project/pi.jsonl');
    const codexFile = path.join(home, '.codex/sessions/2026/10/rollout.jsonl');
    const claudeFile = path.join(home, '.claude/projects/project', `${uuid}.jsonl`);
    for (const file of [piFile, codexFile, claudeFile]) fs.mkdirSync(path.dirname(file), { recursive: true });
    const original = jsonl([
      { type: 'session', version: 3, id: uuid, cwd: workspace },
      { type: 'message', id: 'one', parentId: null, message: { role: 'user', content: 'Hello 中文' } },
      { type: 'message', id: 'abandoned', parentId: 'one', message: { role: 'assistant', content: 'old branch' } },
      { type: 'message', id: 'current', parentId: 'one', message: { role: 'assistant', content: [{ type: 'text', text: 'active branch' }] } },
    ]);
    fs.writeFileSync(piFile, original);
    fs.writeFileSync(codexFile, jsonl([
      { type: 'session_meta', payload: { id: uuid, cwd: workspace } },
      { type: 'event_msg', payload: { type: 'user_message', message: 'Codex text' } },
      { type: 'response_item', payload: { type: 'message', role: 'user', content: [{ type: 'input_text', text: 'Codex text' }] } },
    ]));
    fs.writeFileSync(claudeFile, jsonl([
      { type: 'user', sessionId: uuid, cwd: workspace, uuid: 'c1', message: { role: 'user', content: 'Claude text' } },
      { type: 'assistant', sessionId: uuid, uuid: 'c2', message: { role: 'assistant', content: [{ type: 'text', text: 'Claude reply' }] } },
    ]));
    // Even a session-shaped file outside the root cannot be traversed via symlink.
    fs.writeFileSync(path.join(home, 'outside.jsonl'), original);
    fs.symlinkSync(path.join(home, 'outside.jsonl'), path.join(path.dirname(piFile), 'linked.jsonl'));
    fs.writeFileSync(path.join(home, '.pi/agent/auth.json'), 'NOT_A_SESSION_DO_NOT_READ');
    const local = new LocalSessions({ home });
    let list = await local.list();
    assert.equal(list.sessions.length, 3);
    const pi = list.sessions.find((s) => s.tool === 'pi');
    const bin = path.join(home, '.local/bin'); fs.mkdirSync(bin, { recursive: true });
    fs.writeFileSync(path.join(bin, 'pi'), '#!/bin/sh\nexit 99\n', { mode: 0o700 });
    const launch = await local.resume(pi.id);
    assert.deepEqual(launch.args, ['--session', fs.realpathSync(piFile)]); // Only argv preparation, never execute it.
    assert.equal(launch.cwd, fs.realpathSync(workspace));
    assert.deepEqual((await local.history(pi.id)).messages.map((m) => m.text), ['Hello 中文', 'active branch']);
    const codex = list.sessions.find((s) => s.tool === 'codex');
    assert.equal((await local.history(codex.id)).messages.length, 1);
    fs.appendFileSync(claudeFile, jsonl([{ type: 'user', uuid: 'c3', sessionId: uuid, message: { role: 'user', content: 'updated' } }]));
    list = await local.list();
    assert.equal((await local.history(list.sessions.find((s) => s.tool === 'claude').id)).messages.at(-1).text, 'updated');

    const root = path.join(home, 'pad');
    const store = new WorkbenchStore({ root }); store.open();
    host = new WorkbenchHost({ root, store, emit() {}, diagnostic() {} });
    host.localSessions = local;
    const sessions = path.join(root, 'private-sessions'); fs.mkdirSync(sessions);
    host.profileDirs = () => ({ sessions });
    const result = await host.execute('local_session_import_pi', { sessionId: pi.id, profileId: store.state.profiles[0].id });
    const copied = fs.readFileSync(result.task.sessionFile, 'utf8');
    assert.notEqual(JSON.parse(copied.split('\n')[0]).id, uuid);
    assert.ok(copied.includes('old branch')); // Full tree preserved, not just the displayed branch.
    assert.equal(result.task.provider, undefined); // No silent provider/model substitution.
    assert.equal(fs.readFileSync(piFile, 'utf8'), original);
    assert.equal(fs.statSync(result.task.sessionFile).mode & 0o777, 0o600);
    await assert.rejects(host.execute('local_session_import_pi', { sessionId: codex.id, profileId: store.state.profiles[0].id }), /只有 Pi/);
    fs.unlinkSync(piFile); fs.symlinkSync(path.join(home, 'outside.jsonl'), piFile);
    await assert.rejects(local.history(pi.id), /路径已改变/);
  } finally { await host?.close(); fs.rmSync(home, { recursive: true, force: true }); }
});
