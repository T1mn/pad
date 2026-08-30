import { randomUUID } from 'node:crypto';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { EventEmitter } from 'node:events';
import path from 'node:path';
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

export interface LocalBackendOptions {
  dataRoot: string;
  resourcesPath: string;
  environment?: NodeJS.ProcessEnv;
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
    this.started = true;
    this.emitDesktop({ type: 'host_status', status: 'starting' });
    this.store = new LocalStore(this.options.dataRoot);
    this.pi = new PiRuntime({
      resourcesPath: this.options.resourcesPath,
      environment: this.options.environment,
      onTaskChanged: (taskId, patch) => {
        try { this.requireStore().updateTask(taskId, patch); } catch { /* task may have been removed */ }
      },
      onEvent: (taskId) => this.emitServer('task_changed', { task_id: taskId }),
    });
    this.auth = new AuthCoordinator(
      this.options.resourcesPath,
      this.options.environment ?? process.env,
      () => this.emitServer('auth_changed', this.auth?.status() ?? {}),
    );
    this.remote = new RemoteGateway(
      this.options.dataRoot,
      (action, params) => this.handle(action, params),
      () => this.emitServer('remote_changed', this.remote?.status() ?? {}),
    );
    void this.remote.startIfEnabled().catch(() => undefined);
    this.emitDesktop({ type: 'host_status', status: 'ready' });
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
        });
        this.emitServer('task_changed', { task_id: task.id });
        return { task: publicTask(task), sidebar: store.sidebar(), records: store.records() };
      }
      case 'set_task': {
        const taskId = requiredString(fields, 'task_id');
        const patch: Partial<StoredTask> = {};
        if (typeof fields.pinned === 'boolean') patch.pinned = fields.pinned;
        if (typeof fields.archived === 'boolean') patch.archived = fields.archived;
        if (typeof fields.unread === 'boolean') patch.unread = fields.unread;
        const task = store.updateTask(taskId, patch);
        this.emitServer('task_changed', { task_id: task.id });
        return { task: publicTask(task), sidebar: store.sidebar(), records: store.records() };
      }
      case 'provider_status': return this.providerStatus(optionalString(fields, 'profile_id'));
      case 'model_catalog': {
        const profile = this.profile(requiredString(fields, 'profile_id'));
        return modelCatalog(this.options.resourcesPath, profile, fields.refresh === true, this.options.environment);
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
      case 'shutdown': await this.stop(); return { stopped: true };
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
      server: { name: 'pad-electron-local', version: '0.7.6' },
      capabilities: CAPABILITIES,
      limits: { max_frame_bytes: DESKTOP_MAX_FRAME_BYTES, max_request_id_bytes: 128 },
    };
  }

  private bootstrap(): DesktopBootstrapResult {
    const store = this.requireStore();
    const fallback = store.ensureDefaultProfile();
    const state = store.getUiState();
    const profile = state.active_profile_id ? store.getStoredProfile(state.active_profile_id) ?? fallback : fallback;
    store.ensureDefaultProject(profile.id);
    if (!state.active_profile_id) store.setUiState({ ...state, active_profile_id: profile.id });
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
      sidebar: store.sidebar(),
      ui_state: store.getUiState(),
      records: store.records(),
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

  private async startTask(taskId: string, launch: { provider?: string; model?: string; thinkingLevel?: string; fastMode?: boolean }): Promise<Record<string, unknown>> {
    const task = this.task(taskId);
    const profile = this.profile(task.profile_id);
    await this.requirePi().start(task, profile, { ...launch, fastMode: launch.fastMode ?? true });
    return { task_id: taskId, running: true, backend: { status: 'ready', provider_authentication: this.authentication(profile).status, task_runtime: 'starting' } };
  }

  private async prompt(fields: Record<string, unknown>): Promise<Record<string, unknown>> {
    const taskId = requiredString(fields, 'task_id');
    const message = requiredString(fields, 'prompt');
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
    return { task_id: taskId, command: 'history', response: { success: true, data: { messages } }, messages, pending: false, task: publicTask(this.task(taskId)), sidebar: this.requireStore().sidebar() };
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
