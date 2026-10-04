// Explicit, offline data-root migration. No credentials or SDK are loaded here.
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { resolveDataRoot, resolveLegacyDataRoot } from './host-env.mjs';
import { validateState, standaloneState } from './workbench-store.mjs';

export const MIGRATION_RECEIPT = 'v1/data-root-migration.json';
export const INCOMPLETE_MARKER = '.data-root-incomplete.json';
const bridgeLeasePath = (root) => path.join(path.dirname(root), `.${path.basename(root)}.bridge.lock`);
const instructions = 'Close ALL PAD and standalone Pi clients, then run node scripts/migrate-data-root.mjs --confirm-closed; or explicitly choose a root with PAD_TERMINAL_DATA_ROOT. Never merge the roots.';
const lockRelative = 'v1/host.lock';
const exists = (file) => {
  try { fs.lstatSync(file); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; }
};
const exclusionPath = (root) => path.join(path.dirname(root), `.${path.basename(root)}.migration.lock`);
function fail(message) { throw new Error(`${message} ${instructions}`); }
function inside(root, file) {
  const relative = path.relative(root, file);
  return relative !== '' && relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}
function plain(file, directory = false) {
  const stat = fs.lstatSync(file);
  if (directory ? !stat.isDirectory() : !stat.isFile()) fail(`Not a regular ${directory ? 'directory' : 'file'}: ${file}.`);
  return stat;
}
function readRegular(file) {
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  try {
    if (!fs.fstatSync(fd).isFile()) fail(`Special file refused: ${file}.`);
    return fs.readFileSync(fd);
  } finally { fs.closeSync(fd); }
}
function identity(root) {
  const stat = plain(root, true);
  return { dev: stat.dev, ino: stat.ino };
}
function readState(root) {
  plain(root, true);
  if (exists(path.join(root, INCOMPLETE_MARKER))) fail('Incomplete data-root publication refused.');
  plain(path.join(root, 'v1'), true);
  const file = path.join(root, 'v1/workbench.json');
  if (!exists(file)) {
    try { return standaloneState(root); } catch { fail('Unrecognized or partial standalone root structure.'); }
  }
  plain(file);
  let raw, state;
  try {
    raw = JSON.parse(readRegular(file).toString('utf8'));
    state = validateState(raw);
  } catch { fail('Invalid workbench metadata (workbench.json).'); }
  for (const [name, items] of [['workspaces', state.workspaces], ['profiles', state.profiles], ['tasks', state.tasks]]) {
    const ids = new Set();
    for (const item of items) {
      if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(item.id) || ids.has(item.id)) fail(`Invalid or duplicate ${name} ID.`);
      ids.add(item.id);
    }
  }
  return raw; // Retain unknown metadata/provenance rather than serializing the normalized view.
}

/** Walk without following links. Digest content AND identities/timestamps to detect writers. */
function snapshot(root, { skipHostLock = false } = {}) {
  const records = [];
  function walk(relative) {
    if (skipHostLock && relative === lockRelative) return;
    const file = path.join(root, relative);
    const stat = fs.lstatSync(file, { bigint: true });
    if (!stat.isDirectory() && !stat.isFile()) fail(`Symlink or special file refused: ${file}.`);
    const record = [relative, stat.isDirectory() ? 'd' : 'f', ...['dev', 'ino', 'size', 'mtimeNs', 'ctimeNs', 'mode'].map((key) => String(stat[key]))];
    if (stat.isFile()) record.push(crypto.createHash('sha256').update(readRegular(file)).digest('hex'));
    records.push(record);
    if (stat.isDirectory()) for (const name of fs.readdirSync(file).sort()) walk(path.join(relative, name));
  }
  walk('');
  // Creating/removing our host.lock changes v1 directory timestamps, not its data.
  const v1 = records.find((record) => record[0] === 'v1');
  if (skipHostLock && v1) { v1[4] = ''; v1[5] = ''; v1[6] = ''; }
  return { records, digest: crypto.createHash('sha256').update(JSON.stringify(records)).digest('hex') };
}

/** Inspect BEFORE WorkbenchStore.open can seed missing metadata. Overrides skip legacy discovery. */
function inspectRoot({ env = process.env, home = env?.HOME || os.homedir() } = {}, reservedRoot) {
  const root = resolveDataRoot(env, home);
  if (root !== reservedRoot && exists(exclusionPath(root))) fail('A migration reservation exists for the selected root.');
  const override = typeof env.PAD_TERMINAL_DATA_ROOT === 'string' && env.PAD_TERMINAL_DATA_ROOT.trim() !== '';
  if (override) {
    if (exists(root)) {
      plain(root, true);
      if (fs.readdirSync(root).length !== 0) readState(root);
    }
    return root;
  }
  const legacy = resolveLegacyDataRoot(env, home);
  const hasNew = exists(root);
  const hasLegacy = exists(legacy);
  if (exists(exclusionPath(legacy))) fail('A legacy migration reservation exists.');
  if (!hasNew && !hasLegacy) return root;
  if (!hasNew) fail(`Legacy PAD data exists at ${legacy}; automatic initialization is disabled.`);
  readState(root);
  if (!hasLegacy) return root;
  try {
    if (exists(path.join(legacy, lockRelative))) throw new Error('retained legacy source has a host lock');
    const receiptFile = path.join(root, MIGRATION_RECEIPT);
    plain(receiptFile);
    const receipt = JSON.parse(readRegular(receiptFile).toString('utf8'));
    const sourceIdentity = identity(legacy);
    const targetIdentity = identity(root);
    if (receipt.version !== 1 || receipt.status !== 'completed' || receipt.source !== legacy || receipt.destination !== root
      || receipt.sourceIdentity?.dev !== sourceIdentity.dev || receipt.sourceIdentity?.ino !== sourceIdentity.ino
      || receipt.destinationIdentity?.dev !== targetIdentity.dev || receipt.destinationIdentity?.ino !== targetIdentity.ino
      || typeof receipt.completedAt !== 'string' || !Number.isFinite(Date.parse(receipt.completedAt))
      || receipt.sourceDigest !== snapshot(legacy, { skipHostLock: true }).digest) throw new Error('receipt mismatch or legacy source changed');
    readState(legacy);
  } catch {
    fail('Both PAD roots exist without a valid completed migration receipt. Choose a root explicitly; migration will not overwrite a destination.');
  }
  return root;
}

export function inspectDataRoot(options = {}) {
  return inspectRoot(options);
}

/** Hold the same exclusion as migration across inspection AND initial store.open. */
export function withDataRootStartup(open, { env = process.env, home = env?.HOME || os.homedir() } = {}) {
  const root = resolveDataRoot(env, home);
  fs.mkdirSync(path.dirname(root), { recursive: true, mode: 0o700 });
  const unlock = reserve(exclusionPath(root));
  try {
    const selected = inspectRoot({ env, home }, root);
    const result = open(selected);
    unlock.assertOwned();
    return result;
  } finally { unlock(); }
}

/** CLI-only lease: acquire under startup exclusion and retain until child close.
 * Pure launchSpec/SDK environment helpers intentionally do not acquire it.
 */
export function withStandaloneBridgeStartup(open, options = {}) {
  let release;
  try {
    return withDataRootStartup((root) => {
      release = reserve(bridgeLeasePath(root));
      return { result: open(root), release };
    }, options);
  } catch (error) { if (release) release(); throw error; }
}

function reserve(file) {
  // Refuse ALL existing locks, including apparently stale/malformed locks. Never
  // unlink somebody else's lock based on a PID check (PID reuse and unlink races).
  let fd;
  try { fd = fs.openSync(file, 'wx', 0o600); } catch (error) {
    if (error.code === 'EEXIST') fail(`Lock already exists: ${file}. Live, stale, malformed or unreadable locks require manual investigation.`);
    throw error;
  }
  const token = crypto.randomUUID();
  const text = `${JSON.stringify({ pid: process.pid, startedAt: new Date().toISOString(), migrationToken: token })}\n`;
  const reserved = fs.fstatSync(fd);
  try { fs.writeFileSync(fd, text); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
  const isOwned = () => {
    if (!exists(file)) return false;
    const current = fs.lstatSync(file);
    return current.isFile() && current.dev === reserved.dev && current.ino === reserved.ino
      && readRegular(file).toString('utf8') === text;
  };
  const assertOwned = () => {
    if (!isOwned()) fail(`Migration reservation changed: ${file}.`);
  };
  const unlock = () => {
    // Remove only the exact reservation we created, never a replacement.
    if (isOwned()) fs.unlinkSync(file);
  };
  unlock.assertOwned = assertOwned;
  return unlock;
}

function reserveDestination(root) {
  // mkdir (not recursive) is the no-clobber destination reservation.
  try { fs.mkdirSync(root, { mode: 0o700 }); } catch (error) {
    if (error.code === 'EEXIST') fail('Destination appeared during migration; refusing publication.');
    throw error;
  }
  const owned = new Map([['', fs.lstatSync(root)]]);
  const marker = Buffer.from(`${JSON.stringify({ version: 1, token: crypto.randomUUID(), identity: identity(root) })}\n`);
  const same = (relative, unchanged = false) => {
    const previous = owned.get(relative);
    if (!previous) return false;
    try {
      const current = fs.lstatSync(path.join(root, relative));
      return current.dev === previous.dev && current.ino === previous.ino
        && current.isDirectory() === previous.isDirectory()
        && (!unchanged || (current.size === previous.size && current.mtimeMs === previous.mtimeMs && current.ctimeMs === previous.ctimeMs));
    } catch { return false; }
  };
  function assertOwned(relative = '') {
    if (!same('')) fail('Destination reservation identity changed.');
    if (owned.has(INCOMPLETE_MARKER) && (!same(INCOMPLETE_MARKER, true)
      || !readRegular(path.join(root, INCOMPLETE_MARKER)).equals(marker))) fail('Destination incomplete marker changed.');
    let parent = path.dirname(relative);
    while (parent !== '.') {
      if (!same(parent)) fail('Destination parent reservation changed.');
      parent = path.dirname(parent);
    }
  }
  function write(relative, bytes) {
    assertOwned(relative);
    const file = path.join(root, relative);
    const fd = fs.openSync(file, 'wx', 0o600);
    try { fs.writeFileSync(fd, bytes); fs.fsyncSync(fd); }
    finally { owned.set(relative, fs.fstatSync(fd)); fs.closeSync(fd); }
    assertOwned(relative);
  }
  function cleanup() {
    // No recursive removal: replaced/modified files and nonempty directories
    // belong to somebody else (or require manual investigation).
    for (const [relative, stat] of [...owned].reverse()) {
      if (relative === '' || relative === INCOMPLETE_MARKER) continue;
      try {
        assertOwned(relative);
        if (!same(relative, !stat.isDirectory())) continue;
        if (stat.isDirectory()) fs.rmdirSync(path.join(root, relative));
        else fs.unlinkSync(path.join(root, relative));
      } catch { /* Retain incomplete artifacts rather than risk another writer. */ }
    }
    try {
      assertOwned();
      const remaining = fs.readdirSync(root);
      if (remaining.length === 1 && remaining[0] === INCOMPLETE_MARKER && same(INCOMPLETE_MARKER, true)) fs.unlinkSync(path.join(root, INCOMPLETE_MARKER));
      if (same('')) fs.rmdirSync(root); // refuses any foreign entry
    } catch { /* Leave marker-gated destination for manual investigation. */ }
  }
  try { write(INCOMPLETE_MARKER, marker); } catch (error) { cleanup(); throw error; }
  return {
    write, cleanup,
    copy(stage) {
      function copy(relative) {
        const file = path.join(stage, relative);
        const stat = fs.lstatSync(file);
        if (stat.isDirectory()) {
          if (relative) {
            assertOwned(relative);
            fs.mkdirSync(path.join(root, relative), { mode: 0o700 });
            owned.set(relative, fs.lstatSync(path.join(root, relative)));
          }
          for (const name of fs.readdirSync(file).sort()) copy(path.join(relative, name));
        } else if (stat.isFile()) write(relative, readRegular(file));
        else fail('Special staged file refused.');
      }
      copy('');
    },
    finish() {
      assertOwned();
      for (const [relative, stat] of owned) {
        if (!same(relative, !stat.isDirectory())) fail('Published artifact ownership changed.');
        if (stat.isDirectory()) {
          for (const name of fs.readdirSync(path.join(root, relative))) {
            if (!owned.has(path.join(relative, name))) fail('Unexpected entry in destination reservation.');
          }
        }
      }
      fs.unlinkSync(path.join(root, INCOMPLETE_MARKER));
    },
  };
}

function copyTree(source, stage) {
  function copy(relative) {
    if (relative === lockRelative) return;
    const from = path.join(source, relative);
    const to = path.join(stage, relative);
    const stat = fs.lstatSync(from);
    if (stat.isDirectory()) {
      if (relative) fs.mkdirSync(to, { mode: 0o700 });
      for (const name of fs.readdirSync(from).sort()) copy(path.join(relative, name));
    } else if (stat.isFile()) {
      fs.writeFileSync(to, readRegular(from), { flag: 'wx', mode: 0o600 });
    } else fail(`Symlink or special file refused: ${from}.`);
  }
  copy('');
}

function rebaseCopiedData(source, destination, stage) {
  const metadata = readState(stage);
  for (const task of metadata.tasks) {
    if (task.sessionFile === undefined || task.sessionFile === null) continue;
    const sessions = path.join(source, 'v1/profiles', task.profileId, 'pi-sessions');
    if (!path.isAbsolute(task.sessionFile) || !inside(sessions, task.sessionFile)) fail('Task sessionFile is outside its original profile pi-sessions.');
    const relative = path.relative(source, task.sessionFile);
    plain(path.join(stage, relative));
    task.sessionFile = path.join(destination, relative);
  }
  fs.writeFileSync(path.join(stage, 'v1/workbench.json'), `${JSON.stringify(metadata, null, 2)}\n`, { mode: 0o600 });
  // Only a Pi session's FIRST structural header can be rebased. Body records,
  // tool output, auth/config, transcripts and external provenance remain opaque.
  const profiles = path.join(stage, 'v1/profiles');
  if (!exists(profiles)) return;
  plain(profiles, true);
  function headers(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) { headers(file); continue; }
      if (!entry.name.endsWith('.jsonl')) continue;
      const bytes = readRegular(file);
      const end = bytes.indexOf(10);
      const first = bytes.subarray(0, end < 0 ? bytes.length : end).toString('utf8');
      let header;
      try { header = JSON.parse(first); } catch { continue; }
      if (header?.type !== 'session' || typeof header.parentSession !== 'string' || !path.isAbsolute(header.parentSession) || !inside(source, header.parentSession)) continue;
      const relative = path.relative(source, header.parentSession);
      plain(path.join(stage, relative));
      header.parentSession = path.join(destination, relative);
      fs.writeFileSync(file, Buffer.concat([Buffer.from(JSON.stringify(header)), end < 0 ? Buffer.alloc(0) : bytes.subarray(end)]));
    }
  }
  for (const profile of fs.readdirSync(profiles)) {
    if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(profile)) fail('Invalid profile directory ID.');
    plain(path.join(profiles, profile), true);
    const sessions = path.join(profiles, profile, 'pi-sessions');
    if (exists(sessions)) { plain(sessions, true); headers(sessions); }
  }
}

/** Caller attests unmanaged standalone clients are closed; no process shutdown. */
export function migrateDataRoot({ confirmClosed = false, home = os.homedir() } = {}) {
  if (confirmClosed !== true) fail('Migration requires --confirm-closed: ALL PAD and standalone Pi clients for this root must be closed.');
  const source = resolveLegacyDataRoot({}, home);
  const destination = resolveDataRoot({}, home);
  readState(source);
  if (exists(destination)) fail(`Destination already exists: ${destination}.`);
  fs.mkdirSync(path.dirname(destination), { recursive: true, mode: 0o700 });
  const release = [];
  let stage;
  let stageIdentity;
  let stageSnapshot;
  let publication;
  let completed = false;
  try {
    release.push(reserve(exclusionPath(destination)));
    release.push(reserve(exclusionPath(source)));
    if (exists(bridgeLeasePath(source)) || exists(bridgeLeasePath(destination))) fail('Standalone bridge lease exists; migration refused.');
    release.push(reserve(path.join(source, lockRelative)));
    const before = snapshot(source, { skipHostLock: true });
    stage = fs.mkdtempSync(path.join(path.dirname(destination), '.preview-migration-'));
    fs.chmodSync(stage, 0o700);
    stageIdentity = identity(stage);
    copyTree(source, stage);
    rebaseCopiedData(source, destination, stage);
    readState(stage);
    stageSnapshot = snapshot(stage).digest;
    if (before.digest !== snapshot(source, { skipHostLock: true }).digest) fail('Source changed during migration; no snapshot was published.');
    for (const unlock of release) unlock.assertOwned();
    if (before.digest !== snapshot(source, { skipHostLock: true }).digest) fail('Source changed before publication; no snapshot was published.');
    publication = reserveDestination(destination);
    const receipt = {
      version: 1, status: 'completed', source, destination,
      sourceIdentity: identity(source), destinationIdentity: identity(destination), sourceDigest: before.digest,
      completedAt: new Date().toISOString(),
    };
    // Never rename a directory over the destination. Every entry is exclusively
    // created inside our reservation; startup rejects the marker until complete.
    publication.copy(stage);
    for (const unlock of release) unlock.assertOwned();
    if (before.digest !== snapshot(source, { skipHostLock: true }).digest) fail('Source changed during publication.');
    publication.write(MIGRATION_RECEIPT, Buffer.from(`${JSON.stringify(receipt, null, 2)}\n`));
    publication.finish();
    completed = true;
    return receipt;
  } finally {
    if (publication && !completed) publication.cleanup();
    if (stage && stageSnapshot) {
      try {
        const current = identity(stage);
        if (current.dev === stageIdentity?.dev && current.ino === stageIdentity?.ino
          && snapshot(stage).digest === stageSnapshot) fs.rmSync(stage, { recursive: true });
      } catch { /* Changed/unverified staging artifacts require manual cleanup. */ }
    }
    for (const unlock of release.reverse()) unlock();
  }
}
