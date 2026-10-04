import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { WorkbenchStore } from './workbench-store.mjs';

test('persists private metadata atomically, locks one host, and rejects corrupt state', () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), 'pad-workbench-store-'));
  try {
    const store = new WorkbenchStore({ root: temporary });
    const seeded = store.open();
    assert.equal(seeded.version, 1);
    assert.equal(seeded.profiles.length, 1);
    assert.equal(seeded.profiles[0].name, 'Default');
    // The initial profile must not reuse the CLI's shared `default` account dir.
    assert.match(seeded.profiles[0].id, /^profile-[0-9a-f]{12}$/);
    assert.notEqual(seeded.profiles[0].id, 'default');
    assert.equal(seeded.tasks.length, 0);
    const statePath = path.join(temporary, 'v1', 'workbench.json');
    assert.equal(statSync(statePath).mode & 0o777, 0o600);
    assert.equal(statSync(path.join(temporary, 'v1')).mode & 0o777, 0o700);

    store.mutate((draft) => {
      draft.workspaces.push({ id: 'ws-1', name: 'demo', path: temporary });
      draft.tasks.push({ id: 'task-1', workspaceId: 'ws-1', profileId: seeded.profiles[0].id, title: 'Demo', status: 'running', updatedAt: new Date().toISOString() });
    });
    assert.equal(JSON.parse(readFileSync(statePath, 'utf8')).workspaces.length, 1);

    // A second host in the same data root must not be able to open concurrently.
    const second = new WorkbenchStore({ root: temporary });
    assert.throws(() => second.open(), /already running/);

    store.releaseLock();
    const third = new WorkbenchStore({ root: temporary });
    const reloaded = third.open();
    // Ephemeral statuses are normalized to idle on reopen.
    assert.equal(reloaded.tasks[0].status, 'idle');
    third.releaseLock();

    writeFileSync(statePath, '{ not valid json');
    const corrupt = new WorkbenchStore({ root: temporary });
    assert.throws(() => corrupt.load(), /not valid JSON/);
    assert.equal(readFileSync(statePath, 'utf8'), '{ not valid json');
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});
