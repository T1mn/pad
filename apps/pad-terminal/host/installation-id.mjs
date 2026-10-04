import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { AuthError } from './auth-policy.mjs';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
function privateDirectory(dir) {
  let created = true;
  try { fs.mkdirSync(dir, { mode: 0o700 }); } catch (error) {
    created = false;
    if (error.code !== 'EEXIST') throw error;
  }
  const stat = fs.lstatSync(dir);
  if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('Invalid directory');
  fs.chmodSync(dir, 0o700);
  return created;
}
function syncDirectory(dir) {
  const fd = fs.openSync(dir, fs.constants.O_RDONLY | fs.constants.O_DIRECTORY | fs.constants.O_NOFOLLOW);
  try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}
function readIdentity(file) {
  let fd;
  try {
    // lstat before open also rejects FIFOs without a blocking open.
    const stat = fs.lstatSync(file);
    if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('Invalid identity');
    fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK);
    const opened = fs.fstatSync(fd);
    if (!opened.isFile() || opened.size > 1024) throw new Error('Invalid identity');
    const value = JSON.parse(fs.readFileSync(fd, 'utf8'));
    if (!value || value.version !== 1 || typeof value.deviceId !== 'string' || !UUID.test(value.deviceId)
        || Object.keys(value).length !== 2) throw new Error('Invalid identity');
    fs.fchmodSync(fd, 0o600);
    return value.deviceId;
  } finally { if (fd !== undefined) fs.closeSync(fd); }
}

/** Synchronous SDK callback. No disk access or UUID generation until requested. */
export function createInstallationIdGetter({ dataRoot }) {
  const root = path.resolve(dataRoot);
  const dir = path.join(root, 'v1');
  const file = path.join(dir, 'installation-id.json');
  return () => {
    let temp;
    let fd;
    try {
      const rootCreated = privateDirectory(root);
      privateDirectory(dir);
      try { return readIdentity(file); } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      const deviceId = crypto.randomUUID();
      temp = path.join(dir, `.installation-id-${crypto.randomUUID()}.tmp`);
      fd = fs.openSync(temp, 'wx', 0o600);
      fs.writeFileSync(fd, `${JSON.stringify({ version: 1, deviceId })}\n`);
      fs.fsyncSync(fd);
      fs.closeSync(fd);
      fd = undefined;
      // Hard-link publication is atomic and cannot replace a competing winner.
      try { fs.linkSync(temp, file); } catch (error) {
        if (error.code !== 'EEXIST') throw error;
      }
      fs.unlinkSync(temp);
      temp = undefined;
      syncDirectory(dir);
      syncDirectory(root);
      if (rootCreated) syncDirectory(path.dirname(root));
      return readIdentity(file);
    } catch {
      throw new AuthError('identity_persistence');
    } finally {
      if (fd !== undefined) { try { fs.closeSync(fd); } catch {} }
      if (temp) { try { fs.unlinkSync(temp); } catch {} }
    }
  };
}
