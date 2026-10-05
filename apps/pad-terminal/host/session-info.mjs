// Read-only Pi identity inspection. Never loads the SDK, starts a child, or
// derives an identity from a filename. RPC get_state.sessionId / header.id only.
import fs from 'node:fs';
import path from 'node:path';

const HEADER_BYTES = 16 * 1024;
const validId = (id) => typeof id === 'string' && /^[A-Za-z0-9][A-Za-z0-9._-]{0,255}$/.test(id);

// Reject symlinks throughout the owned profile subtree, including missing-file
// parents. Canonicalizing the data root itself supports an explicitly chosen root.
function inspectPath(root, profileId, file) {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(profileId)) return null;
  const base = path.resolve(root);
  const sessions = path.join(base, 'v1', 'profiles', profileId, 'pi-sessions');
  if (!path.isAbsolute(file) || path.extname(file) !== '.jsonl') return null;
  const candidate = path.resolve(file);
  const relative = path.relative(sessions, candidate);
  if (!relative || relative.startsWith(`..${path.sep}`) || relative === '..' || path.isAbsolute(relative)) return null;
  let current = fs.realpathSync(base);
  const parts = path.relative(base, candidate).split(path.sep);
  for (let i = 0; i < parts.length; i++) {
    current = path.join(current, parts[i]);
    let stat;
    try { stat = fs.lstatSync(current); } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      return { file: path.join(current, ...parts.slice(i + 1)), exists: false };
    }
    if (stat.isSymbolicLink() || (i < parts.length - 1 ? !stat.isDirectory() : !stat.isFile())) return null;
  }
  return { file: current, exists: true };
}

function headerId(file, validate) {
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK);
  try {
    // Recheck after open, before reading: reject a replaced/symlinked parent
    // and ensure the descriptor still identifies the checked owned file.
    const checked = validate();
    if (!checked?.exists || checked.file !== file) return undefined;
    const opened = fs.fstatSync(fd);
    const current = fs.lstatSync(file);
    if (!opened.isFile() || !current.isFile() || opened.dev !== current.dev || opened.ino !== current.ino) return undefined;
    const bytes = Buffer.alloc(HEADER_BYTES);
    const count = fs.readSync(fd, bytes, 0, bytes.length, 0);
    const newline = bytes.subarray(0, count).indexOf(10);
    if (newline < 0 && count === HEADER_BYTES) return undefined;
    const header = JSON.parse(bytes.subarray(0, newline < 0 ? count : newline).toString('utf8'));
    return header?.type === 'session' && validId(header.id) &&
      (header.version === undefined || [1, 2, 3].includes(header.version)) ? header.id : undefined;
  } finally { fs.closeSync(fd); }
}

export function sessionInfo({ root, task, cwd, profileName, runtime }) {
  const dto = { taskId: task.id, engine: 'pi', cwd,
    ...(profileName ? { profileName } : {}), state: 'unavailable', fileState: 'unavailable' };
  try {
    const live = runtime.get(task.id); // get only: NEVER ensure / launch.
    const state = live?.state;
    const runtimeId = validId(state?.sessionId) ? state.sessionId : undefined;
    const pinned = path.join(path.resolve(root), 'v1', 'profiles', task.profileId, 'pi-sessions', `${task.id}.jsonl`);
    const file = state?.sessionFile ?? task.sessionFile ?? pinned;
    if (typeof file !== 'string') return dto;
    // Both saved and runtime paths must belong to this profile. Never expose a
    // bad saved path even when a live child reports a different safe path.
    if (task.sessionFile && !inspectPath(root, task.profileId, task.sessionFile)) return dto;
    const owned = inspectPath(root, task.profileId, file);
    if (!owned) return dto;
    dto.fileState = owned.exists ? 'present' : 'absent';
    if (runtimeId) {
      return { ...dto, state: 'available', sessionId: runtimeId,
        sessionFile: owned.file, source: 'runtime' };
    }
    if (!owned.exists) {
      return { ...dto, state: !task.sessionFile && !live ? 'not_created' : 'unavailable' };
    }
    const id = headerId(owned.file, () => inspectPath(root, task.profileId, file));
    if (!id) return { ...dto, fileState: 'unavailable' };
    return { ...dto, state: 'available', sessionId: id, sessionFile: owned.file, source: 'header' };
  } catch {
    // No raw parser/filesystem errors (or content) cross the wire or get logged.
    return { ...dto, state: 'unavailable', fileState: 'unavailable' };
  }
}
