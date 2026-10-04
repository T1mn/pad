import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { inspectDataRoot, migrateDataRoot, MIGRATION_RECEIPT, INCOMPLETE_MARKER, withStandaloneBridgeStartup, withDataRootStartup } from './data-root-migration.mjs';
import { WorkbenchStore } from './workbench-store.mjs';
import { resolveDataRoot, resolveLegacyDataRoot } from './host-env.mjs';

function fixture(t) {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), 'pad-migration-'));
  t.after(() => fs.rmSync(home, { recursive: true, force: true }));
  const source = resolveLegacyDataRoot({}, home);
  const destination = resolveDataRoot({}, home);
  function write(relative, bytes) {
    const file = path.join(source, relative);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, bytes);
    return file;
  }
  const parent = write('v1/profiles/alpha/pi-sessions/parent.jsonl', '{"type":"session","id":"parent"}\n');
  const transcript = `{"type":"message","content":"opaque ${source}"}\n`;
  const sessionFile = write('v1/profiles/alpha/pi-sessions/task.jsonl', `${JSON.stringify({ type: 'session', id: 'child', parentSession: parent })}\n${transcript}`);
  write('v1/profiles/beta/pi-sessions/history.jsonl', '{"type":"session","id":"beta"}\n{"type":"message","content":"saved history"}\n');
  // CLI-only profile, not registered in workbench metadata, must also survive.
  const auth = Buffer.from([0, 1, 2, 127, 255]);
  write('v1/profiles/cli-only/pi-agent/auth.json', auth);
  write('v1/profiles/alpha/pi-agent/settings.json', `{"opaque":"${source}"}`);
  const installation = Buffer.from(' { "installationId": "synthetic-offline-id" }\n');
  write('v1/installation-id.json', installation);
  const state = {
    version: 1,
    workspaces: [{ id: 'ws-one', name: 'Workspace', path: path.join(source, 'external-workspace-path') }],
    profiles: [{ id: 'alpha', name: 'First' }, { id: 'beta', name: 'Second' }],
    tasks: [
      { id: 'task-one', workspaceId: 'ws-one', profileId: 'alpha', title: 'Task', status: 'idle', updatedAt: '2026-01-01', sessionFile },
      { id: 'task-two', workspaceId: 'ws-one', profileId: 'beta', title: 'Other', status: 'idle', updatedAt: '2026-01-01', sessionFile: path.join(source, 'v1/profiles/beta/pi-sessions/history.jsonl') },
    ],
    externalProvenance: `original ${source}`,
  };
  write('v1/workbench.json', JSON.stringify(state));
  return { home, source, destination, write, state, auth, installation, transcript };
}

test('explicit migration preserves all profiles and opaque bytes, rebases only owned structural paths, and retains source', (t) => {
  const f = fixture(t);
  const original = fs.readFileSync(path.join(f.source, 'v1/workbench.json'));
  assert.throws(() => inspectDataRoot({ env: {}, home: f.home }), /Legacy PAD data.*--confirm-closed/);
  assert.throws(() => migrateDataRoot({ home: f.home }), /requires --confirm-closed/);
  assert.equal(inspectDataRoot({ env: { PAD_TERMINAL_DATA_ROOT: f.source }, home: f.home }), f.source);
  const receipt = migrateDataRoot({ confirmClosed: true, home: f.home });
  assert.equal(receipt.status, 'completed');
  assert.equal(receipt.source, f.source);
  assert.equal(receipt.destination, f.destination);
  assert.deepEqual(fs.readFileSync(path.join(f.source, 'v1/workbench.json')), original);
  assert.equal(fs.existsSync(path.join(f.source, 'v1/host.lock')), false);
  assert.equal(fs.existsSync(path.join(f.destination, 'v1/host.lock')), false);
  const migrated = JSON.parse(fs.readFileSync(path.join(f.destination, 'v1/workbench.json')));
  assert.deepEqual(migrated.profiles, f.state.profiles);
  assert.deepEqual(migrated.tasks.map((task) => ({ ...task, sessionFile: path.join(f.source, path.relative(f.destination, task.sessionFile)) })), f.state.tasks);
  assert.equal(migrated.tasks[0].sessionFile, path.join(f.destination, 'v1/profiles/alpha/pi-sessions/task.jsonl'));
  assert.deepEqual(migrated.workspaces, f.state.workspaces);
  assert.equal(migrated.externalProvenance, f.state.externalProvenance);
  const session = fs.readFileSync(migrated.tasks[0].sessionFile, 'utf8');
  assert.equal(JSON.parse(session.split('\n')[0]).parentSession, path.join(f.destination, 'v1/profiles/alpha/pi-sessions/parent.jsonl'));
  assert.equal(session.substring(session.indexOf('\n') + 1), f.transcript);
  for (const relative of ['v1/profiles/beta/pi-sessions/history.jsonl', 'v1/profiles/alpha/pi-agent/settings.json', 'v1/profiles/cli-only/pi-agent/auth.json', 'v1/installation-id.json']) {
    assert.deepEqual(fs.readFileSync(path.join(f.destination, relative)), fs.readFileSync(path.join(f.source, relative)));
    assert.equal(fs.statSync(path.join(f.destination, relative)).mode & 0o777, 0o600);
  }
  assert.equal(fs.statSync(f.destination).mode & 0o777, 0o700);
  assert.equal(fs.existsSync(path.join(f.destination, MIGRATION_RECEIPT)), true);
  assert.equal(inspectDataRoot({ env: {}, home: f.home }), f.destination);
  f.write('v1/profiles/alpha/pi-agent/settings.json', 'old binary diverged');
  assert.throws(() => inspectDataRoot({ env: {}, home: f.home }), /Both PAD roots.*explicitly/);
  assert.equal(inspectDataRoot({ env: { PAD_TERMINAL_DATA_ROOT: f.destination }, home: f.home }), f.destination);
});

test('publication reserves without replacing a racing destination and incomplete roots refuse overrides', (t) => {
  const f = fixture(t);
  const mkdir = fs.mkdirSync;
  fs.mkdirSync = function (file, options) {
    if (file === f.destination && options?.recursive !== true) {
      mkdir(file, options);
      fs.writeFileSync(path.join(file, 'foreign'), 'keep');
    }
    return mkdir(file, options);
  };
  try { assert.throws(() => migrateDataRoot({ confirmClosed: true, home: f.home }), /Destination appeared/); }
  finally { fs.mkdirSync = mkdir; }
  assert.equal(fs.readFileSync(path.join(f.destination, 'foreign'), 'utf8'), 'keep');
  fs.rmSync(f.destination, { recursive: true });
  const write = fs.writeFileSync;
  fs.writeFileSync = function (file, ...args) {
    if (typeof file === 'number') {
      // Destination writes use exclusive descriptors; fail only after reservation.
      if (fs.existsSync(path.join(f.destination, 'v1'))) throw new Error('fixture publication failure');
    }
    return write(file, ...args);
  };
  try { assert.throws(() => migrateDataRoot({ confirmClosed: true, home: f.home }), /fixture publication failure/); }
  finally { fs.writeFileSync = write; }
  assert.equal(fs.existsSync(f.destination), false);
  assert.equal(fs.existsSync(path.join(f.source, 'v1/workbench.json')), true);
  fs.mkdirSync(f.destination);
  fs.writeFileSync(path.join(f.destination, INCOMPLETE_MARKER), 'incomplete');
  for (const env of [{}, { PAD_TERMINAL_DATA_ROOT: f.destination }]) {
    assert.throws(() => inspectDataRoot({ env, home: f.home }), /Incomplete/);
  }
});

test('validated standalone profiles migrate and adopt existing default without reading auth', (t) => {
  const f = fixture(t);
  fs.rmSync(f.source, { recursive: true });
  for (const id of ['other', 'default']) {
    fs.mkdirSync(path.join(f.source, 'v1/profiles', id, 'pi-sessions'), { recursive: true });
    f.write(`v1/profiles/${id}/pi-agent/auth.json`, f.auth);
  }
  const lease = withStandaloneBridgeStartup((root) => root, { env: { PAD_TERMINAL_DATA_ROOT: f.source }, home: f.home });
  try { assert.throws(() => migrateDataRoot({ confirmClosed: true, home: f.home }), /bridge lease/); }
  finally { lease.release(); }
  migrateDataRoot({ confirmClosed: true, home: f.home });
  assert.equal(fs.existsSync(path.join(f.source, 'v1/workbench.json')), false);
  const store = new WorkbenchStore({ root: f.destination });
  try {
    const state = withDataRootStartup(() => store.open(), { env: {}, home: f.home });
    assert.deepEqual(state.profiles.map((profile) => profile.id), ['default', 'other']);
    assert.deepEqual(fs.readFileSync(path.join(f.destination, 'v1/profiles/default/pi-agent/auth.json')), f.auth);
  } finally { store.releaseLock(); }
  // Explicit initialization of a standalone source adopts too; no empty account.
  const legacyStore = new WorkbenchStore({ root: f.source });
  try {
    const state = withDataRootStartup(() => legacyStore.open(), { env: { PAD_TERMINAL_DATA_ROOT: f.source }, home: f.home });
    assert.equal(state.profiles[0].id, 'default');
  } finally { legacyStore.releaseLock(); }
});

test('malformed and invalid metadata diagnostics never expose stored secret text', (t) => {
  const f = fixture(t);
  const secret = 'SECRET_SENTINEL_DO_NOT_PRINT';
  for (const text of [`{"${secret}":`, JSON.stringify({ ...f.state, version: secret }), JSON.stringify({ ...f.state, tasks: [{ ...f.state.tasks[0], status: secret }] })]) {
    f.write('v1/workbench.json', text);
    for (const operation of [
      () => inspectDataRoot({ env: { PAD_TERMINAL_DATA_ROOT: f.source }, home: f.home }),
      () => migrateDataRoot({ confirmClosed: true, home: f.home }),
      () => new WorkbenchStore({ root: f.source }).load(),
    ]) {
      assert.throws(operation, (error) => !error.message.includes(secret) && /metadata/.test(error.message));
    }
  }
});

test('live or malformed source locks and existing destinations never publish partial data', (t) => {
  const f = fixture(t);
  const lock = f.write('v1/host.lock', JSON.stringify({ pid: process.pid, startedAt: 'now' }));
  assert.throws(() => migrateDataRoot({ confirmClosed: true, home: f.home }), /Lock already exists/);
  assert.equal(fs.existsSync(f.destination), false);
  assert.equal(JSON.parse(fs.readFileSync(lock)).pid, process.pid);
  fs.writeFileSync(lock, 'malformed');
  assert.throws(() => migrateDataRoot({ confirmClosed: true, home: f.home }), /Lock already exists/);
  assert.equal(fs.readFileSync(lock, 'utf8'), 'malformed');
  fs.unlinkSync(lock);
  fs.mkdirSync(f.destination);
  fs.writeFileSync(path.join(f.destination, 'keep'), 'collision');
  assert.throws(() => migrateDataRoot({ confirmClosed: true, home: f.home }), /Destination already exists/);
  assert.equal(fs.readFileSync(path.join(f.destination, 'keep'), 'utf8'), 'collision');
  assert.throws(() => inspectDataRoot({ env: {}, home: f.home }));
  fs.rmSync(f.destination, { recursive: true });
  fs.cpSync(f.source, f.destination, { recursive: true });
  assert.throws(() => inspectDataRoot({ env: {}, home: f.home }), /Both PAD roots/);
  assert.equal(fs.readdirSync(path.dirname(f.destination)).some((name) => name.includes('migration')), false);
  fs.rmSync(f.source, { recursive: true });
  assert.equal(inspectDataRoot({ env: {}, home: f.home }), f.destination);
  fs.unlinkSync(path.join(f.destination, 'v1/workbench.json'));
  assert.throws(() => inspectDataRoot({ env: {}, home: f.home }));
});
