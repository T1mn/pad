import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createInstallationIdGetter } from './installation-id.mjs';
import { AuthError } from './auth-policy.mjs';

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'pad-identity-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}
test('identity is lazy, shared, private and stable across getter restarts', (t) => {
  const root = fixture(t);
  const file = path.join(root, 'v1', 'installation-id.json');
  const first = createInstallationIdGetter({ dataRoot: root });
  assert.equal(fs.existsSync(file), false);
  const id = first();
  assert.equal(typeof id, 'string');
  assert.equal(first(), id);
  assert.equal(createInstallationIdGetter({ dataRoot: root })(), id);
  assert.deepEqual(JSON.parse(fs.readFileSync(file, 'utf8')), { version: 1, deviceId: id });
  assert.equal(fs.statSync(file).mode & 0o777, 0o600);
  assert.equal(fs.statSync(path.dirname(file)).mode & 0o777, 0o700);
  assert.notEqual(createInstallationIdGetter({ dataRoot: fixture(t) })(), id);
});
test('competing publisher reads the winner without clobbering it', (t) => {
  const root = fixture(t);
  const winner = '12345678-1234-4234-8234-123456789abc';
  const originalLink = fs.linkSync;
  fs.linkSync = (_temp, file) => {
    fs.writeFileSync(file, JSON.stringify({ version: 1, deviceId: winner }), { flag: 'wx', mode: 0o600 });
    throw Object.assign(new Error('competing publisher'), { code: 'EEXIST' });
  };
  try {
    assert.equal(createInstallationIdGetter({ dataRoot: root })(), winner);
    assert.deepEqual(fs.readdirSync(path.join(root, 'v1')), ['installation-id.json']);
  } finally { fs.linkSync = originalLink; }
});
test('invalid, unsupported and symlink identities are rejected, never rotated', (t) => {
  const root = fixture(t);
  const dir = path.join(root, 'v1');
  fs.mkdirSync(dir);
  const file = path.join(dir, 'installation-id.json');
  for (const text of ['broken', '{"version":2,"deviceId":"invalid"}', '{"version":1,"deviceId":"invalid"}']) {
    fs.writeFileSync(file, text);
    assert.throws(createInstallationIdGetter({ dataRoot: root }), (e) => e instanceof AuthError && e.code === 'identity_persistence');
    assert.equal(fs.readFileSync(file, 'utf8'), text);
  }
  fs.unlinkSync(file);
  const target = path.join(root, 'target');
  fs.writeFileSync(target, 'unchanged');
  fs.symlinkSync(target, file);
  assert.throws(createInstallationIdGetter({ dataRoot: root }), AuthError);
  assert.equal(fs.readFileSync(target, 'utf8'), 'unchanged');
  fs.unlinkSync(file);
  fs.mkdirSync(file);
  assert.throws(createInstallationIdGetter({ dataRoot: root }), AuthError);
});
