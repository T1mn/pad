// Private metadata store for the native workbench: v1/workbench.json under the
// Preview data root, with a single-host lock and atomic, 0o600 writes.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { isThinkingLevel } from './thinking-levels.mjs';

export const STATE_VERSION = 1;
export const TASK_STATUSES = ['idle', 'starting', 'running', 'error'];
const STATUS_SET = new Set(TASK_STATUSES);
const EPHEMERAL_STATUSES = new Set(['starting', 'running']);

export class CorruptStateError extends Error {
  constructor(message) {
    super(message);
    this.name = 'CorruptStateError';
  }
}

export class HostLockError extends Error {
  constructor(message) {
    super(message);
    this.name = 'HostLockError';
  }
}

export function nowIso() {
  return new Date().toISOString();
}

function fail(message) {
  throw new CorruptStateError(`Workbench metadata is invalid: ${message}`);
}

function requireString(value, field) {
  if (typeof value !== 'string' || value.length === 0) fail(`${field} must be a non-empty string`);
  return value;
}

function optionalString(value, field) {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== 'string') fail(`${field} must be a string when present`);
  return value;
}

function requireArray(value, field) {
  if (!Array.isArray(value)) fail(`${field} must be an array`);
  return value;
}

function validateWorkspace(value, index) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail(`workspaces[${index}] must be an object`);
  requireString(value.id, `workspaces[${index}].id`);
  requireString(value.name, `workspaces[${index}].name`);
  requireString(value.path, `workspaces[${index}].path`);
  return { id: value.id, name: value.name, path: value.path };
}

function validateProfile(value, index) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail(`profiles[${index}] must be an object`);
  requireString(value.id, `profiles[${index}].id`);
  requireString(value.name, `profiles[${index}].name`);
  return { id: value.id, name: value.name };
}

function validateTask(value, index) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail(`tasks[${index}] must be an object`);
  const status = requireString(value.status, `tasks[${index}].status`);
  if (!STATUS_SET.has(status)) fail('tasks status is unknown');
  const task = {
    id: requireString(value.id, `tasks[${index}].id`),
    workspaceId: requireString(value.workspaceId, `tasks[${index}].workspaceId`),
    profileId: requireString(value.profileId, `tasks[${index}].profileId`),
    title: requireString(value.title, `tasks[${index}].title`),
    status,
    updatedAt: requireString(value.updatedAt, `tasks[${index}].updatedAt`),
  };
  const provider = optionalString(value.provider, `tasks[${index}].provider`);
  const modelId = optionalString(value.modelId, `tasks[${index}].modelId`);
  const sessionFile = optionalString(value.sessionFile, `tasks[${index}].sessionFile`);
  if (provider !== undefined) task.provider = provider;
  if (modelId !== undefined) task.modelId = modelId;
  if (sessionFile !== undefined) task.sessionFile = sessionFile;
  if (value.thinkingLevel !== undefined) {
    if (!isThinkingLevel(value.thinkingLevel)) fail('tasks thinkingLevel is unknown');
    task.thinkingLevel = value.thinkingLevel;
  }
  return task;
}

/** Validate an on-disk state document. Throws CorruptStateError; never repairs silently. */
export function validateState(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail('root must be an object');
  if (value.version !== STATE_VERSION) fail('unsupported version');
  const workspaces = requireArray(value.workspaces, 'workspaces').map(validateWorkspace);
  const profiles = requireArray(value.profiles, 'profiles').map(validateProfile);
  const tasks = requireArray(value.tasks, 'tasks').map(validateTask);
  const workspaceIds = new Set(workspaces.map((workspace) => workspace.id));
  const profileIds = new Set(profiles.map((profile) => profile.id));
  const taskIds = new Set();
  for (const task of tasks) {
    if (taskIds.has(task.id)) fail('duplicate task id');
    taskIds.add(task.id);
    if (!workspaceIds.has(task.workspaceId)) fail('task references an unknown workspace');
    if (!profileIds.has(task.profileId)) fail('task references an unknown profile');
  }
  return { version: STATE_VERSION, workspaces, profiles, tasks };
}

/** Fresh native profile id; never the CLI's shared `default` account dir. */
export function nativeProfileId() {
  return `profile-${crypto.randomUUID().replace(/-/g, '').slice(0, 12)}`;
}

export function defaultState() {
  return {
    version: STATE_VERSION,
    workspaces: [],
    profiles: [{ id: nativeProfileId(), name: 'Default' }],
    tasks: [],
  };
}

/** Old standalone bridge signature: only v1/profiles/<id>/{pi-agent,pi-sessions}.
 * Validate directory structure, never read authentication or session payloads.
 * Also used by root inspection; no import back into the root/bridge modules.
 */
export function standaloneState(root) {
  const invalid = () => { throw new CorruptStateError('Unrecognized standalone root structure'); };
  const directory = (file) => { if (!fs.lstatSync(file).isDirectory()) invalid(); };
  directory(root);
  if (fs.readdirSync(root).some((name) => name !== 'v1')) invalid();
  const v1 = path.join(root, 'v1');
  directory(v1);
  if (fs.readdirSync(v1).some((name) => !['profiles', 'host.lock'].includes(name))) invalid();
  const profiles = path.join(v1, 'profiles');
  directory(profiles);
  const ids = fs.readdirSync(profiles).sort();
  if (!ids.length) invalid();
  function walk(file) {
    const stat = fs.lstatSync(file);
    if (stat.isDirectory()) for (const name of fs.readdirSync(file)) walk(path.join(file, name));
    else if (!stat.isFile()) invalid();
  }
  for (const id of ids) {
    if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(id)) invalid();
    const profile = path.join(profiles, id);
    directory(profile);
    const children = fs.readdirSync(profile).sort();
    if (children.join(',') !== 'pi-agent,pi-sessions') invalid();
    for (const name of children) { directory(path.join(profile, name)); walk(path.join(profile, name)); }
  }
  ids.sort((a, b) => a === 'default' ? -1 : b === 'default' ? 1 : a.localeCompare(b));
  return { version: STATE_VERSION, workspaces: [], profiles: ids.map((id) => ({ id, name: id === 'default' ? 'Default' : id })), tasks: [] };
}

function ensurePrivateDir(directory) {
  fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
  try {
    fs.chmodSync(directory, 0o700);
  } catch {
    // Best effort; the directory may live on a filesystem without POSIX modes.
  }
}

function processAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code === 'EPERM';
  }
}

export class WorkbenchStore {
  constructor({ root }) {
    this.root = path.resolve(root);
    this.dir = path.join(this.root, 'v1');
    this.statePath = path.join(this.dir, 'workbench.json');
    this.lockPath = path.join(this.dir, 'host.lock');
    this.state = null;
    this.locked = false;
  }

  /** Create directories, take the host lock, and load (or seed) metadata. */
  open() {
    ensurePrivateDir(this.dir);
    this.acquireLock();
    this.state = this.load();
    // Statuses are process-local; a crash must not resurrect a running task.
    let changed = false;
    for (const task of this.state.tasks) {
      if (EPHEMERAL_STATUSES.has(task.status)) {
        task.status = 'idle';
        task.updatedAt = nowIso();
        changed = true;
      }
    }
    if (changed) this.save(this.state);
    return this.state;
  }

  acquireLock() {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        const fd = fs.openSync(this.lockPath, 'wx', 0o600);
        fs.writeFileSync(fd, `${JSON.stringify({ pid: process.pid, startedAt: nowIso() })}\n`);
        fs.closeSync(fd);
        this.locked = true;
        return;
      } catch (error) {
        if (error?.code !== 'EEXIST') throw error;
        const owner = this.readLockOwner();
        if (owner !== null && processAlive(owner)) {
          throw new HostLockError('Another PAD Terminal host is already running for this data root');
        }
        try {
          fs.rmSync(this.lockPath, { force: true });
        } catch {
          // Retry; a concurrent holder may still win.
        }
      }
    }
    throw new HostLockError('Unable to acquire the PAD Terminal host lock');
  }

  readLockOwner() {
    try {
      const parsed = JSON.parse(fs.readFileSync(this.lockPath, 'utf8'));
      return Number.isInteger(parsed?.pid) ? parsed.pid : null;
    } catch {
      return null;
    }
  }

  releaseLock() {
    if (!this.locked) return;
    this.locked = false;
    if (this.readLockOwner() !== process.pid) return;
    try {
      fs.rmSync(this.lockPath, { force: true });
    } catch {
      // Lock cleanup is best effort on shutdown.
    }
  }

  /** Read metadata. Missing file seeds the isolated default profile; corrupt state throws. */
  load() {
    let text;
    try {
      text = fs.readFileSync(this.statePath, 'utf8');
    } catch (error) {
      if (error?.code === 'ENOENT') {
        const seeded = fs.existsSync(path.join(this.dir, 'profiles')) ? standaloneState(this.root) : defaultState();
        this.save(seeded);
        return seeded;
      }
      throw error;
    }
    let parsed;
    try {
      parsed = JSON.parse(text);
    } catch {
      throw new CorruptStateError('Workbench metadata is not valid JSON (workbench.json)');
    }
    return validateState(parsed);
  }

  /** Persist `state` atomically with private modes. */
  save(state) {
    const validated = validateState(state);
    const serialized = `${JSON.stringify(validated, null, 2)}\n`;
    const temp = path.join(this.dir, `.workbench.json.${process.pid}.${crypto.randomBytes(4).toString('hex')}.tmp`);
    const fd = fs.openSync(temp, 'wx', 0o600);
    try {
      fs.writeFileSync(fd, serialized);
      fs.fsyncSync(fd);
    } finally {
      fs.closeSync(fd);
    }
    try {
      fs.renameSync(temp, this.statePath);
    } catch (error) {
      fs.rmSync(temp, { force: true });
      throw error;
    }
    try {
      fs.chmodSync(this.statePath, 0o600);
    } catch {
      // Best effort.
    }
    try {
      const dirFd = fs.openSync(this.dir, 'r');
      try {
        fs.fsyncSync(dirFd);
      } finally {
        fs.closeSync(dirFd);
      }
    } catch {
      // Directory fsync is not supported everywhere; the rename already happened.
    }
    this.state = validated;
    return validated;
  }

  /** Transactional helper: mutate a structural clone, then persist on success. */
  mutate(fn) {
    if (!this.state) throw new Error('Workbench store is not open');
    const draft = structuredClone(this.state);
    const result = fn(draft);
    this.save(draft);
    return result;
  }
}
