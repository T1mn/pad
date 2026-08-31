import { randomUUID } from 'node:crypto';
import { mkdirSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import type {
  DesktopRecords,
  DesktopSidebarViewDto,
  DesktopThemeDto,
  DesktopUiStateDto,
  PermissionMode,
  ProfileDto,
  ProjectDto,
  TaskDto,
  TaskEnvironment,
} from '../../shared/protocol';

export interface StoredProfile extends ProfileDto {
  agent_dir: string;
  session_dir: string;
  credential_ref: string | null;
  default_provider: string | null;
  default_model: string | null;
}

export interface StoredTask extends TaskDto {
  pi_session_id: string | null;
  session_file: string | null;
  leaf_id: string | null;
}

interface StoredPolicy {
  mode?: PermissionMode | null;
  unattended?: boolean | null;
}

const DEFAULT_UI_STATE: DesktopUiStateDto = {
  active_profile_id: null,
  selected_task_id: null,
  collapsed_section_ids: [],
  collapsed_project_ids: [],
  sidebar_width: 275,
  sidebar_view: 'all',
  theme: 'system',
  right_panel_open: false,
  bottom_panel_open: false,
  sidebar_open: true,
};

const SCHEMA = `
CREATE TABLE IF NOT EXISTS profiles (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  agent_dir TEXT NOT NULL,
  session_dir TEXT NOT NULL,
  credential_ref TEXT,
  default_provider TEXT,
  default_model TEXT,
  policy_json TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  primary_root TEXT NOT NULL,
  additional_roots_json TEXT NOT NULL DEFAULT '[]',
  profile_id TEXT REFERENCES profiles(id) ON DELETE SET NULL,
  policy_json TEXT NOT NULL DEFAULT '{}',
  pinned INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY NOT NULL,
  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE RESTRICT,
  pi_session_id TEXT,
  session_file TEXT,
  title TEXT NOT NULL DEFAULT '',
  summary TEXT NOT NULL DEFAULT '',
  cwd TEXT NOT NULL,
  environment TEXT NOT NULL DEFAULT 'local',
  status TEXT NOT NULL DEFAULT 'idle',
  leaf_id TEXT,
  unread INTEGER NOT NULL DEFAULT 0,
  pinned INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  policy_json TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS sections (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  section_order INTEGER NOT NULL DEFAULT 0,
  collapsed INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS section_items (
  section_id TEXT NOT NULL REFERENCES sections(id) ON DELETE CASCADE,
  item_kind TEXT NOT NULL,
  item_id TEXT NOT NULL,
  item_order INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (section_id, item_kind, item_id)
);
CREATE TABLE IF NOT EXISTS desktop_ui_state (
  singleton_id INTEGER PRIMARY KEY NOT NULL,
  state_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_projects_profile ON projects(profile_id);
CREATE INDEX IF NOT EXISTS idx_tasks_profile ON tasks(profile_id, archived, updated_at DESC);
`;

function now(): number {
  return Math.floor(Date.now() / 1000);
}

function parseJson<T>(value: unknown, fallback: T): T {
  if (typeof value !== 'string') return fallback;
  try {
    return JSON.parse(value) as T;
  } catch {
    return fallback;
  }
}

function bool(value: unknown): boolean {
  return Number(value) !== 0;
}

function text(value: unknown): string {
  return typeof value === 'string' ? value : '';
}

function optionalText(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function number(value: unknown): number {
  return typeof value === 'number' ? value : Number(value) || 0;
}

function safeSegment(value: string): string {
  const segment = value.replace(/[^a-zA-Z0-9._-]+/g, '-').replace(/^-+|-+$/g, '');
  return segment || randomUUID();
}

function mapProfile(row: Record<string, unknown>): StoredProfile {
  const policy = parseJson<StoredPolicy>(row.policy_json, {});
  return {
    id: text(row.id),
    name: text(row.name),
    agent_dir: text(row.agent_dir),
    session_dir: text(row.session_dir),
    credential_ref: optionalText(row.credential_ref),
    default_provider: optionalText(row.default_provider),
    default_model: optionalText(row.default_model),
    policy: {
      mode: policy.mode ?? null,
      unattended: policy.unattended ?? null,
    },
    created_at: number(row.created_at),
    updated_at: number(row.updated_at),
  };
}

function mapProject(row: Record<string, unknown>): ProjectDto {
  return {
    id: text(row.id),
    name: text(row.name),
    primary_root: text(row.primary_root),
    additional_roots: parseJson<string[]>(row.additional_roots_json, []),
    profile_id: optionalText(row.profile_id),
    pinned: bool(row.pinned),
    archived: bool(row.archived),
    created_at: number(row.created_at),
    updated_at: number(row.updated_at),
  };
}

function mapTask(row: Record<string, unknown>): StoredTask {
  const policy = parseJson<StoredPolicy>(row.policy_json, {});
  return {
    id: text(row.id),
    project_id: optionalText(row.project_id),
    profile_id: text(row.profile_id),
    pi_session_id: optionalText(row.pi_session_id),
    session_file: optionalText(row.session_file),
    leaf_id: optionalText(row.leaf_id),
    title: text(row.title),
    summary: text(row.summary),
    cwd: text(row.cwd),
    environment: text(row.environment) as TaskEnvironment,
    status: text(row.status),
    unread: bool(row.unread),
    pinned: bool(row.pinned),
    archived: bool(row.archived),
    policy: { mode: policy.mode ?? null, unattended: policy.unattended ?? null },
    created_at: number(row.created_at),
    updated_at: number(row.updated_at),
  };
}

function publicProfile(profile: StoredProfile): ProfileDto {
  const { agent_dir: _agentDir, session_dir: _sessionDir, credential_ref: _credentialRef, ...safe } = profile;
  return safe;
}

function publicTask(task: StoredTask): TaskDto {
  const { pi_session_id: _sessionId, session_file: _sessionFile, leaf_id: _leafId, ...safe } = task;
  return safe;
}

export class LocalStore {
  private readonly database: DatabaseSync;

  constructor(readonly dataRoot: string) {
    const storeDirectory = path.join(dataRoot, 'v1', 'store');
    mkdirSync(storeDirectory, { recursive: true, mode: 0o700 });
    mkdirSync(path.join(dataRoot, 'v1', 'profiles'), { recursive: true, mode: 0o700 });
    this.database = new DatabaseSync(path.join(storeDirectory, 'pad.sqlite'));
    this.database.exec('PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;');
    this.database.exec(SCHEMA);
    this.database.exec('PRAGMA user_version = 2;');
    this.migrateLegacyDefaultWorkspaces();
    this.database.prepare(
      "UPDATE tasks SET status = 'disconnected' WHERE status IN ('starting','running','streaming','tool_running','compacting','retrying')",
    ).run();
  }

  private migrateLegacyDefaultWorkspaces(): void {
    const legacyRoot = path.join(os.homedir(), 'Documents');
    const profiles = this.database.prepare('SELECT id FROM profiles').all() as Array<{ id: string }>;
    for (const profile of profiles) {
      const projectId = `project-${safeSegment(profile.id)}`;
      const project = this.database.prepare('SELECT primary_root FROM projects WHERE id = ?').get(projectId) as { primary_root?: string } | undefined;
      if (project?.primary_root !== legacyRoot) continue;
      const workspace = path.join(this.dataRoot, 'v1', 'profiles', safeSegment(profile.id), 'workspace');
      mkdirSync(workspace, { recursive: true, mode: 0o700 });
      this.database.prepare('UPDATE tasks SET cwd = ? WHERE project_id = ? AND cwd = ?').run(workspace, projectId, legacyRoot);
      this.database.prepare('UPDATE projects SET primary_root = ?, updated_at = ? WHERE id = ?').run(workspace, now(), projectId);
    }
  }

  close(): void {
    this.database.close();
  }

  ensureDefaultProfile(): StoredProfile {
    const existing = this.listStoredProfiles()[0];
    if (existing) {
      mkdirSync(existing.agent_dir, { recursive: true, mode: 0o700 });
      mkdirSync(existing.session_dir, { recursive: true, mode: 0o700 });
      return existing;
    }
    const timestamp = now();
    const root = path.join(this.dataRoot, 'v1', 'profiles', 'default');
    const profile: StoredProfile = {
      id: 'default',
      name: 'Default',
      agent_dir: path.join(root, 'pi-agent'),
      session_dir: path.join(root, 'pi-sessions'),
      credential_ref: null,
      default_provider: null,
      default_model: null,
      policy: { mode: 'system_full', unattended: true },
      created_at: timestamp,
      updated_at: timestamp,
    };
    mkdirSync(profile.agent_dir, { recursive: true, mode: 0o700 });
    mkdirSync(profile.session_dir, { recursive: true, mode: 0o700 });
    this.database.prepare(`INSERT INTO profiles
      (id,name,agent_dir,session_dir,credential_ref,default_provider,default_model,policy_json,created_at,updated_at)
      VALUES (?,?,?,?,?,?,?,?,?,?)`).run(
      profile.id, profile.name, profile.agent_dir, profile.session_dir, null, null, null,
      JSON.stringify(profile.policy), timestamp, timestamp,
    );
    return profile;
  }

  ensureDefaultProject(profileId: string): ProjectDto {
    const existing = this.listProjects(true).find((project) => project.profile_id === profileId);
    if (existing) return existing;
    const timestamp = now();
    const workspace = path.join(this.dataRoot, 'v1', 'profiles', safeSegment(profileId), 'workspace');
    mkdirSync(workspace, { recursive: true, mode: 0o700 });
    const project: ProjectDto = {
      id: `project-${safeSegment(profileId)}`,
      name: 'Workspace',
      primary_root: workspace,
      additional_roots: [],
      profile_id: profileId,
      pinned: false,
      archived: false,
      created_at: timestamp,
      updated_at: timestamp,
    };
    this.insertProject(project);
    return project;
  }

  getStoredProfile(id: string): StoredProfile | null {
    const row = this.database.prepare('SELECT * FROM profiles WHERE id = ?').get(id) as Record<string, unknown> | undefined;
    return row ? mapProfile(row) : null;
  }

  listStoredProfiles(): StoredProfile[] {
    return (this.database.prepare('SELECT * FROM profiles ORDER BY created_at, id').all() as Record<string, unknown>[]).map(mapProfile);
  }

  listProfiles(): ProfileDto[] {
    return this.listStoredProfiles().map(publicProfile);
  }

  createProfile(input: {
    id?: string;
    name: string;
    defaultProvider?: string;
    defaultModel?: string;
    permissionMode?: PermissionMode;
    unattended?: boolean;
  }): StoredProfile {
    const id = input.id?.trim() || `profile-${randomUUID()}`;
    const segment = safeSegment(id);
    const timestamp = now();
    const root = path.join(this.dataRoot, 'v1', 'profiles', segment);
    const profile: StoredProfile = {
      id,
      name: input.name.trim() || id,
      agent_dir: path.join(root, 'pi-agent'),
      session_dir: path.join(root, 'pi-sessions'),
      credential_ref: null,
      default_provider: input.defaultProvider?.trim() || null,
      default_model: input.defaultModel?.trim() || null,
      policy: {
        mode: input.permissionMode ?? 'system_full',
        unattended: input.unattended ?? true,
      },
      created_at: timestamp,
      updated_at: timestamp,
    };
    mkdirSync(profile.agent_dir, { recursive: true, mode: 0o700 });
    mkdirSync(profile.session_dir, { recursive: true, mode: 0o700 });
    this.database.prepare(`INSERT INTO profiles
      (id,name,agent_dir,session_dir,credential_ref,default_provider,default_model,policy_json,created_at,updated_at)
      VALUES (?,?,?,?,?,?,?,?,?,?)`).run(
      profile.id, profile.name, profile.agent_dir, profile.session_dir, null,
      profile.default_provider, profile.default_model, JSON.stringify(profile.policy), timestamp, timestamp,
    );
    return profile;
  }

  updateProfile(id: string, patch: {
    defaultProvider?: string;
    defaultModel?: string;
    permissionMode?: PermissionMode;
    unattended?: boolean;
  }): StoredProfile {
    const profile = this.getStoredProfile(id);
    if (!profile) throw new Error(`Profile not found: ${id}`);
    if (patch.defaultProvider !== undefined) profile.default_provider = patch.defaultProvider.trim() || null;
    if (patch.defaultModel !== undefined) profile.default_model = patch.defaultModel.trim() && patch.defaultModel !== 'auto' ? patch.defaultModel : null;
    if (patch.permissionMode !== undefined) profile.policy.mode = patch.permissionMode;
    if (patch.unattended !== undefined) profile.policy.unattended = patch.unattended;
    profile.updated_at = now();
    this.database.prepare(`UPDATE profiles SET default_provider=?, default_model=?, policy_json=?, updated_at=? WHERE id=?`).run(
      profile.default_provider, profile.default_model, JSON.stringify(profile.policy), profile.updated_at, id,
    );
    return profile;
  }

  listProjects(includeArchived = true): ProjectDto[] {
    const sql = includeArchived
      ? 'SELECT * FROM projects ORDER BY updated_at DESC, id'
      : 'SELECT * FROM projects WHERE archived = 0 ORDER BY updated_at DESC, id';
    return (this.database.prepare(sql).all() as Record<string, unknown>[]).map(mapProject);
  }

  createProject(profileId: string, name: string | undefined, cwd: string): ProjectDto {
    if (!this.getStoredProfile(profileId)) throw new Error(`Profile not found: ${profileId}`);
    const timestamp = now();
    const project: ProjectDto = {
      id: `project-${randomUUID()}`,
      name: name?.trim() || path.basename(cwd) || 'Workspace',
      primary_root: cwd,
      additional_roots: [],
      profile_id: profileId,
      pinned: false,
      archived: false,
      created_at: timestamp,
      updated_at: timestamp,
    };
    this.insertProject(project);
    return project;
  }

  private insertProject(project: ProjectDto): void {
    this.database.prepare(`INSERT INTO projects
      (id,name,primary_root,additional_roots_json,profile_id,policy_json,pinned,archived,created_at,updated_at)
      VALUES (?,?,?,?,?,?,?,?,?,?)`).run(
      project.id, project.name, project.primary_root, JSON.stringify(project.additional_roots), project.profile_id,
      '{}', Number(project.pinned), Number(project.archived), project.created_at, project.updated_at,
    );
  }

  getStoredTask(id: string): StoredTask | null {
    const row = this.database.prepare('SELECT * FROM tasks WHERE id = ?').get(id) as Record<string, unknown> | undefined;
    return row ? mapTask(row) : null;
  }

  listStoredTasks(includeArchived = true): StoredTask[] {
    const sql = includeArchived
      ? 'SELECT * FROM tasks ORDER BY updated_at DESC, id'
      : 'SELECT * FROM tasks WHERE archived = 0 ORDER BY updated_at DESC, id';
    return (this.database.prepare(sql).all() as Record<string, unknown>[]).map(mapTask);
  }

  createTask(input: {
    id?: string;
    projectId?: string;
    profileId: string;
    title?: string;
    summary?: string;
    cwd?: string;
    environment?: TaskEnvironment;
    permissionMode?: PermissionMode;
    unattended?: boolean;
  }): StoredTask {
    if (!this.getStoredProfile(input.profileId)) throw new Error(`Profile not found: ${input.profileId}`);
    const project = input.projectId
      ? this.listProjects(true).find((candidate) => candidate.id === input.projectId)
      : this.ensureDefaultProject(input.profileId);
    const timestamp = now();
    const task: StoredTask = {
      id: input.id?.trim() || `task-${randomUUID()}`,
      project_id: input.projectId ?? project?.id ?? null,
      profile_id: input.profileId,
      pi_session_id: null,
      session_file: null,
      leaf_id: null,
      title: input.title?.trim() || 'New task',
      summary: input.summary?.trim() || '',
      cwd: input.cwd?.trim() || project?.primary_root || path.join(os.homedir(), 'Documents'),
      environment: input.environment ?? 'local',
      status: 'idle',
      unread: false,
      pinned: false,
      archived: false,
      policy: { mode: input.permissionMode ?? null, unattended: input.unattended ?? null },
      created_at: timestamp,
      updated_at: timestamp,
    };
    this.database.prepare(`INSERT INTO tasks
      (id,project_id,profile_id,pi_session_id,session_file,title,summary,cwd,environment,status,leaf_id,
       unread,pinned,archived,policy_json,created_at,updated_at)
      VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)`).run(
      task.id, task.project_id, task.profile_id, null, null, task.title, task.summary, task.cwd,
      task.environment, task.status, null, 0, 0, 0, JSON.stringify(task.policy), timestamp, timestamp,
    );
    return task;
  }

  updateTask(id: string, patch: Partial<Pick<StoredTask,
    'status' | 'unread' | 'pinned' | 'archived' | 'pi_session_id' | 'session_file' | 'leaf_id' | 'title' | 'summary'>>): StoredTask {
    const task = this.getStoredTask(id);
    if (!task) throw new Error(`Task not found: ${id}`);
    Object.assign(task, patch);
    task.updated_at = now();
    this.database.prepare(`UPDATE tasks SET pi_session_id=?,session_file=?,title=?,summary=?,status=?,leaf_id=?,
      unread=?,pinned=?,archived=?,updated_at=? WHERE id=?`).run(
      task.pi_session_id, task.session_file, task.title, task.summary, task.status, task.leaf_id,
      Number(task.unread), Number(task.pinned), Number(task.archived), task.updated_at, id,
    );
    return task;
  }

  records(): DesktopRecords {
    return {
      profiles: this.listProfiles(),
      projects: this.listProjects(true),
      tasks: this.listStoredTasks(true).map(publicTask),
    };
  }

  getUiState(): DesktopUiStateDto {
    const row = this.database.prepare('SELECT state_json FROM desktop_ui_state WHERE singleton_id = 1').get() as { state_json?: unknown } | undefined;
    const state = parseJson<DesktopUiStateDto>(row?.state_json, DEFAULT_UI_STATE);
    const profiles = this.listStoredProfiles();
    const activeProfileId = profiles.some((profile) => profile.id === state.active_profile_id)
      ? state.active_profile_id
      : profiles[0]?.id ?? null;
    const selectedTaskId = this.listStoredTasks(true).some((task) => task.id === state.selected_task_id && task.profile_id === activeProfileId)
      ? state.selected_task_id
      : null;
    return { ...DEFAULT_UI_STATE, ...state, active_profile_id: activeProfileId, selected_task_id: selectedTaskId };
  }

  setUiState(state: DesktopUiStateDto): DesktopUiStateDto {
    const normalized: DesktopUiStateDto = {
      active_profile_id: state.active_profile_id,
      selected_task_id: state.selected_task_id,
      collapsed_section_ids: Array.isArray(state.collapsed_section_ids) ? state.collapsed_section_ids.slice(0, 256) : [],
      collapsed_project_ids: Array.isArray(state.collapsed_project_ids) ? state.collapsed_project_ids.slice(0, 256) : [],
      sidebar_width: Math.min(520, Math.max(240, Math.round(state.sidebar_width || 275))),
      sidebar_view: (['all', 'pinned', 'archive'].includes(state.sidebar_view) ? state.sidebar_view : 'all') as DesktopSidebarViewDto,
      theme: (['light', 'dark', 'system'].includes(state.theme) ? state.theme : 'system') as DesktopThemeDto,
      right_panel_open: Boolean(state.right_panel_open),
      bottom_panel_open: Boolean(state.bottom_panel_open),
      sidebar_open: state.sidebar_open !== false,
    };
    this.database.prepare(`INSERT INTO desktop_ui_state (singleton_id,state_json,updated_at) VALUES (1,?,?)
      ON CONFLICT(singleton_id) DO UPDATE SET state_json=excluded.state_json,updated_at=excluded.updated_at`).run(
      JSON.stringify(normalized), now(),
    );
    return normalized;
  }

  sidebar(): Record<string, unknown> {
    const state = this.getUiState();
    const profileId = state.active_profile_id;
    const projects = this.listProjects(true).filter((project) => project.profile_id === profileId);
    const tasks = this.listStoredTasks(true).filter((task) => task.profile_id === profileId);
    const visibleTask = (task: StoredTask) => state.sidebar_view === 'archive'
      ? task.archived
      : state.sidebar_view === 'pinned'
        ? !task.archived && task.pinned
        : !task.archived;
    const rows: Array<Record<string, unknown>> = [
      { key: 'new-task', kind: 'new_task', depth: 0, title: '新任务', status: 'none' },
    ];
    for (const project of projects) {
      const children = tasks.filter((task) => task.project_id === project.id && visibleTask(task));
      if (!children.length && (project.archived !== (state.sidebar_view === 'archive'))) continue;
      rows.push({ key: `project:${project.id}`, node: { kind: 'project', id: project.id }, depth: 0, title: project.name, pinned: project.pinned, archived: project.archived });
      for (const task of children) {
        rows.push({
          key: `task:${task.id}`,
          node: { kind: 'task', id: task.id },
          depth: 1,
          title: task.title,
          status: task.status,
          unread: task.unread,
          pinned: task.pinned,
          archived: task.archived,
        });
      }
    }
    for (const task of tasks.filter((candidate) => !candidate.project_id && visibleTask(candidate))) {
      rows.push({ key: `task:${task.id}`, node: { kind: 'task', id: task.id }, depth: 0, title: task.title, status: task.status, unread: task.unread, pinned: task.pinned, archived: task.archived });
    }
    return {
      view: state.sidebar_view,
      query: '',
      active_profile_id: profileId,
      selected_key: state.selected_task_id ? `task:${state.selected_task_id}` : null,
      rows,
    };
  }
}

export { publicProfile, publicTask };
