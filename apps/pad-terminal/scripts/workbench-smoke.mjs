#!/usr/bin/env node
// One offline integration path. Never authenticate or send an accepted prompt.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { sanitizeAmbientCredentials } from '../host/host-env.mjs';

sanitizeAmbientCredentials(process.env);
process.env.PI_OFFLINE = '1';
const { WorkbenchStore } = await import('../host/workbench-store.mjs');
const { WorkbenchHost } = await import('../host/workbench.mjs');
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const dataRoot = fs.mkdtempSync(path.join(root, '.cache/workbench-smoke-'));
const workspacePath = path.join(dataRoot, 'workspace');
fs.mkdirSync(workspacePath, { mode: 0o700 });
const open = () => {
  const store = new WorkbenchStore({ root: dataRoot });
  store.open();
  return new WorkbenchHost({ root: dataRoot, store, emit() {}, diagnostic() {} });
};
let host = open();
let result;
try {
  const profile = host.snapshot().profiles[0];
  const otherProfile = host.addProfile('第二个隔离账号');
  const catalog = await host.catalog(profile.id);
  assert.ok(catalog.providers.length > 0);
  assert.ok(catalog.providers.some((item) => item.authTypes.includes('oauth')));
  assert.ok(catalog.providers.some((item) => item.authTypes.includes('api_key')));
  assert.equal(catalog.models.length, 0);
  assert.equal(catalog.providers.filter((item) => item.authenticated).length, 0);
  assert.notEqual(host.profileDirs(profile.id).agent, host.profileDirs(otherProfile.id).agent);
  const workspace = host.addWorkspace(workspacePath);
  const task = host.createTask({ workspaceId: workspace.id, profileId: profile.id, title: '离线验收（非模型会话）' });
  assert.deepEqual((await host.history(task.id)).messages, []);
  await assert.rejects(host.prompt(task.id, 'must never reach a model'), /Select a model/);
  const peer = await host.ensureTaskProcess(task);
  const piPid = peer.child.pid;
  const bash = await peer.request({ type: 'bash', command: "printf 'PAD_NATIVE_HOST_RPC_OK 中文\\n'" });
  assert.equal(bash.exitCode, 0);
  assert.match(bash.output, /PAD_NATIVE_HOST_RPC_OK 中文/);
  const stats = await peer.request({ type: 'get_session_stats' });
  assert.equal(stats.cost, 0);
  assert.equal(stats.tokens.total, 0);
  const storedTask = host.requireTask(task.id);
  assert.ok(storedTask.sessionFile.startsWith(host.profileDirs(profile.id).sessions + path.sep));
  await host.close();
  assert.throws(() => process.kill(piPid, 0), { code: 'ESRCH' });

  // Explicit offline fixture: test history restore without impersonating a model reply.
  const historyText = '离线历史夹具：未登录，未发送模型请求。';
  const timestamp = new Date().toISOString();
  fs.writeFileSync(storedTask.sessionFile, [
    { type: 'session', version: 3, id: randomUUID(), timestamp, cwd: workspacePath },
    { type: 'message', id: 'fixture1', parentId: null, timestamp,
      message: { role: 'user', content: historyText, timestamp: Date.now() } },
  ].map((entry) => JSON.stringify(entry)).join('\n') + '\n', { mode: 0o600 });
  host = open();
  assert.equal(host.snapshot().tasks[0].id, task.id);
  assert.equal((await host.history(task.id)).messages[0].content, historyText);
  assert.equal(host.runtime.processes.size, 0, 'history browsing must not create a Pi process');
  assert.equal(fs.statSync(host.store.statePath).mode & 0o777, 0o600);
  result = { dataRoot, taskId: task.id, workspaceId: workspace.id, profileId: profile.id,
    providers: catalog.providers.length, availableModels: 0, modelRequests: 0, stats,
    rpcOutput: bash.output, historyText, metadataRestored: true, childStopped: true };
} finally {
  await host.close();
}
fs.writeFileSync(path.join(dataRoot, 'result.json'), JSON.stringify(result, null, 2) + '\n', { mode: 0o600 });
console.log(JSON.stringify(result, null, 2));
