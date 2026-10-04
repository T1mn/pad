// Native workbench host logic: workspace/profile/task persistence, Pi task
// lifecycle, auth, and protocol command dispatch.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { launchSpec } from '../scripts/pi-session.mjs';
import { resolvePiPackage } from './host-env.mjs';
import { PiSdk, readSessionMessages } from './pi-sdk.mjs';
import { PiTaskRuntime } from './pi-task.mjs';
import { AuthManager } from './workbench-auth.mjs';
import { createAuthDiagnostics } from './auth-diagnostics.mjs';
import { createInstallationIdGetter } from './installation-id.mjs';
import { LocalSessions } from './local-sessions.mjs';
import { importLocalPi } from './local-session-import.mjs';
import { nativeProfileId, nowIso } from './workbench-store.mjs';

const SNAPSHOT_COALESCE_MS = 60;
const MAX_HISTORY_MESSAGES = 200;
const MAX_HISTORY_BYTES = 4 * 1024 * 1024;

export class CommandError extends Error {
  constructor(message) {
    super(message);
    this.name = 'CommandError';
  }
}

const clone = (value) => structuredClone(value);

function limitHistory(messages) {
  let list = messages;
  let truncated = false;
  if (list.length > MAX_HISTORY_MESSAGES) {
    list = list.slice(list.length - MAX_HISTORY_MESSAGES);
    truncated = true;
  }
  let bytes = Buffer.byteLength(JSON.stringify(list), 'utf8');
  while (list.length > 0 && bytes > MAX_HISTORY_BYTES) {
    list = list.slice(1);
    truncated = true;
    bytes = Buffer.byteLength(JSON.stringify(list), 'utf8');
  }
  return { messages: list, truncated };
}

export class WorkbenchHost {
  constructor({ root, store, emit, diagnostic }) {
    this.root = root;
    this.store = store;
    this.emit = emit;
    this.diagnostic = diagnostic;
    this.sdk = null;
    this.localSessions = new LocalSessions(); // No filesystem scan until an explicit UI request.
    this.profileDirsCache = new Map();
    this.activeRuns = new Set();
    this.catalogRefreshes = new Map();
    // Per-task send reservation; its token cancels a prompt/set_model that has
    // not reached its Pi RPC yet.
    this.taskOps = new Map();
    this.snapshotTimer = null;
    this.closed = false;
    this.auth = new AuthManager({
      requireProfile: (profileId) => this.requireProfile(profileId),
      isClosed: () => this.closed,
      profileBusy: (profileId) => this.profileBusy(profileId),
      agentDirFor: (profileId) => this.profileDirs(profileId).agent,
      sdkClient: () => this.sdkClient(),
      emit: (frame) => this.emit(frame),
      diagnostic: (message) => this.diagnostic(message),
      recordAuthDiagnostic: createAuthDiagnostics({ dataRoot: this.store.root }),
      makeError: (message) => new CommandError(message),
      onCredentialsChanged: async (profileId) => {
        this.sdk?.invalidateOpenAI?.(this.profileDirs(profileId).agent);
        await this.stopProfilePeers(profileId);
      },
    });
    this.runtime = new PiTaskRuntime({
      createLaunch: (task) => this.createLaunch(task),
      onPiEvent: (taskId, message) => this.onPiEvent(taskId, message),
      onExit: (taskId, info) => this.onTaskExit(taskId, info),
      onDiagnostic: (taskId, message) => this.diagnostic(`[pi ${taskId}] ${message}`),
    });
  }

  // ---------------------------------------------------------------- lookups

  findTask(taskId) {
    return this.store.state.tasks.find((task) => task.id === taskId);
  }

  requireWorkspace(workspaceId) {
    if (typeof workspaceId !== 'string' || workspaceId.length === 0) throw new CommandError('workspaceId is required');
    const workspace = this.store.state.workspaces.find((entry) => entry.id === workspaceId);
    if (!workspace) throw new CommandError(`Unknown workspace: ${workspaceId}`);
    return workspace;
  }

  requireProfile(profileId) {
    if (typeof profileId !== 'string' || profileId.length === 0) throw new CommandError('profileId is required');
    const profile = this.store.state.profiles.find((entry) => entry.id === profileId);
    if (!profile) throw new CommandError(`Unknown profile: ${profileId}`);
    return profile;
  }

  requireTask(taskId) {
    if (typeof taskId !== 'string' || taskId.length === 0) throw new CommandError('taskId is required');
    const task = this.findTask(taskId);
    if (!task) throw new CommandError(`Unknown task: ${taskId}`);
    return task;
  }

  sdkClient() {
    if (!this.sdk) {
      const { packageRoot } = resolvePiPackage(process.env);
      this.sdk = new PiSdk({ packageRoot, getDeviceId: createInstallationIdGetter({ dataRoot: this.store.root }) });
    }
    return this.sdk;
  }

  /** Per-profile Pi agent/session paths, derived from the shared launchSpec. */
  profileDirs(profileId) {
    const cached = this.profileDirsCache.get(profileId);
    if (cached) return cached;
    const spec = launchSpec({ profile: profileId, dataRoot: this.root, cwd: this.root, source: process.env });
    const dirs = { agent: spec.agent, sessions: spec.sessions, cli: spec.cli, env: spec.env };
    this.profileDirsCache.set(profileId, dirs);
    return dirs;
  }

  /**
   * Realpath-resolved containment. Resolves symlinks in the sessions directory
   * and in the candidate file (or its parent when it does not exist yet), so a
   * symlink inside the directory cannot escape it. Returns the resolved path or
   * null when the candidate is outside the directory.
   */
  resolveContained(file, sessionsDir) {
    if (typeof file !== 'string' || file.length === 0) return null;
    let base;
    try {
      base = fs.realpathSync(sessionsDir);
    } catch {
      return null;
    }
    const candidate = path.resolve(file);
    let resolved;
    try {
      resolved = fs.realpathSync(candidate);
    } catch {
      let parent;
      try {
        parent = fs.realpathSync(path.dirname(candidate));
      } catch {
        return null;
      }
      resolved = path.join(parent, path.basename(candidate));
    }
    return resolved === base || resolved.startsWith(`${base}${path.sep}`) ? resolved : null;
  }

  /** The task's own session file, fixed by task id so restarts keep history. */
  taskSessionPath(task, dirs = this.profileDirs(task.profileId)) {
    return this.resolveContained(path.join(dirs.sessions, `${task.id}.jsonl`), dirs.sessions);
  }

  // -------------------------------------------------------------- snapshots

  snapshot() {
    const state = this.store.state;
    return {
      workspaces: clone(state.workspaces),
      profiles: clone(state.profiles),
      tasks: clone(state.tasks),
    };
  }

  scheduleSnapshot() {
    if (this.closed || this.snapshotTimer) return;
    this.snapshotTimer = setTimeout(() => {
      this.snapshotTimer = null;
      this.emit({ type: 'event', event: 'snapshot', data: this.snapshot() });
    }, SNAPSHOT_COALESCE_MS);
    this.snapshotTimer.unref?.();
  }

  mutateTask(taskId, patch) {
    const updated = this.store.mutate((draft) => {
      const task = draft.tasks.find((entry) => entry.id === taskId);
      if (!task) throw new CommandError(`Unknown task: ${taskId}`);
      Object.assign(task, patch);
      task.updatedAt = nowIso();
      return clone(task);
    });
    this.scheduleSnapshot();
    return updated;
  }

  setTaskStatus(taskId, status) {
    const task = this.findTask(taskId);
    if (!task || task.status === status) return;
    this.mutateTask(taskId, { status });
  }

  profileBusy(profileId) {
    return this.store.state.tasks.some((task) => task.profileId === profileId
      && (this.activeRuns.has(task.id) || this.taskOps.has(task.id)
        || task.status === 'starting' || task.status === 'running'));
  }

  /** Reserve a task synchronously so concurrent sends cannot both pass checks. */
  beginTaskOp(taskId, kind) {
    const token = { kind, cancelled: false };
    this.taskOps.set(taskId, token);
    return token;
  }

  endTaskOp(taskId, token) {
    if (this.taskOps.get(taskId) === token) this.taskOps.delete(taskId);
  }

  /** Checked after every await between reserving and the actual Pi RPC. */
  checkTaskOp(token) {
    if (token.cancelled) throw new CommandError('The request was cancelled');
    if (this.closed) throw new CommandError('The host is shutting down');
  }

  // ------------------------------------------------------------ Pi lifecycle

  createLaunch(task) {
    const profile = this.requireProfile(task.profileId);
    const workspace = this.requireWorkspace(task.workspaceId);
    const dirs = this.profileDirs(profile.id);
    const args = [
      '--mode', 'rpc',
      '--session-dir', dirs.sessions,
      '--name', 'PAD Terminal Preview',
      '--offline', '--no-approve', '--no-extensions', '--no-skills', '--no-prompt-templates', '--no-themes',
    ];
    const pinned = this.taskSessionPath(task, dirs);
    if (typeof task.sessionFile === 'string' && task.sessionFile.length > 0) {
      const contained = this.resolveContained(task.sessionFile, dirs.sessions);
      if (!contained) throw new CommandError('Task session file is outside its profile session directory');
      if (!fs.existsSync(contained) && contained !== pinned) {
        throw new CommandError('The saved session for this task is missing; start a new task instead');
      }
      args.push('--session', contained);
    } else if (pinned) {
      // First launch: pin this task's own session path so a restart keeps it.
      args.push('--session', pinned);
    }
    if (typeof task.provider === 'string' && typeof task.modelId === 'string') {
      args.push('--provider', task.provider, '--model', task.modelId);
    }
    return { command: process.execPath, args: [dirs.cli, ...args], cwd: path.resolve(workspace.path), env: dirs.env };
  }

  async ensureTaskProcess(task) {
    const process = await this.runtime.ensure(task);
    const sessionFile = process.state?.sessionFile;
    if (typeof sessionFile === 'string' && sessionFile.length > 0) {
      const dirs = this.profileDirs(task.profileId);
      const contained = this.resolveContained(sessionFile, dirs.sessions);
      if (!contained) {
        this.diagnostic(`[pi ${task.id}] ignored a session file outside the profile session directory`);
      } else if (contained !== task.sessionFile) {
        this.mutateTask(task.id, { sessionFile: contained });
      }
    }
    if (task.status === 'error') this.setTaskStatus(task.id, 'idle');
    return process;
  }

  onPiEvent(taskId, message) {
    this.emit({ type: 'event', event: 'pi', taskId, data: message });
    const task = this.findTask(taskId);
    if (!task || this.closed) return;
    switch (message?.type) {
      case 'agent_start':
      case 'turn_start':
      case 'message_start':
      case 'tool_execution_start':
        this.activeRuns.add(taskId);
        this.setTaskStatus(taskId, 'running');
        break;
      case 'agent_settled':
        this.activeRuns.delete(taskId);
        if (task.status !== 'error') this.setTaskStatus(taskId, 'idle');
        break;
      default:
        break;
    }
  }

  onTaskExit(taskId, info) {
    this.activeRuns.delete(taskId);
    this.taskOps.delete(taskId);
    const task = this.findTask(taskId);
    if (!task || this.closed) return;
    const failed = info.error !== null || (typeof info.code === 'number' && info.code !== 0);
    this.setTaskStatus(taskId, failed ? 'error' : 'idle');
    if (info.error) this.diagnostic(`[pi ${taskId}] exited: ${info.error.message}`);
  }

  // --------------------------------------------------------------- commands

  async execute(command, message) {
    switch (command) {
      case 'snapshot':
        return this.snapshot();
      case 'add_workspace':
        return this.addWorkspace(message.path);
      case 'add_profile':
        return this.addProfile(message.name);
      case 'create_task':
        return this.createTask(message);
      case 'catalog':
        return this.catalog(message.profileId, message.refreshOpenAI === true);
      case 'history':
        return this.history(message.taskId);
      case 'local_sessions':
        return this.localSessions.list();
      case 'local_session_history':
        return this.localSessions.history(message.sessionId);
      case 'local_session_resume':
        return this.localSessions.resume(message.sessionId);
      case 'local_session_import_pi':
        return importLocalPi(this, message.sessionId, message.profileId);
      case 'set_model':
        return this.setModel(message.taskId, message.provider, message.modelId);
      case 'prompt':
        return this.prompt(message.taskId, message.message);
      case 'abort':
        return this.abort(message.taskId);
      case 'auth_begin':
        return this.authBegin(message.profileId, message.provider, message.method, message.attemptId);
      case 'auth_respond':
        return this.authRespond(message.profileId, message.promptId, message.value);
      case 'auth_cancel':
        return this.authCancel(message.profileId, message.attemptId);
      case 'logout':
        return this.logout(message.profileId, message.provider);
      default:
        throw new CommandError(`Unknown command: ${String(command)}`);
    }
  }

  addWorkspace(rawPath) {
    if (typeof rawPath !== 'string' || rawPath.trim().length === 0) throw new CommandError('Workspace path is required');
    const candidate = rawPath.trim();
    if (!path.isAbsolute(candidate)) throw new CommandError('Workspace path must be absolute');
    let resolved;
    try {
      resolved = fs.realpathSync(candidate);
    } catch {
      throw new CommandError('Workspace directory does not exist');
    }
    let stats;
    try {
      stats = fs.statSync(resolved);
    } catch {
      throw new CommandError('Workspace directory is unavailable');
    }
    if (!stats.isDirectory()) throw new CommandError('Workspace path must be an existing directory');
    const existing = this.store.state.workspaces.find((workspace) => workspace.path === resolved);
    if (existing) return clone(existing);
    const workspace = {
      id: `ws-${crypto.randomUUID().replace(/-/g, '').slice(0, 8)}`,
      name: path.basename(resolved) || resolved,
      path: resolved,
    };
    const created = this.store.mutate((draft) => {
      if (draft.workspaces.some((entry) => entry.path === resolved)) {
        return clone(draft.workspaces.find((entry) => entry.path === resolved));
      }
      draft.workspaces.push(workspace);
      return clone(workspace);
    });
    this.scheduleSnapshot();
    return created;
  }

  addProfile(rawName) {
    if (typeof rawName !== 'string' || rawName.trim().length === 0) throw new CommandError('Profile name is required');
    const profile = {
      id: nativeProfileId(),
      name: rawName.trim().slice(0, 80),
    };
    const created = this.store.mutate((draft) => {
      draft.profiles.push(profile);
      return clone(profile);
    });
    this.scheduleSnapshot();
    return created;
  }

  createTask({ workspaceId, profileId, title }) {
    const workspace = this.requireWorkspace(workspaceId);
    const profile = this.requireProfile(profileId);
    const task = {
      id: `task-${crypto.randomUUID().replace(/-/g, '').slice(0, 8)}`,
      workspaceId: workspace.id,
      profileId: profile.id,
      title: typeof title === 'string' && title.trim().length > 0 ? title.trim().slice(0, 120) : workspace.name,
      status: 'idle',
      updatedAt: nowIso(),
    };
    const created = this.store.mutate((draft) => {
      draft.tasks.push(task);
      return clone(task);
    });
    this.scheduleSnapshot();
    return created;
  }

  async catalog(profileId, refreshOpenAI = false) {
    const profile = this.requireProfile(profileId);
    if (this.closed) throw new CommandError('The host is shutting down');
    if (refreshOpenAI && this.catalogRefreshes.has(profile.id)) return this.catalogRefreshes.get(profile.id);
    const agentDir = this.profileDirs(profile.id).agent;
    if (refreshOpenAI) {
      // Reuse the existing synchronous profile auth/task guard; always release
      // it in finally, including bounded discovery failures. No Pi RPC starts.
      this.auth.reserveLogout(profile.id);
    }
    const pending = (async () => {
      try {
        const catalog = await this.sdkClient().catalog(agentDir, { refreshOpenAI });
        const { providers, models, openaiDiscovery } = catalog;
        // Browsing during login/logout must not expose the preceding account.
        const changingAuth = this.auth.isAuthBusy(profile.id) && !this.catalogRefreshes.has(profile.id) && !refreshOpenAI;
        return {
          profileId: profile.id, providers,
          models: changingAuth ? models.filter((m) => m.provider !== 'openai') : models,
          ...(openaiDiscovery ? { openaiDiscovery: changingAuth ? { state: 'not_loaded' } : openaiDiscovery } : {}),
        };
      } catch {
        throw new CommandError('Provider catalog is unavailable. Check the Pi installation and profile configuration.');
      } finally {
        if (refreshOpenAI) {
          this.catalogRefreshes.delete(profile.id);
          this.auth.releaseLogout(profile.id);
        }
      }
    })();
    if (refreshOpenAI) this.catalogRefreshes.set(profile.id, pending);
    return pending;
  }

  async history(taskId) {
    const task = this.requireTask(taskId);
    let messages = [];
    const live = this.runtime.get(task.id);
    if (live) {
      const data = await live.request({ type: 'get_messages' });
      messages = Array.isArray(data.messages) ? data.messages : [];
    } else {
      // Browsing must not leave an idle Pi process behind; read the active
      // branch offline instead.
      messages = await this.readTaskHistory(task);
    }
    const limited = limitHistory(messages);
    return limited.truncated ? { messages: limited.messages, truncated: true } : { messages: limited.messages };
  }

  /**
   * Offline history: prefer the SDK SessionManager active branch, falling back
   * to a read-only parentId walk. A recorded session outside the profile dir is
   * refused, and a recorded session that is missing (and is not this task's own
   * pinned path) fails instead of silently reporting an empty session.
   */
  async readTaskHistory(task) {
    if (typeof task.sessionFile !== 'string' || task.sessionFile.length === 0) return [];
    let dirs;
    try {
      dirs = this.profileDirs(task.profileId);
    } catch {
      return [];
    }
    const contained = this.resolveContained(task.sessionFile, dirs.sessions);
    if (!contained) {
      this.diagnostic(`[pi ${task.id}] refused to read a session outside its profile directory`);
      throw new CommandError('The saved session for this task is outside its profile session directory');
    }
    if (!fs.existsSync(contained)) {
      if (contained === this.taskSessionPath(task, dirs)) return [];
      throw new CommandError('The saved session for this task is missing');
    }
    try {
      return await this.sdkClient().sessionMessages(contained);
    } catch (error) {
      this.diagnostic(`[pi ${task.id}] session read unavailable`);
      return readSessionMessages(contained);
    }
  }

  async setModel(taskId, provider, modelId) {
    const task = this.requireTask(taskId);
    if (typeof provider !== 'string' || provider.trim().length === 0) throw new CommandError('provider is required');
    if (typeof modelId !== 'string' || modelId.trim().length === 0) throw new CommandError('modelId is required');
    if (this.activeRuns.has(task.id) || this.taskOps.has(task.id) || task.status === 'starting') {
      throw new CommandError('Cannot change the model while the task is running');
    }
    if (this.auth.isAuthBusy(task.profileId)) throw new CommandError('Cannot change the model while authentication is in progress');
    if (this.closed) throw new CommandError('The host is shutting down');
    const nextProvider = provider.trim();
    const nextModel = modelId.trim();
    // Reserve synchronously so a prompt cannot swap the model underneath us.
    const token = this.beginTaskOp(task.id, 'set_model');
    try {
      const agentDir = this.profileDirs(task.profileId).agent;
      let available;
      try { available = await this.sdkClient().modelAvailable(agentDir, nextProvider, nextModel); } catch {
        throw new CommandError('Model availability could not be checked. Check the profile configuration.');
      }
      this.checkTaskOp(token);
      if (!available) throw new CommandError(`Model ${nextProvider}/${nextModel} is not available in this profile's catalog`);
      const process = this.runtime.get(task.id);
      if (process) {
        await process.request({ type: 'set_model', provider: nextProvider, modelId: nextModel });
        this.checkTaskOp(token);
        process.currentProvider = nextProvider;
        process.currentModel = nextModel;
      }
      return this.mutateTask(task.id, { provider: nextProvider, modelId: nextModel });
    } finally {
      this.endTaskOp(task.id, token);
    }
  }

  async prompt(taskId, message) {
    const task = this.requireTask(taskId);
    if (typeof message !== 'string' || message.trim().length === 0) throw new CommandError('Prompt message must not be empty');
    if (this.activeRuns.has(task.id) || this.taskOps.has(task.id) || task.status === 'starting') {
      throw new CommandError('This task already has an active prompt');
    }
    if (this.auth.isAuthBusy(task.profileId)) throw new CommandError('Cannot send a prompt while authentication is in progress');
    if (this.closed) throw new CommandError('The host is shutting down');
    if (typeof task.provider !== 'string' || typeof task.modelId !== 'string') throw new CommandError('Select a model before sending a prompt');
    // Reserve synchronously so two concurrent prompts cannot both pass validation.
    const token = this.beginTaskOp(task.id, 'prompt');
    try {
      const agentDir = this.profileDirs(task.profileId).agent;
      let available;
      try { available = await this.sdkClient().modelAvailable(agentDir, task.provider, task.modelId); } catch {
        throw new CommandError('Model availability could not be checked. Check the profile configuration.');
      }
      this.checkTaskOp(token);
      if (!available) throw new CommandError(`Model ${task.provider}/${task.modelId} has no credentials in this profile`);
      const process = await this.ensureTaskProcess(task);
      this.checkTaskOp(token);
      if (process.currentProvider !== task.provider || process.currentModel !== task.modelId) {
        await process.request({ type: 'set_model', provider: task.provider, modelId: task.modelId });
        this.checkTaskOp(token);
        process.currentProvider = task.provider;
        process.currentModel = task.modelId;
      }
      // No await between this check and the request, so a concurrent abort can
      // never let a paid prompt slip out after it returned.
      this.checkTaskOp(token);
      this.activeRuns.add(task.id);
      this.setTaskStatus(task.id, 'running');
      let data;
      try {
        data = await process.request({ type: 'prompt', message });
      } catch (error) {
        this.activeRuns.delete(task.id);
        this.setTaskStatus(task.id, 'idle');
        throw new CommandError(error instanceof Error ? error.message : String(error));
      }
      const disposition = typeof data.disposition === 'string' ? data.disposition : 'started';
      if (disposition === 'handled') {
        this.activeRuns.delete(task.id);
        this.setTaskStatus(task.id, 'idle');
      }
      return { disposition };
    } finally {
      this.endTaskOp(task.id, token);
    }
  }

  async abort(taskId) {
    const task = this.requireTask(taskId);
    // Synchronously cancel any in-flight send that has not reached its RPC yet.
    const token = this.taskOps.get(task.id);
    if (token) token.cancelled = true;
    const process = this.runtime.get(task.id);
    if (!process) {
      if (!token && !this.activeRuns.has(task.id) && task.status !== 'error') this.setTaskStatus(task.id, 'idle');
      return {};
    }
    try {
      await process.request({ type: 'clear_queue' }, 5_000);
    } catch {
      // Nothing queued or the process is already idle.
    }
    let abortConfirmed = false;
    try {
      await process.request({ type: 'abort' }, 30_000);
      abortConfirmed = true;
    } catch (error) {
      this.diagnostic(`[pi ${task.id}] abort failed: ${error instanceof Error ? error.message : String(error)}`);
    }
    this.activeRuns.delete(task.id);
    if (abortConfirmed) {
      if (task.status !== 'error') this.setTaskStatus(task.id, 'idle');
      return {};
    }
    // Fallback: we own this child, so stop it rather than pretend the task is
    // idle while the run may still be going. Surface an explicit error.
    try {
      await this.runtime.stop(task.id);
    } catch (stopError) {
      this.setTaskStatus(task.id, 'error');
      throw new CommandError(`Abort failed and the Pi process could not be stopped: ${stopError instanceof Error ? stopError.message : String(stopError)}`);
    }
    this.setTaskStatus(task.id, 'error');
    throw new CommandError('Abort could not be confirmed; the Pi process was stopped');
  }

  // ------------------------------------------------------------------- auth

  authBegin(profileId, provider, method, attemptId) {
    return this.auth.begin(profileId, provider, method, attemptId);
  }

  authRespond(profileId, promptId, value) {
    return this.auth.respond(profileId, promptId, value);
  }

  authCancel(profileId, attemptId) {
    return this.auth.cancel(profileId, attemptId);
  }

  /** Stop this profile's idle Pi peers so a restart reloads the new credentials. */
  async stopProfilePeers(profileId) {
    for (const task of this.store.state.tasks) {
      if (task.profileId !== profileId) continue;
      if (this.activeRuns.has(task.id) || this.taskOps.has(task.id)) continue;
      if (task.status === 'running' || task.status === 'starting') continue;
      try {
        await this.runtime.stop(task.id);
      } catch {
        throw new CommandError('Credential cleanup failed. Restart PAD before using this profile.');
      }
    }
  }

  async logout(profileId, provider) {
    const profile = this.requireProfile(profileId);
    if (typeof provider !== 'string' || provider.trim().length === 0) throw new CommandError('provider is required');
    // Reserve synchronously so a prompt cannot start against the old credential.
    this.auth.reserveLogout(profile.id);
    let cleanupPending = false;
    try {
      const agentDir = this.profileDirs(profile.id).agent;
      try {
        await this.sdkClient().logout({ agentDir, providerId: provider.trim(), signal: new AbortController().signal });
      } catch {
        throw new CommandError('Unable to sign out of this provider');
      }
      cleanupPending = true;
      await this.stopProfilePeers(profile.id);
      cleanupPending = false;
      return {};
    } finally {
      // A failed cleanup leaves old peers alive: keep sends blocked until restart.
      if (!cleanupPending) this.auth.releaseLogout(profile.id);
    }
  }

  // ---------------------------------------------------------------- shutdown

  async close() {
    if (this.closed) return;
    this.closed = true;
    if (this.snapshotTimer) {
      clearTimeout(this.snapshotTimer);
      this.snapshotTimer = null;
    }
    for (const token of this.taskOps.values()) token.cancelled = true;
    this.auth.close();
    this.sdk?.close?.();
    await this.runtime.stopAll();
    this.store.releaseLock();
  }
}
