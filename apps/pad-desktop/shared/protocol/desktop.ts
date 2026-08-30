export const DESKTOP_PROTOCOL_VERSION = 2 as const;
export const DESKTOP_SUPPORTED_PROTOCOL_VERSIONS = [1, 2] as const;
export const DESKTOP_MAX_FRAME_BYTES = 1024 * 1024;

export type DesktopProtocolVersion = (typeof DESKTOP_SUPPORTED_PROTOCOL_VERSIONS)[number];

export const DESKTOP_IPC = {
  bootstrap: 'pad-desktop:bootstrap',
  request: 'pad-desktop:request',
  event: 'pad-desktop:event',
  chooseProjectDirectory: 'pad-desktop:choose-project-directory',
  chooseAttachments: 'pad-desktop:choose-attachments',
} as const;

export type PermissionMode = 'guarded' | 'workspace_full' | 'system_full';
export type TaskEnvironment = 'local' | 'worktree' | 'remote';

/** Compatibility shape: protocol v2 omits workspace/protected roots. */
export interface PolicyLayerDto {
  mode: PermissionMode | null;
  unattended: boolean | null;
  workspace_roots?: string[];
  protected_namespaces?: Array<{ name: string; root: string }>;
}

/** Renderer-safe account record; v1 private path members are intentionally absent. */
export interface ProfileDto {
  id: string;
  name: string;
  default_provider?: string | null;
  default_model?: string | null;
  policy: PolicyLayerDto;
  authentication?: {
    status: string;
    authenticated_providers: string[];
  };
  created_at: number;
  updated_at: number;
}

export interface ProjectDto {
  id: string;
  name: string;
  primary_root: string;
  additional_roots: string[];
  profile_id: string | null;
  pinned: boolean;
  archived: boolean;
  created_at: number;
  updated_at: number;
}

export interface TaskDto {
  id: string;
  project_id: string | null;
  profile_id: string;
  title: string;
  summary: string;
  cwd: string;
  environment: TaskEnvironment;
  status: string;
  unread: boolean;
  pinned: boolean;
  archived: boolean;
  policy?: Pick<PolicyLayerDto, 'mode' | 'unattended'>;
  created_at: number;
  updated_at: number;
}

export interface DesktopRecords {
  profiles: ProfileDto[];
  projects: ProjectDto[];
  tasks: TaskDto[];
}

export interface DesktopHelloResult {
  protocol: {
    current: typeof DESKTOP_PROTOCOL_VERSION;
    supported: DesktopProtocolVersion[];
    minimum_compatible: DesktopProtocolVersion;
  };
  server: { name: string; version: string };
  capabilities: string[];
  limits: { max_frame_bytes: number; max_request_id_bytes: number };
}

export interface DesktopBootstrapResult {
  protocol_version: number;
  protocol?: DesktopHelloResult['protocol'];
  backend: {
    status: string;
    provider_authentication: string;
    authenticated_providers: string[];
    selected_provider: string | null;
    selected_model: string | null;
  };
  profile: ProfileDto;
  capabilities: string[];
  sidebar: unknown;
  ui_state: DesktopUiStateDto;
  records: DesktopRecords;
}

export type DesktopThemeDto = 'light' | 'dark' | 'system';
export type DesktopSidebarViewDto = 'all' | 'pinned' | 'archive';

/** Complete PAD-owned presentation document; it contains no provider paths or credentials. */
export interface DesktopUiStateDto {
  active_profile_id: string | null;
  selected_task_id: string | null;
  collapsed_section_ids: string[];
  collapsed_project_ids: string[];
  sidebar_width: number;
  sidebar_view: DesktopSidebarViewDto;
  theme: DesktopThemeDto;
  right_panel_open: boolean;
  bottom_panel_open: boolean;
  sidebar_open: boolean;
}

export interface DesktopUiStateResultDto {
  state: DesktopUiStateDto;
  sidebar: unknown;
}

export type AuthPhase = 'idle' | 'running' | 'succeeded' | 'failed' | 'cancelled';
export type AuthOperation = 'login' | 'logout';
export type AuthType = 'oauth' | 'api_key';

export interface AuthOptionDto {
  id: string;
  label: string;
  description?: string;
}

export interface AuthPromptDto {
  id: string;
  kind: string;
  message: string;
  placeholder?: string;
  options: AuthOptionDto[];
}

export interface AuthNoticeDto {
  kind: string;
  message: string;
  url?: string;
  user_code?: string;
}

export interface AuthSnapshotDto {
  attempt_id?: string;
  profile_id?: string;
  provider?: string;
  auth_type?: AuthType;
  operation: AuthOperation;
  phase: AuthPhase;
  prompt?: AuthPromptDto;
  notices: AuthNoticeDto[];
  error?: string;
  updated_at: number;
}

export interface AuthResultDto {
  auth: AuthSnapshotDto;
  account: {
    profile: ProfileDto;
    provider_authentication: string;
    authenticated_providers: string[];
  } | null;
}

export interface TerminalSizeDto {
  columns: number;
  rows: number;
}

export interface TerminalOpenDto {
  pane_id: string;
  task_id: string;
  epoch: number;
  status: 'opening';
  size: TerminalSizeDto;
}

export interface TerminalAcceptedDto {
  pane_id: string;
  accepted: true;
  bytes?: number;
  size?: TerminalSizeDto;
}

export interface TerminalSnapshotDto {
  pane_id: string;
  task_id: string;
  epoch: number;
  revision: number;
  status: 'opening' | 'running' | 'exited' | 'failed';
  is_open: boolean;
  size: TerminalSizeDto;
  /** At most 80 visible rows. Raw cells, scrollback, cwd, argv and env are never exposed. */
  lines: string[];
  cursor?: { column: number; row: number; shape: 'block' | 'underline' | 'beam' | 'hollow_block' } | null;
  mode: {
    alternate_screen: boolean;
    bracketed_paste: boolean;
    mouse_reporting: boolean;
    sgr_mouse: boolean;
    application_cursor: boolean;
  };
  viewport: { display_offset: number; history_size: number };
  error?: string | null;
  exit?: { code?: number | null; signaled: boolean } | null;
}

export interface TerminalCloseDto {
  pane_id: string;
  closed: true;
}

export type RemoteHostStateDto = 'disabled' | 'starting' | 'ready' | 'degraded' | 'failed';

/** Renderer-safe paired-device record. Transport details and credentials are never exposed. */
export interface RemoteDeviceDto {
  id: string;
  display_name: string;
  platform: string;
  online: boolean;
  paired_at: number;
  last_seen_at?: number;
}

/** Public remote-host status. It intentionally contains no endpoint, path, token, or raw error. */
export interface RemoteHostStatusDto {
  enabled: boolean;
  state: RemoteHostStateDto;
  display_name: string;
  active_connections: number;
  devices: RemoteDeviceDto[];
  updated_at: number;
  error_code?: string;
}

export interface RemoteStatusResultDto {
  remote: RemoteHostStatusDto;
}

export interface RemotePairingDto {
  pairing_id: string;
  /** Short-lived opaque URI. Keep only in the pairing sheet's component state. */
  qr_payload: string;
  expires_at: number;
}

export interface RemotePairBeginResultDto {
  pairing: RemotePairingDto;
}

export interface DesktopRequestParams {
  hello: Record<string, never>;
  ping: Record<string, never>;
  list_sidebar: Record<string, never>;
  create_profile: {
    profile_id?: string;
    name: string;
    default_provider?: string;
    default_model?: string;
    permission_mode?: PermissionMode;
    unattended?: boolean;
  };
  create_project: { profile_id: string; name?: string; cwd: string };
  create_task: {
    task_id?: string;
    project_id?: string;
    profile_id: string;
    title?: string;
    summary?: string;
    cwd?: string;
    environment?: TaskEnvironment;
    permission_mode?: PermissionMode;
    unattended?: boolean;
  };
  start_task: { task_id: string };
  retry_task: { task_id: string };
  prompt: { task_id: string; prompt: string };
  poll: { task_id: string };
  history: { task_id: string };
  get_messages: { task_id: string };
  get_state: { task_id: string };
  get_entries: { task_id: string; since?: string };
  set_model: { task_id: string; provider: string; model: string };
  set_thinking_level: { task_id: string; thinking_level: string };
  respond_ui: {
    task_id: string;
    request_id?: string;
    interaction_id?: string;
    response_kind?: string;
    value: unknown;
  };
  extension_ui_response: DesktopRequestParams['respond_ui'];
  provider_status: { profile_id?: string };
  /** Pi ModelRuntime-backed public catalog; credentials never cross this DTO. */
  model_catalog: { profile_id: string; refresh?: boolean };
  abort: { task_id: string };
  runtime_snapshot: { task_id: string };
  stop: { task_id: string };
  stop_task: { task_id: string };
  set_task: {
    task_id: string;
    pinned?: boolean;
    archived?: boolean;
    unread?: boolean;
  };
  set_profile: {
    profile_id: string;
    default_provider?: string;
    default_model?: string;
    permission_mode?: PermissionMode;
    unattended?: boolean;
  };
  auth_begin: { profile_id: string; provider: string; auth_type: AuthType };
  auth_status: { attempt_id?: string; profile_id?: string };
  auth_respond: {
    attempt_id: string;
    prompt_id: string;
    value?: unknown;
    cancelled?: boolean;
  };
  auth_cancel: { attempt_id: string };
  logout: { profile_id: string; provider: string };
  terminal_open: {
    task_id: string;
    pane_id?: string;
    label?: string;
    columns?: number;
    rows?: number;
  };
  terminal_input: { pane_id: string; data: string };
  terminal_resize: { pane_id: string; columns: number; rows: number };
  terminal_snapshot: { pane_id: string };
  terminal_close: { pane_id: string };
  get_ui_state: Record<string, never>;
  set_ui_state: { state: DesktopUiStateDto };
  remote_status: Record<string, never>;
  remote_set_enabled: { enabled: boolean };
  remote_pair_begin: Record<string, never>;
  remote_pair_cancel: { pairing_id: string };
  remote_device_revoke: { device_id: string };
}

export type DesktopAction = keyof DesktopRequestParams;

export interface DesktopRendererRequest<A extends DesktopAction = DesktopAction> {
  action: A;
  params: DesktopRequestParams[A];
}

export interface DesktopHostRequest {
  id: string;
  action: string;
  protocol_version?: DesktopProtocolVersion;
  [key: string]: unknown;
}

export interface DesktopHostResponse<T = unknown> {
  id?: string | null;
  ok: boolean;
  result?: T;
  error?: { code: string; message: string };
}

export type DesktopServerEventKind =
  | 'task_changed'
  | 'account_changed'
  | 'runtime_changed'
  | 'auth_changed'
  | 'remote_changed';

export interface DesktopServerEvent<T = unknown> {
  type: 'desktop_event';
  protocol_version: typeof DESKTOP_PROTOCOL_VERSION;
  sequence: number;
  event: { kind: DesktopServerEventKind; payload: T };
}

export type DesktopMenuAction =
  | 'new_task'
  | 'search'
  | 'settings'
  | 'toggle_sidebar'
  | 'toggle_terminal';

export type DesktopEvent =
  | { type: 'host_status'; status: 'starting' | 'ready' | 'stopped' | 'failed'; message?: string }
  | { type: 'backend_event'; payload: DesktopServerEvent | unknown }
  | { type: 'menu_action'; action: DesktopMenuAction; status?: never };

export interface PadDesktopApi {
  bootstrap(): Promise<DesktopBootstrapResult>;
  chooseProjectDirectory(): Promise<string | null>;
  chooseAttachments?(): Promise<string[]>;
  request<A extends DesktopAction, T = unknown>(
    action: A,
    params: DesktopRequestParams[A],
  ): Promise<T>;
  subscribe(listener: (event: DesktopEvent) => void): () => void;
}
