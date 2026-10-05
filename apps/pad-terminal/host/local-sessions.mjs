// Explicit, read-only local-session discovery. Never reads credentials/settings,
// invokes an agent, writes a source file, or sends session content to a model.
import fs from 'node:fs/promises';
import { constants } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { parseLines, sessionMetadata, sessionMessages } from './local-session-formats.mjs';

const HEAD_BYTES = 64 * 1024, HISTORY_BYTES = 8 * 1024 * 1024;
const MAX_FILES = 5000, MAX_VISITS = 30000, MAX_RESULTS = 2000;
const inside = (root, file) => file.startsWith(root + path.sep);
const publicSession = ({ root, fingerprint, rawCwd, ...session }) => session;
async function canonicalCwd(cwd) {
  if (!cwd) return '';
  try { return await fs.realpath(cwd); } catch { return cwd; } // Missing projects remain browsable.
}

export class LocalSessions {
  constructor({ home = os.homedir() } = {}) {
    this.home = home;
    this.roots = [
      { tool: 'codex', path: path.join(home, '.codex/sessions') },
      { tool: 'pi', path: path.join(home, '.pi/agent/sessions') },
    ];
    this.records = new Map();
    this.scanning = null;
  }

  // O_NOFOLLOW plus canonical containment for every read, not only discovery.
  async open(record) {
    const file = await fs.realpath(record.file);
    if (!inside(record.root, file) || file !== record.file) throw new Error('本地会话路径已改变，请刷新列表');
    const handle = await fs.open(file, constants.O_RDONLY | constants.O_NOFOLLOW);
    const stat = await handle.stat();
    if (!stat.isFile()) { await handle.close(); throw new Error('不是会话文件'); }
    return { handle, stat };
  }

  async sample(record, history = false) {
    const { handle, stat } = await this.open(record);
    try {
      const count = Math.min(stat.size, history ? HISTORY_BYTES : HEAD_BYTES);
      const head = Buffer.alloc(history ? 0 : count);
      if (head.length) await handle.read(head, 0, head.length, 0);
      const offset = Math.max(0, stat.size - count);
      const tail = Buffer.alloc(count);
      await handle.read(tail, 0, count, offset);
      let ending = tail.toString('utf8');
      if (offset > 0) ending = ending.slice(ending.indexOf('\n') + 1);
      const text = !history && offset > 0 ? `${head.toString('utf8')}\n${ending}` : ending;
      return { entries: parseLines(text), stat, truncated: stat.size > count };
    } finally { await handle.close(); }
  }

  async list() {
    if (this.scanning) return this.scanning;
    this.scanning = this.scan();
    try { return await this.scanning; } finally { this.scanning = null; }
  }

  async scan() {
    const records = new Map(), roots = [];
    let visited = 0, truncated = false;
    for (const source of this.roots) {
      let root;
      try { root = await fs.realpath(source.path); }
      catch (error) { roots.push({ ...source, status: error.code === 'ENOENT' ? 'missing' : 'unreadable' }); continue; }
      const state = { ...source, status: 'ready' }; roots.push(state);
      const queue = [{ dir: root, depth: 0 }];
      while (queue.length && records.size < MAX_FILES && visited < MAX_VISITS) {
        const { dir, depth } = queue.shift();
        try {
          const directory = await fs.opendir(dir);
          for await (const entry of directory) {
            if (++visited > MAX_VISITS || records.size >= MAX_FILES) { truncated = true; break; }
            const file = path.join(dir, entry.name);
            if (entry.isSymbolicLink()) continue;
            if (entry.isDirectory() && depth < 8) {
              if (entry.name !== 'subagents') queue.push({ dir: file, depth: depth + 1 });
              continue;
            }
            if (!entry.isFile() || !entry.name.endsWith('.jsonl')) continue;
            try {
              const stat = await fs.stat(file);
              const fingerprint = `${stat.ino}:${stat.size}:${stat.mtimeMs}`;
              const id = crypto.createHash('sha256').update(`${source.tool}:${file}`).digest('hex');
              const cached = this.records.get(id);
              if (cached?.fingerprint === fingerprint) {
                // Project availability/symlink targets can change without a session-file write.
                records.set(id, { ...cached, cwd: (await canonicalCwd(cached.rawCwd)).slice(0, 1024) });
                continue;
              }
              const sample = await this.sample({ root, file });
              const meta = sessionMetadata(source.tool, sample.entries);
              if (!meta.sessionId) {
                if (sample.stat.size > HEAD_BYTES) state.status = 'partial';
                continue;
              }
              records.set(id, {
                id, tool: source.tool, ...meta, rawCwd: meta.cwd, cwd: (await canonicalCwd(meta.cwd)).slice(0, 1024),
                title: meta.title || `${source.tool} · ${meta.sessionId.slice(0, 12)}`,
                file, root, fingerprint,
                updatedAt: sample.stat.mtime.toISOString(),
              });
            } catch { state.status = 'partial'; }
          }
        } catch { state.status = 'partial'; }
      }
      if (queue.length) truncated = true;
    }
    this.records = records;
    const sessions = [...records.values()].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
    const result = []; let bytes = 0;
    for (const record of sessions.slice(0, MAX_RESULTS)) {
      const value = publicSession(record);
      bytes += Buffer.byteLength(JSON.stringify(value));
      if (bytes > 4 * 1024 * 1024) { truncated = true; break; }
      result.push(value);
    }
    return { sessions: result, roots,
      truncated: truncated || sessions.length > MAX_RESULTS, scannedAt: new Date().toISOString() };
  }

  require(id) {
    const record = this.records.get(id);
    if (!record) throw new Error('本地会话已不存在或列表过期，请刷新后重试');
    if (!['codex', 'pi'].includes(record.tool)) throw new Error('本地会话仅支持 Codex / Pi，请刷新列表');
    return record;
  }

  async history(id) {
    const record = this.require(id);
    const sample = await this.sample(record, true);
    const all = sessionMessages(record.tool, sample.entries);
    const messages = all.slice(-200);
    let bytes = messages.reduce((sum, m) => sum + Buffer.byteLength(JSON.stringify(m)), 0);
    while (bytes > 4 * 1024 * 1024 && messages.length) bytes -= Buffer.byteLength(JSON.stringify(messages.shift()));
    return { messages, truncated: sample.truncated || messages.length < all.length };
  }

  async resume(id) {
    const record = this.require(id);
    const { entries } = await this.sample(record);
    const meta = sessionMetadata(record.tool, entries);
    if (meta.sessionId !== record.sessionId || meta.cwd !== record.rawCwd || await canonicalCwd(meta.cwd) !== record.cwd) throw new Error('会话标识或项目已改变，请刷新后重试');
    if (record.tool !== 'pi' && [...this.records.values()].filter((r) => r.tool === record.tool && r.sessionId === record.sessionId).length > 1) {
      throw new Error('同一会话标识存在多份记录，请使用原工具的会话选择器，避免续接错误副本');
    }
    if (!record.cwd || !(await fs.stat(record.cwd)).isDirectory()) throw new Error('原项目目录已不存在，无法安全续接');
    if (record.tool !== 'pi' && !/^[a-f\d]{8}(-[a-f\d]{4}){3}-[a-f\d]{12}$/i.test(record.sessionId)) {
      throw new Error('此会话标识不支持直接续接，请使用原工具的会话选择器');
    }
    // Resolve only known CLI names in standard user executable locations.
    const directories = [path.join(this.home, '.local/bin'), '/opt/homebrew/bin', '/usr/local/bin',
      path.join(this.home, '.npm-global/bin'), path.join(this.home, '.cargo/bin')];
    let executable;
    for (const directory of directories) {
      const candidate = path.join(directory, record.tool);
      try { await fs.access(candidate, constants.X_OK); if ((await fs.stat(candidate)).isFile()) { executable = candidate; break; } } catch { /* next location */ }
    }
    if (!executable) throw new Error(`未找到 ${record.tool} CLI，请先安装原工具或从普通终端使用其 resume 命令`);
    const args = record.tool === 'codex' ? ['resume', record.sessionId] : ['--session', record.file];
    return { cwd: record.cwd, executable, args };
  }

  async piCopy(id) {
    const record = this.require(id);
    if (record.tool !== 'pi') throw new Error('只有 Pi 会话可导入原生 Pi 面板；Codex 请使用原工具续接');
    const { handle, stat } = await this.open(record);
    try {
      if (stat.size > 32 * 1024 * 1024) throw new Error('此会话超过 32 MiB，请用原 Pi 续接，避免不完整导入');
      const buffer = Buffer.alloc(stat.size);
      let bytes = 0;
      while (bytes < buffer.length) {
        const read = await handle.read(buffer, bytes, buffer.length - bytes, bytes);
        if (!read.bytesRead) throw new Error('会话正在变化，请停止原会话后重试');
        bytes += read.bytesRead;
      }
      const after = await handle.stat();
      if (after.size !== stat.size || after.mtimeMs !== stat.mtimeMs) throw new Error('原会话仍在写入，请停止后再导入副本');
      const lines = buffer.toString('utf8').split('\n').filter((line) => line.trim());
      let entries;
      try { entries = lines.map((line) => JSON.parse(line)); }
      catch { throw new Error('会话存在未完成记录，请等待原工具写入完成后再导入'); }
      const header = entries[0];
      if (header?.type !== 'session' || header.id !== record.sessionId) throw new Error('Pi 会话头无效或已改变');
      if (![1, 2, 3].includes(header.version ?? 1)) throw new Error('此 Pi 会话格式较新，请先使用原 Pi 续接');
      header.id = crypto.randomUUID();
      header.parentSession = record.file;
      return { record: publicSession(record), text: entries.map((entry) => JSON.stringify(entry)).join('\n') + '\n' };
    } finally { await handle.close(); }
  }
}
