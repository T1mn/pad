import { randomUUID } from 'node:crypto';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { EventEmitter } from 'node:events';
import {
  DESKTOP_MAX_FRAME_BYTES,
  DESKTOP_PROTOCOL_VERSION,
  type AuthResultDto,
  type AuthType,
  type DesktopBootstrapResult,
  type DesktopEvent,
  type DesktopHelloResult,
  type DesktopServerEvent,
  type DesktopServerEventKind,
  type DesktopUiStateDto,
  type PermissionMode,
  type TaskEnvironment,
  type TerminalAcceptedDto,
  type TerminalCloseDto,
  type TerminalOpenDto,
  type TerminalSnapshotDto,
} from '../../shared/protocol';
import { LocalStore, publicProfile, publicTask, type StoredProfile, type StoredTask } from './local-store';
import { PiRuntime } from './pi-runtime';
import { AuthCoordinator, authenticatedProviders, modelCatalog } from './pi-sdk';
import { RemoteGateway } from './remote-gateway';
import packageMetadata from '../../package.json';
import type { RuntimeProcessLauncher } from './runtime-process';

export interface LocalBackendOptions {
  dataRoot: string;
  resourcesPath: string;
  environment?: NodeJS.ProcessEnv;
  runtimeLauncher?: RuntimeProcessLauncher;
}

interface TerminalPane {
  id: string;
  taskId: string;
  epoch: number;
  revision: number;
  columns: number;
  rows: number;
  status: TerminalSnapshotDto['status'];
  lines: string[];
  child: ChildProcessWithoutNullStreams;
  exit: TerminalSnapshotDto['exit'];
}

const CAPABILITIES = [
  'profiles', 'projects', 'tasks', 'codex_sidebar', 'pi_rpc', 'pi_fast_mode',
  'full_access_policy', 'private_store', 'history', 'provider_status', 'model_catalog',
  'extension_ui_response', 'set_model', 'set_thinking_level', 'create_project', 'stop',
  'desktop_ui_state_v1', 'terminal_v1', 'auth_control_plane_v1',
  'remote_gateway_v1', 'remote_pairing', 'remote_device_management',
  'cross_session_v1', 'session_rename', 'subagents_v1',
];

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function requiredString(fields: Record<string, unknown>, key: string): string {
  const value = fields[key];
  if (typeof value !== 'string' || !value.trim()) throw new Error(`Missing ${key}`);
  return value;
}

function optionalString(fields: Record<string, unknown>, key: string): string | undefined {
  const value = fields[key];
  return typeof value === 'string' ? value : undefined;
}

function optionalBoolean(fields: Record<string, unknown>, key: string): boolean | undefined {
  return typeof fields[key] === 'boolean' ? fields[key] : undefined;
}

function optionalNumber(fields: Record<string, unknown>, key: string): number | undefined {
  const value = fields[key];
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

function messageText(value: unknown): { role: string; text: string } | null {
  const message = record(value);
  const role = typeof message.role === 'string' ? message.role : 'unknown';
  const content = message.content;
  if (typeof content === 'string') return { role, text: content.slice(0, 8_000) };
  if (!Array.isArray(content)) return null;
  const text = content.map((part) => {
    const item = record(part);
    if (typeof item.text === 'string') return item.text;
    if (typeof item.content === 'string') return item.content;
    if (typeof item.name === 'string') return `[工具：${item.name}]`;
    return '';
  }).filter(Boolean).join('\n');
  return text ? { role, text: text.slice(0, 8_000) } : null;
}

function terminalText(chunk: Buffer): string {
  return chunk.toString('utf8')
    .replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, '')
    .replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '')
    .replace(/\r/g, '');
}

export class LocalBackend extends EventEmitter {
  private store: LocalStore | null = null;
  private pi: PiRuntime | null = null;
  private auth: AuthCoordinator | null = null;
  private remote: RemoteGateway | null = null;
  private readonly terminals = new Map<string, TerminalPane>();
  private sequence = 0;
  private started = false;

  constructor(private readonly options: LocalBackendOptions) {
    super();
  }

  override on(event: 'event', listener: (event: DesktopEvent) => void): this {
    return super.on(event, listener);
  }

  start(): void {
    if (this.started) return;
    this.emitDesktop({ type: 'host_status', status: 'starting' });
    try {
      this.store = new LocalStore(this.options.dataRoot);
      this.pi = new PiRuntime({
        resourcesPath: this.options.resourcesPath,
        environment: this.options.environment,
        runtimeLauncher: this.options.runtimeLauncher,
        onTaskChanged: (taskId, patch) => {
          try { this.requireStore().updateTask(taskId, patch); } catch { /* task may have been removed */ }
        },
        onEvent: (taskId) => this.emitServer('task_changed', { task_id: taskId }),
        onCollaborationAction: (sourceTaskId, action, params) => this.handleCollaborationAction(sourceTaskId, action, params),
      });
      this.auth = new AuthCoordinator(
        this.options.resourcesPath,
        this.options.environment ?? process.env,
        () => this.emitServer('auth_changed', this.auth?.status() ?? {}),
        this.options.runtimeLauncher,
      );
      this.remote = new RemoteGateway(
        this.options.dataRoot,
        (action, params, profileId) => this.handleRemote(action, params, profileId),
        () => this.emitServer('remote_changed', this.remote?.status() ?? {}),
        () => this.requireStore().getUiState().active_profile_id,
      );
      this.started = true;
      void this.remote.startIfEnabled().catch(() => undefined);
      this.emitDesktop({ type: 'host_status', status: 'ready' });
    } catch (error) {
      this.started = false;
      this.auth?.stop();
      void this.remote?.stop().catch(() => undefined);
      this.remote = null;
      this.auth = null;
      this.pi = null;
      try { this.store?.close(); } catch { /* keep the original startup error */ }
      this.store = null;
      this.emitDesktop({
        type: 'host_status',
        status: 'failed',
        message: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  async stop(): Promise<void> {
    if (!this.started) return;
    this.started = false;
    await this.pi?.stopAll();
    for (const pane of this.terminals.values()) pane.child.kill('SIGTERM');
    this.terminals.clear();
    this.auth?.stop();
    await this.remote?.stop();
    this.remote = null;
    this.auth = null;
    this.pi = null;
    this.store?.close();
    this.store = null;
    this.emitDesktop({ type: 'host_status', status: 'stopped' });
  }

  async request<T>(action: string, fields: Record<string, unknown> = {}): Promise<T> {
    this.start();
    const serialized = JSON.stringify(fields);
    if (Buffer.byteLength(serialized) > DESKTOP_MAX_FRAME_BYTES) throw new Error(`PAD request is too large: ${action}`);
    return await this.handle(action, fields) as T;
  }

  private async handle(action: string, fields: Record<string, unknown>): Promise<unknown> {
    const store = this.requireStore();
    switch (action) {
      case 'hello': return this.hello();
      case 'ping': return { pong: true, protocol_version: DESKTOP_PROTOCOL_VERSION };
      case 'bootstrap': return this.bootstrap();
      case 'list_sidebar': return { sidebar: store.sidebar(), ui_state: store.getUiState(), records: store.records() };
      case 'get_ui_state': return { state: store.getUiState(), sidebar: store.sidebar() };
      case 'set_ui_state': {
        const state = fields.state as DesktopUiStateDto;
        const saved = store.setUiState(state);
        return { state: saved, sidebar: store.sidebar() };
      }
      case 'create_profile': {
        const profile = store.createProfile({
          id: optionalString(fields, 'profile_id'),
          name: requiredString(fields, 'name'),
          defaultProvider: optionalString(fields, 'default_provider'),
          defaultModel: optionalString(fields, 'default_model'),
          permissionMode: optionalString(fields, 'permission_mode') as PermissionMode | undefined,
          unattended: optionalBoolean(fields, 'unattended'),
        });
        store.ensureDefaultProject(profile.id);
        this.emitServer('account_changed', { profile_id: profile.id });
        return { profile: publicProfile(profile), authentication: this.authentication(profile), sidebar: store.sidebar(), records: store.records() };
      }
      case 'set_profile': {
        const profileId = requiredString(fields, 'profile_id');
        const profile = store.updateProfile(profileId, {
          defaultProvider: optionalString(fields, 'default_provider'),
          defaultModel: optionalString(fields, 'default_model'),
          permissionMode: optionalString(fields, 'permission_mode') as PermissionMode | undefined,
          unattended: optionalBoolean(fields, 'unattended'),
        });
        this.emitServer('account_changed', { profile_id: profile.id });
        return { profile: publicProfile(profile), sidebar: store.sidebar(), records: store.records() };
      }
      case 'create_project': {
        const project = store.createProject(requiredString(fields, 'profile_id'), optionalString(fields, 'name'), requiredString(fields, 'cwd'));
        return { project, sidebar: store.sidebar(), records: store.records() };
      }
      case 'create_task': {
        const profileId = optionalString(fields, 'profile_id') ?? store.getUiState().active_profile_id ?? store.ensureDefaultProfile().id;
        const task = store.createTask({
          id: optionalString(fields, 'task_id'),
          projectId: optionalString(fields, 'project_id'),
          profileId,
          title: optionalString(fields, 'title'),
          summary: optionalString(fields, 'summary'),
          cwd: optionalString(fields, 'cwd'),
          environment: optionalString(fields, 'environment') as TaskEnvironment | undefined,
          permissionMode: optionalString(fields, 'permission_mode') as PermissionMode | undefined,
          unattended: optionalBoolean(fields, 'unattended'),
          parentTaskId: optionalString(fields, 'parent_task_id'),
          agentName: optionalString(fields, 'agent_name'),
        });
        this.emitServer('task_changed', { task_id: task.id });
        return { task: publicTask(task), sidebar: store.sidebar(), records: store.records() };
      }
      case 'set_task': {
        const taskId = requiredString(fields, 'task_id');
        const patch: Partial<StoredTask> = {};
        if (typeof fields.title === 'string') patch.title = fields.title.trim().slice(0, 100) || '未命名任务';
        if (typeof fields.pinned === 'boolean') patch.pinned = fields.pinned;
        if (typeof fields.archived === 'boolean') patch.archived = fields.archived;
        if (typeof fields.unread === 'boolean') patch.unread = fields.unread;
        const task = store.updateTask(taskId, patch);
        this.emitServer('task_changed', { task_id: task.id });
        return { task: publicTask(task), sidebar: store.sidebar(), records: store.records() };
      }
      case 'provider_status': return this.providerStatus(optionalString(fields, 'profile_id'));
      case 'list_sessions': return this.listSessions(fields);
      case 'read_session': return this.readSession(fields);
      case 'rename_session': return this.renameSession(fields);
      case 'spawn_agent': return this.spawnAgent(fields);
      case 'send_message': return this.sendSessionMessage(fields);
      case 'followup_task': return this.followupTask(fields);
      case 'wait_agent': return this.waitAgent(fields);
      case 'interrupt_agent': return this.interruptAgent(fields);
      case 'list_agents': return this.listAgents(fields);
      case 'model_catalog': {
        const profile = this.profile(requiredString(fields, 'profile_id'));
        return await modelCatalog(
          this.options.resourcesPath,
          profile,
          fields.refresh === true,
          this.options.environment,
          this.options.runtimeLauncher,
        );
      }
      case 'start_task': return this.startTask(requiredString(fields, 'task_id'), {});
      case 'prompt': return this.prompt(fields);
      case 'poll': return this.poll(requiredString(fields, 'task_id'));
      case 'history':
      case 'get_messages': return this.history(requiredString(fields, 'task_id'));
      case 'get_state': return this.piCommand(requiredString(fields, 'task_id'), { type: 'get_state' });
      case 'get_entries': return this.piCommand(requiredString(fields, 'task_id'), { type: 'get_entries', since: optionalString(fields, 'since') });
      case 'set_model': return this.setModel(fields);
      case 'set_thinking_level': return this.setThinking(fields);
      case 'abort': return this.abort(requiredString(fields, 'task_id'));
      case 'stop':
      case 'stop_task': return this.stopTask(requiredString(fields, 'task_id'));
      case 'retry_task': return this.retryTask(requiredString(fields, 'task_id'));
      case 'respond_ui':
      case 'extension_ui_response': return this.respondUi(fields);
      case 'auth_begin': return this.authBegin(fields);
      case 'auth_status': return this.authResult();
      case 'auth_respond': return this.authRespond(fields);
      case 'auth_cancel': return this.authCancel(fields);
      case 'logout': return this.logout(fields);
      case 'terminal_open': return this.terminalOpen(fields);
      case 'terminal_input': return this.terminalInput(fields);
      case 'terminal_resize': return this.terminalResize(fields);
      case 'terminal_snapshot': return this.terminalSnapshot(fields);
      case 'terminal_close': return this.terminalClose(fields);
      case 'runtime_snapshot': return this.runtimeSnapshot(requiredString(fields, 'task_id'));
      case 'remote_status': return { remote: this.requireRemote().status() };
      case 'remote_set_enabled': return { remote: await this.requireRemote().setEnabled(fields.enabled === true) };
      case 'remote_pair_begin': return this.requireRemote().beginPairing();
      case 'remote_pair_cancel': return { remote: this.requireRemote().cancelPairing(requiredString(fields, 'pairing_id')) };
      case 'remote_device_revoke': return { remote: this.requireRemote().revokeDevice(requiredString(fields, 'device_id')) };
      default: throw new Error(`Unsupported PAD desktop action: ${action}`);
    }
  }

  private hello(): DesktopHelloResult {
    return {
      protocol: { current: DESKTOP_PROTOCOL_VERSION, supported: [1, 2], minimum_compatible: 1 },
      server: { name: 'pad-electron-local', version: packageMetadata.version },
      capabilities: CAPABILITIES,
      limits: { max_frame_bytes: DESKTOP_MAX_FRAME_BYTES, max_request_id_bytes: 128 },
    };
  }

  private bootstrap(profileId?: string): DesktopBootstrapResult {
    const store = this.requireStore();
    const fallback = profileId ? this.profile(profileId) : store.ensureDefaultProfile();
    const state = store.getUiState();
    const profile = profileId ? fallback : state.active_profile_id ? store.getStoredProfile(state.active_profile_id) ?? fallback : fallback;
    store.ensureDefaultProject(profile.id);
    if (!profileId && !state.active_profile_id) store.setUiState({ ...state, active_profile_id: profile.id });
    const selectedTask = state.selected_task_id ? store.getStoredTask(state.selected_task_id) : null;
    const scopedProjectIds = profileId
      ? new Set(store.records(profileId).projects.map((project) => project.id))
      : null;
    const uiState = profileId
      ? {
        ...state,
        active_profile_id: profile.id,
        selected_task_id: selectedTask?.profile_id === profile.id ? selectedTask.id : null,
        collapsed_project_ids: state.collapsed_project_ids.filter((id) => scopedProjectIds?.has(id)),
      }
      : store.getUiState();
    const auth = this.authentication(profile);
    return {
      protocol_version: DESKTOP_PROTOCOL_VERSION,
      protocol: this.hello().protocol,
      backend: {
        status: 'ready',
        provider_authentication: auth.status,
        authenticated_providers: auth.authenticated_providers,
        selected_provider: profile.default_provider,
        selected_model: profile.default_model,
      },
      profile: publicProfile(profile),
      capabilities: CAPABILITIES,
      sidebar: store.sidebar(profileId),
      ui_state: uiState,
      records: store.records(profileId),
    };
  }

  private async handleRemote(action: string, fields: Record<string, unknown>, profileId: string): Promise<unknown> {
    const store = this.requireStore();
    if (!store.getStoredProfile(profileId)) throw new Error('Paired profile is unavailable; pair this device again');
    const requestedProfileId = optionalString(fields, 'profile_id');
    if (requestedProfileId && requestedProfileId !== profileId) throw new Error('Remote command is outside the paired profile');
    if (action === 'bootstrap') return this.bootstrap(profileId);
    if (action === 'list_sidebar') {
      const uiState = store.getUiState();
      const selectedTask = uiState.selected_task_id ? store.getStoredTask(uiState.selected_task_id) : null;
      const projectIds = new Set(store.records(profileId).projects.map((project) => project.id));
      return {
        sidebar: store.sidebar(profileId),
        ui_state: {
          ...uiState,
          active_profile_id: profileId,
          selected_task_id: selectedTask?.profile_id === profileId ? selectedTask.id : null,
          collapsed_project_ids: uiState.collapsed_project_ids.filter((id) => projectIds.has(id)),
        },
        records: store.records(profileId),
      };
    }
    if (action === 'create_task') {
      const projectId = optionalString(fields, 'project_id');
      if (projectId) {
        const project = store.listProjects(true).find((candidate) => candidate.id === projectId);
        if (!project || project.profile_id !== profileId) throw new Error('Project is outside the paired profile');
      }
      return this.scopeRemoteResult(await this.handle(action, { ...fields, profile_id: profileId }), profileId);
    }
    const taskId = requiredString(fields, 'task_id');
    const task = store.getStoredTask(taskId);
    if (!task || task.profile_id !== profileId) throw new Error('Task is outside the paired profile');
    return this.scopeRemoteResult(await this.handle(action, fields), profileId);
  }

  private scopeRemoteResult(result: unknown, profileId: string): unknown {
    if (!result || typeof result !== 'object' || Array.isArray(result)) return result;
    const response = result as Record<string, unknown>;
    return {
      ...response,
      ...(Object.prototype.hasOwnProperty.call(response, 'sidebar')
        ? { sidebar: this.requireStore().sidebar(profileId) }
        : {}),
      ...(Object.prototype.hasOwnProperty.call(response, 'records')
        ? { records: this.requireStore().records(profileId) }
        : {}),
    };
  }

  private authentication(profile: StoredProfile): { status: string; authenticated_providers: string[] } {
    const providers = authenticatedProviders(profile);
    return { status: providers.length ? 'authenticated' : 'missing', authenticated_providers: providers };
  }

  private providerStatus(profileId?: string): Record<string, unknown> {
    const store = this.requireStore();
    const profile = profileId
      ? this.profile(profileId)
      : this.profile(store.getUiState().active_profile_id ?? store.ensureDefaultProfile().id);
    const auth = this.authentication(profile);
    return {
      profile_id: profile.id,
      status: 'ready',
      provider_authentication: auth.status,
      authenticated_providers: auth.authenticated_providers,
      selected_provider: profile.default_provider,
      selected_model: profile.default_model,
    };
  }

  private async handleCollaborationAction(
    sourceTaskId: string,
    action: string,
    params: Record<string, unknown>,
  ): Promise<unknown> {
    this.task(sourceTaskId);
    const supported = new Set([
      'list_sessions', 'read_session', 'rename_session', 'spawn_agent', 'send_message',
      'followup_task', 'wait_agent', 'interrupt_agent', 'list_agents',
    ]);
    if (!supported.has(action)) throw new Error(`Unsupported collaboration action: ${action}`);
    const fields: Record<string, unknown> = { ...params, source_task_id: sourceTaskId };
    if (action === 'rename_session' && typeof fields.task_id !== 'string') fields.task_id = sourceTaskId;
    return this.handle(action, fields);
  }

  private listSessions(fields: Record<string, unknown>): Record<string, unknown> {
    const query = (optionalString(fields, 'query') ?? '').trim().toLocaleLowerCase('zh-CN');
    const limit = Math.max(1, Math.min(50, Math.round(optionalNumber(fields, 'limit') ?? 20)));
    const sessions = this.requireStore().listStoredTasks(true)
      .filter((task) => !query || [task.title, task.summary, task.id, task.agent_name ?? '']
        .some((value) => value.toLocaleLowerCase('zh-CN').includes(query)))
      .slice(0, limit)
      .map((task) => this.sessionSummary(task));
    return { sessions };
  }

  private async readSession(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const task = this.task(requiredString(fields, 'task_id'));
    const limit = Math.max(1, Math.min(50, Math.round(optionalNumber(fields, 'limit') ?? 20)));
    const transcript = (await this.sessionMessages(task.id)).map(messageText).filter((entry): entry is { role: string; text: string } => entry !== null).slice(-limit);
    return { session: this.sessionSummary(this.task(task.id)), transcript };
  }

  private renameSession(fields: Record<string, unknown>): Record<string, unknown> {
    const taskId = requiredString(fields, 'task_id');
    const name = requiredString(fields, 'name').trim().replace(/\s+/g, ' ').slice(0, 100);
    const task = this.requireStore().updateTask(taskId, { title: name });
    this.emitServer('task_changed', { task_id: task.id });
    return { renamed: true, session: this.sessionSummary(task) };
  }

  private async spawnAgent(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const source = this.task(requiredString(fields, 'source_task_id'));
    const delegatedTask = requiredString(fields, 'task').trim();
    const requestedName = optionalString(fields, 'name')?.trim();
    const title = (requestedName || delegatedTask.split('\n')[0] || '子 Agent').slice(0, 100);
    const child = this.requireStore().createTask({
      profileId: source.profile_id,
      projectId: source.project_id ?? undefined,
      parentTaskId: source.id,
      agentName: requestedName || title,
      title,
      summary: `由“${source.title}”创建的子 Agent`,
      cwd: source.cwd,
      environment: source.environment,
      permissionMode: source.policy?.mode ?? undefined,
      unattended: source.policy?.unattended ?? undefined,
    });
    this.emitServer('task_changed', { task_id: child.id, parent_task_id: source.id });
    const profile = this.profile(child.profile_id);
    await this.prompt({
      task_id: child.id,
      prompt: `你是 PAD 中由 Session “${source.title}” (${source.id}) 创建的持久化子 Agent。\n\n任务：${delegatedTask}`,
      ...(profile.default_provider && profile.default_model
        ? { provider: profile.default_provider, model: profile.default_model }
        : {}),
    });
    return { spawned: true, agent: this.sessionSummary(this.task(child.id)) };
  }

  private sendSessionMessage(fields: Record<string, unknown>): Record<string, unknown> {
    const source = this.task(requiredString(fields, 'source_task_id'));
    const target = this.task(requiredString(fields, 'task_id'));
    const message = requiredString(fields, 'message');
    const messageId = this.requireStore().enqueueMessage(source.id, target.id, message);
    this.requireStore().updateTask(target.id, { unread: true });
    this.emitServer('task_changed', { task_id: target.id });
    return { queued: true, message_id: messageId, target: this.sessionSummary(this.task(target.id)) };
  }

  private async followupTask(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const source = this.task(requiredString(fields, 'source_task_id'));
    const target = this.task(requiredString(fields, 'task_id'));
    const message = requiredString(fields, 'message');
    const profile = this.profile(target.profile_id);
    await this.prompt({
      task_id: target.id,
      prompt: `[来自 Session “${source.title}” (${source.id}) 的即时消息]\n${message}`,
      ...(profile.default_provider && profile.default_model
        ? { provider: profile.default_provider, model: profile.default_model }
        : {}),
    });
    this.requireStore().updateTask(target.id, { unread: true });
    this.emitServer('task_changed', { task_id: target.id });
    return { accepted: true, target: this.sessionSummary(this.task(target.id)) };
  }

  private async waitAgent(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const sourceTaskId = requiredString(fields, 'source_task_id');
    const targetId = requiredString(fields, 'task_id');
    if (sourceTaskId === targetId) throw new Error('A session cannot wait for itself');
    const timeoutSeconds = Math.max(0, Math.min(30, optionalNumber(fields, 'timeout_seconds') ?? 20));
    const deadline = Date.now() + timeoutSeconds * 1_000;
    const active = new Set(['starting', 'running', 'streaming', 'tool_running', 'compacting', 'retrying']);
    while (active.has(this.task(targetId).status) && Date.now() < deadline) {
      await new Promise<void>((resolve) => setTimeout(resolve, 200));
    }
    const target = this.task(targetId);
    const transcript = (await this.sessionMessages(target.id)).map(messageText).filter((entry): entry is { role: string; text: string } => entry !== null).slice(-6);
    return { timed_out: active.has(target.status), agent: this.sessionSummary(target), transcript };
  }

  private async interruptAgent(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const sourceTaskId = requiredString(fields, 'source_task_id');
    const targetId = requiredString(fields, 'task_id');
    if (sourceTaskId === targetId) throw new Error('A session cannot interrupt itself through this tool');
    await this.abort(targetId);
    return { interrupted: true, agent: this.sessionSummary(this.task(targetId)) };
  }

  private listAgents(fields: Record<string, unknown>): Record<string, unknown> {
    const rootId = optionalString(fields, 'task_id') ?? requiredString(fields, 'source_task_id');
    const tasks = this.requireStore().listStoredTasks(true);
    const root = tasks.find((task) => task.id === rootId);
    if (!root) throw new Error(`Task not found: ${rootId}`);
    const included = new Set([root.id]);
    for (let changed = true; changed;) {
      changed = false;
      for (const task of tasks) {
        if (!task.parent_task_id || !included.has(task.parent_task_id) || included.has(task.id)) continue;
        included.add(task.id);
        changed = true;
      }
    }
    return { root: this.sessionSummary(root), agents: tasks.filter((task) => task.id !== root.id && included.has(task.id)).map((task) => this.sessionSummary(task)) };
  }

  private sessionSummary(task: StoredTask): Record<string, unknown> {
    return {
      id: task.id,
      title: task.title,
      summary: task.summary,
      status: task.status,
      profile_id: task.profile_id,
      project_id: task.project_id,
      parent_task_id: task.parent_task_id ?? null,
      agent_name: task.agent_name ?? null,
      unread: task.unread,
      updated_at: task.updated_at,
    };
  }

  private async startTask(taskId: string, launch: { provider?: string; model?: string; thinkingLevel?: string; fastMode?: boolean }): Promise<Record<string, unknown>> {
    const task = this.task(taskId);
    const profile = this.profile(task.profile_id);
    await this.requirePi().start(task, profile, { ...launch, fastMode: launch.fastMode ?? true });
    return { task_id: taskId, running: true, backend: { status: 'ready', provider_authentication: this.authentication(profile).status, task_runtime: 'starting' } };
  }

  private async prompt(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const taskId = requiredString(fields, 'task_id');
    const directMessage = requiredString(fields, 'prompt');
    const queuedMessages = this.requireStore().pendingMessages(taskId);
    const queuedContext = queuedMessages.map((queued) =>
      `[来自 Session “${queued.sourceTitle}” (${queued.sourceTaskId}) 的消息]\n${queued.message}`,
    ).join('\n\n');
    const message = queuedContext ? `${queuedContext}\n\n${directMessage}` : directMessage;
    const provider = optionalString(fields, 'provider');
    const model = optionalString(fields, 'model');
    if (Boolean(provider) !== Boolean(model)) throw new Error('Provider and model must be selected together');
    const thinkingLevel = optionalString(fields, 'thinking_level');
    const fastMode = fields.fast_mode !== false;
    let process = this.requirePi().get(taskId);
    if (!process) {
      await this.startTask(taskId, { provider, model, thinkingLevel, fastMode });
      process = this.requirePi().get(taskId);
    } else {
      process.setFastMode(fastMode);
      if (provider && model) await process.request({ type: 'set_model', provider, modelId: model });
      if (thinkingLevel && thinkingLevel !== 'default') await process.request({ type: 'set_thinking_level', level: thinkingLevel });
    }
    if (!process) throw new Error('Pi task failed to start');
    await process.request({ type: 'prompt', message, streamingBehavior: process.status === 'idle' ? undefined : 'followUp' });
    this.requireStore().acknowledgeMessages(queuedMessages.map((queued) => queued.id));
    this.requireStore().updateTask(taskId, { status: 'running' });
    void process.request({ type: 'get_state' }).catch(() => undefined);
    this.emitServer('task_changed', { task_id: taskId });
    return { task_id: taskId, accepted: true };
  }

  private poll(taskId: string): Record<string, unknown> {
    const process = this.requirePi().get(taskId);
    const task = this.task(taskId);
    const poll = process?.poll() ?? { events: [], pending_ui_requests: [], status: task.status };
    return { task_id: taskId, poll, task: publicTask(this.task(taskId)), sidebar: this.requireStore().sidebar() };
  }

  private async history(taskId: string): Promise<Record<string, unknown>> {
    const messages = await this.sessionMessages(taskId);
    return { task_id: taskId, command: 'history', response: { success: true, data: { messages } }, messages, pending: false, task: publicTask(this.task(taskId)), sidebar: this.requireStore().sidebar() };
  }

  private async sessionMessages(taskId: string): Promise<unknown[]> {
    const task = this.task(taskId);
    const process = this.requirePi().get(taskId);
    let messages = this.requirePi().history(task);
    if (process) {
      try {
        const response = await process.request({ type: 'get_messages' }, 3_000);
        const data = record(response.data);
        if (Array.isArray(data.messages)) messages = data.messages;
      } catch { /* persisted journal remains usable */ }
    }
    return messages;
  }

  private async piCommand(taskId: string, command: Record<string, unknown>): Promise<Record<string, unknown>> {
    const process = this.requirePi().get(taskId);
    if (!process) throw new Error('Pi task is not running');
    const response = await process.request(command);
    return { task_id: taskId, command: command.type, response, pending: false, task: publicTask(this.task(taskId)), sidebar: this.requireStore().sidebar() };
  }

  private async setModel(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const taskId = requiredString(fields, 'task_id');
    const provider = requiredString(fields, 'provider');
    const model = requiredString(fields, 'model');
    const process = this.requirePi().get(taskId);
    if (process) await process.request({ type: 'set_model', provider, modelId: model });
    this.requireStore().updateProfile(this.task(taskId).profile_id, { defaultProvider: provider, defaultModel: model });
    return { task_id: taskId, accepted: true, provider, model_id: model };
  }

  private async setThinking(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const taskId = requiredString(fields, 'task_id');
    const level = requiredString(fields, 'thinking_level');
    const process = this.requirePi().get(taskId);
    if (!process) throw new Error('Pi task is not running');
    await process.request({ type: 'set_thinking_level', level });
    return { task_id: taskId, accepted: true, thinking_level: level };
  }

  private async abort(taskId: string): Promise<Record<string, unknown>> {
    const process = this.requirePi().get(taskId);
    if (process) await process.request({ type: 'abort' });
    return { task_id: taskId, aborted: true };
  }

  private async stopTask(taskId: string): Promise<Record<string, unknown>> {
    await this.requirePi().stop(taskId);
    this.requireStore().updateTask(taskId, { status: 'disconnected' });
    this.emitServer('task_changed', { task_id: taskId });
    return { task_id: taskId, stopped: true };
  }

  private async retryTask(taskId: string): Promise<Record<string, unknown>> {
    await this.requirePi().stop(taskId);
    this.requireStore().updateTask(taskId, { status: 'retrying' });
    return { ...(await this.startTask(taskId, {})), retrying: true };
  }

  private async respondUi(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const taskId = requiredString(fields, 'task_id');
    const id = optionalString(fields, 'request_id') ?? requiredString(fields, 'interaction_id');
    const process = this.requirePi().get(taskId);
    if (!process) throw new Error('Pi task is not running');
    await process.respondUi(id, optionalString(fields, 'response_kind'), fields.value, fields.cancelled === true);
    return { task_id: taskId, accepted: true };
  }

  private authBegin(fields: Record<string, unknown>): AuthResultDto {
    const profile = this.profile(requiredString(fields, 'profile_id'));
    const provider = requiredString(fields, 'provider');
    const authType = requiredString(fields, 'auth_type') as AuthType;
    this.requireAuth().begin(profile, provider, authType);
    return this.authResult();
  }

  private authRespond(fields: Record<string, unknown>): AuthResultDto {
    this.requireAuth().respond(requiredString(fields, 'attempt_id'), requiredString(fields, 'prompt_id'), fields.value, fields.cancelled === true);
    return this.authResult();
  }

  private authCancel(fields: Record<string, unknown>): AuthResultDto {
    this.requireAuth().cancel(requiredString(fields, 'attempt_id'));
    return this.authResult();
  }

  private logout(fields: Record<string, unknown>): AuthResultDto {
    const profile = this.profile(requiredString(fields, 'profile_id'));
    this.requireAuth().begin(profile, requiredString(fields, 'provider'), 'oauth', 'logout');
    return this.authResult();
  }

  private authResult(): AuthResultDto {
    const snapshot = this.requireAuth().status();
    const profile = snapshot.profile_id ? this.requireStore().getStoredProfile(snapshot.profile_id) : null;
    return {
      auth: snapshot,
      account: profile ? {
        profile: publicProfile(profile),
        provider_authentication: this.authentication(profile).status,
        authenticated_providers: authenticatedProviders(profile),
      } : null,
    };
  }

  private terminalOpen(fields: Record<string, unknown>): TerminalOpenDto {
    const task = this.task(requiredString(fields, 'task_id'));
    const id = optionalString(fields, 'pane_id') ?? `terminal-${randomUUID()}`;
    const columns = typeof fields.columns === 'number' ? fields.columns : 100;
    const rows = typeof fields.rows === 'number' ? fields.rows : 30;
    const shell = process.env.SHELL || '/bin/zsh';
    const child = spawn('/usr/bin/script', ['-q', '/dev/null', shell, '-l'], { cwd: task.cwd, stdio: ['pipe', 'pipe', 'pipe'], env: { ...process.env, TERM: 'xterm-256color' } });
    const pane: TerminalPane = { id, taskId: task.id, epoch: Date.now(), revision: 0, columns, rows, status: 'opening', lines: [], child, exit: null };
    this.terminals.set(id, pane);
    child.once('spawn', () => { pane.status = 'running'; pane.revision += 1; });
    child.stdout.on('data', (chunk: Buffer) => {
      pane.lines = [...pane.lines, ...terminalText(chunk).split('\n')].slice(-80);
      pane.revision += 1;
    });
    child.stderr.on('data', (chunk: Buffer) => {
      pane.lines = [...pane.lines, ...terminalText(chunk).split('\n')].slice(-80);
      pane.revision += 1;
    });
    child.once('exit', (code, signal) => { pane.status = 'exited'; pane.exit = { code, signaled: signal !== null }; pane.revision += 1; });
    return { pane_id: id, task_id: task.id, epoch: pane.epoch, status: 'opening', size: { columns, rows } };
  }

  private terminalInput(fields: Record<string, unknown>): TerminalAcceptedDto {
    const pane = this.terminal(requiredString(fields, 'pane_id'));
    const data = requiredString(fields, 'data');
    pane.child.stdin.write(data);
    return { pane_id: pane.id, accepted: true, bytes: Buffer.byteLength(data) };
  }

  private terminalResize(fields: Record<string, unknown>): TerminalAcceptedDto {
    const pane = this.terminal(requiredString(fields, 'pane_id'));
    pane.columns = typeof fields.columns === 'number' ? fields.columns : pane.columns;
    pane.rows = typeof fields.rows === 'number' ? fields.rows : pane.rows;
    return { pane_id: pane.id, accepted: true, size: { columns: pane.columns, rows: pane.rows } };
  }

  private terminalSnapshot(fields: Record<string, unknown>): TerminalSnapshotDto {
    const pane = this.terminal(requiredString(fields, 'pane_id'));
    return {
      pane_id: pane.id, task_id: pane.taskId, epoch: pane.epoch, revision: pane.revision,
      status: pane.status, is_open: pane.status === 'opening' || pane.status === 'running',
      size: { columns: pane.columns, rows: pane.rows }, lines: pane.lines,
      cursor: null,
      mode: { alternate_screen: false, bracketed_paste: false, mouse_reporting: false, sgr_mouse: false, application_cursor: false },
      viewport: { display_offset: 0, history_size: pane.lines.length }, exit: pane.exit,
    };
  }

  private terminalClose(fields: Record<string, unknown>): TerminalCloseDto {
    const pane = this.terminal(requiredString(fields, 'pane_id'));
    pane.child.kill('SIGTERM');
    this.terminals.delete(pane.id);
    return { pane_id: pane.id, closed: true };
  }

  private runtimeSnapshot(taskId: string): Record<string, unknown> {
    const process = this.requirePi().get(taskId);
    return { task_id: taskId, runtime: process ? { status: process.status } : null };
  }

  private profile(id: string): StoredProfile {
    const profile = this.requireStore().getStoredProfile(id);
    if (!profile) throw new Error(`Profile not found: ${id}`);
    return profile;
  }

  private task(id: string): StoredTask {
    const task = this.requireStore().getStoredTask(id);
    if (!task) throw new Error(`Task not found: ${id}`);
    return task;
  }

  private terminal(id: string): TerminalPane {
    const pane = this.terminals.get(id);
    if (!pane) throw new Error(`Terminal not found: ${id}`);
    return pane;
  }

  private requireStore(): LocalStore {
    if (!this.store) throw new Error('PAD local store is unavailable');
    return this.store;
  }

  private requirePi(): PiRuntime {
    if (!this.pi) throw new Error('Pi runtime is unavailable');
    return this.pi;
  }

  private requireAuth(): AuthCoordinator {
    if (!this.auth) throw new Error('Pi authentication is unavailable');
    return this.auth;
  }

  private requireRemote(): RemoteGateway {
    if (!this.remote) throw new Error('Remote connection is unavailable');
    return this.remote;
  }

  private emitServer(kind: DesktopServerEventKind, payload: unknown): void {
    const event: DesktopServerEvent = { type: 'desktop_event', protocol_version: DESKTOP_PROTOCOL_VERSION, sequence: ++this.sequence, event: { kind, payload } };
    this.remote?.broadcast(event);
    this.emitDesktop({ type: 'backend_event', payload: event });
  }

  private emitDesktop(event: DesktopEvent): void {
    this.emit('event', event);
  }
}
