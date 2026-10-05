import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { WorkbenchHost } from './workbench.mjs';

test('session_info reads real Pi identity without startup, writes, or unsafe content', async () => {
  const root = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'pad-session-info-'));
  try {
    const sessions = path.join(root, 'v1', 'profiles', 'default', 'pi-sessions');
    fs.mkdirSync(sessions, { recursive: true });
    const task = { id: 'task-pad', workspaceId: 'ws', profileId: 'default' };
    const state = { tasks: [task], workspaces: [{ id: 'ws', path: root }], profiles: [{ id: 'default', name: 'Fixture' }] };
    const host = new WorkbenchHost({ root, store: { root, state, mutate() { assert.fail('metadata write'); } },
      emit() { assert.fail('event'); }, diagnostic() { assert.fail('diagnostic'); } });
    let live = null;
    host.runtime = { get(id) { assert.equal(id, task.id); return live; }, ensure() { assert.fail('child startup'); } };
    host.profileDirs = () => assert.fail('launchSpec directory creation');
    host.sdkClient = () => assert.fail('SDK / SessionManager inspection');
    const read = async () => {
      const before = JSON.stringify(state);
      const result = await host.execute('session_info', { taskId: task.id });
      assert.equal(JSON.stringify(state), before, 'inspection must not mutate task/profile metadata');
      return result;
    };
    assert.deepEqual(await read(), { taskId: task.id, engine: 'pi', cwd: root, profileName: 'Fixture',
      state: 'not_created', fileState: 'absent' });
    const file = path.join(sessions, `${task.id}.jsonl`);
    const contents = JSON.stringify({ type: 'session', version: 3, id: 'actual-pi-header-id', cwd: root }) + '\nSECRET_TRANSCRIPT';
    fs.writeFileSync(file, contents);
    const info = await read();
    assert.equal(info.sessionId, 'actual-pi-header-id');
    assert.notEqual(info.sessionId, task.id);
    assert.equal(info.source, 'header');
    assert.equal(info.fileState, 'present');
    assert.equal(fs.readFileSync(file, 'utf8'), contents);
    fs.unlinkSync(file);
    live = { state: { sessionId: 'actual-live-pi-id', sessionFile: file } };
    assert.deepEqual(await read(), { taskId: task.id, engine: 'pi', cwd: root, profileName: 'Fixture',
      state: 'available', fileState: 'absent', sessionId: 'actual-live-pi-id', sessionFile: file, source: 'runtime' });
    live = null;
    const outside = path.join(root, 'private.jsonl');
    fs.writeFileSync(outside, contents);
    task.sessionFile = outside;
    assert.equal((await read()).state, 'unavailable');
    assert.equal((await read()).sessionFile, undefined);
    task.sessionFile = file;
    fs.symlinkSync(outside, file);
    assert.equal((await read()).state, 'unavailable');
    fs.unlinkSync(file);
    fs.writeFileSync(file, '{SECRET_HEADER');
    const corrupt = await read();
    assert.equal(corrupt.state, 'unavailable');
    assert.equal(corrupt.sessionId, undefined);
    assert.equal(corrupt.sessionFile, undefined);
    assert.ok(!JSON.stringify(corrupt).includes('SECRET'));
  } finally { fs.rmSync(root, { recursive: true, force: true }); }
});
