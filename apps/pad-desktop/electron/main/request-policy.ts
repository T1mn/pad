import type { DesktopAction } from '../../shared/protocol';

const allowedFields: Record<DesktopAction, readonly string[]> = {
  hello: [],
  ping: [],
  list_sidebar: [],
  create_profile: [
    'profile_id',
    'name',
    'default_provider',
    'default_model',
    'permission_mode',
    'unattended',
  ],
  create_project: ['profile_id', 'name', 'cwd'],
  create_task: [
    'task_id',
    'project_id',
    'profile_id',
    'title',
    'summary',
    'cwd',
    'environment',
    'permission_mode',
    'unattended',
  ],
  start_task: ['task_id'],
  retry_task: ['task_id'],
  prompt: ['task_id', 'prompt', 'provider', 'model', 'thinking_level'],
  poll: ['task_id'],
  history: ['task_id'],
  get_messages: ['task_id'],
  get_state: ['task_id'],
  get_entries: ['task_id', 'since'],
  set_model: ['task_id', 'provider', 'model'],
  set_thinking_level: ['task_id', 'thinking_level'],
  respond_ui: ['task_id', 'request_id', 'interaction_id', 'response_kind', 'value'],
  extension_ui_response: ['task_id', 'request_id', 'interaction_id', 'response_kind', 'value'],
  provider_status: ['profile_id'],
  model_catalog: ['profile_id', 'refresh'],
  abort: ['task_id'],
  runtime_snapshot: ['task_id'],
  stop: ['task_id'],
  stop_task: ['task_id'],
  set_task: ['task_id', 'pinned', 'archived', 'unread'],
  set_profile: [
    'profile_id',
    'default_provider',
    'default_model',
    'permission_mode',
    'unattended',
  ],
  auth_begin: ['profile_id', 'provider', 'auth_type'],
  auth_status: ['attempt_id', 'profile_id'],
  auth_respond: ['attempt_id', 'prompt_id', 'value', 'cancelled'],
  auth_cancel: ['attempt_id'],
  logout: ['profile_id', 'provider'],
  terminal_open: ['task_id', 'pane_id', 'label', 'columns', 'rows'],
  terminal_input: ['pane_id', 'data'],
  terminal_resize: ['pane_id', 'columns', 'rows'],
  terminal_snapshot: ['pane_id'],
  terminal_close: ['pane_id'],
  get_ui_state: [],
  set_ui_state: ['state'],
  remote_status: [],
  remote_set_enabled: ['enabled'],
  remote_pair_begin: [],
  remote_pair_cancel: ['pairing_id'],
  remote_device_revoke: ['device_id'],
};

export function isDesktopAction(value: unknown): value is DesktopAction {
  return typeof value === 'string' && Object.hasOwn(allowedFields, value);
}

export function sanitizeDesktopParams(
  action: DesktopAction,
  params: unknown,
): Record<string, unknown> {
  if (params === undefined || params === null) return {};
  if (typeof params !== 'object' || Array.isArray(params)) {
    throw new Error(`Invalid parameters for ${action}`);
  }
  const object = params as Record<string, unknown>;
  const allowed = new Set(allowedFields[action]);
  const unknown = Object.keys(object).filter((key) => !allowed.has(key));
  if (unknown.length > 0) {
    throw new Error(`Unsupported ${action} parameters: ${unknown.join(', ')}`);
  }
  return Object.fromEntries(Object.entries(object).filter(([, value]) => value !== undefined));
}
