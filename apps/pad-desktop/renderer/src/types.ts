import type { ModelCatalog } from "./lib/model-catalog";

export type TaskStatus = "idle" | "running" | "attention" | "complete";
export type PermissionMode = "guarded" | "workspace_full" | "system_full";
export type SidebarView = "all" | "pinned" | "archive";

export interface AccountPolicy {
  mode: PermissionMode | null;
  unattended: boolean;
  workspaceRootCount: number;
  protectedNamespaceNames: string[];
}

export type ProviderAuthentication = "authenticated" | "missing" | "partial" | "unknown";

export interface AccountSummary {
  id: string;
  name: string;
  provider: string;
  selectedProvider: string | null;
  selectedModel: string | null;
  /** Pi-backed public catalog for this isolated PAD Profile. */
  modelCatalog: ModelCatalog;
  authenticatedProviders: string[];
  authentication: ProviderAuthentication;
  initials: string;
  active: boolean;
  policy: AccountPolicy;
  fullAccess: boolean;
}

export interface TaskSummary {
  id: string;
  projectId: string | null;
  profileId: string;
  title: string;
  updatedAt: string;
  status: TaskStatus;
  rawStatus: string;
  unread?: boolean;
  pinned?: boolean;
  archived?: boolean;
}

export interface ProjectSummary {
  id: string;
  profileId: string | null;
  name: string;
  path: string;
  accent: string;
  expanded: boolean;
  pinned: boolean;
}

export type SidebarNodeKind = "new_task" | "profile" | "section" | "project" | "task";

export interface SidebarRow {
  key: string;
  kind: SidebarNodeKind;
  id?: string;
  depth: number;
  title: string;
  status: TaskStatus | "none";
  unread: boolean;
  pinned: boolean;
  archived: boolean;
  missingReference: boolean;
}

export interface SidebarHierarchy {
  view: SidebarView;
  query: string;
  activeProfileId: string | null;
  selectedKey: string | null;
  rows: SidebarRow[];
}

export interface BackendSummary {
  status: string;
  capabilities: string[];
  providerAuthentication: string;
}

export type RemoteHostState = "disabled" | "starting" | "ready" | "degraded" | "failed";

export interface RemoteDevice {
  id: string;
  displayName: string;
  platform: string;
  online: boolean;
  pairedAt: number;
  lastSeenAt?: number;
}

export interface RemoteHostStatus {
  enabled: boolean;
  state: RemoteHostState;
  displayName: string;
  activeConnections: number;
  devices: RemoteDevice[];
  updatedAt: number;
  errorCode?: string;
}

/** The QR payload is intentionally returned only to the short-lived pairing sheet. */
export interface RemotePairing {
  pairingId: string;
  qrPayload: string;
  expiresAt: number;
}

export type TurnKind =
  | "user"
  | "assistant"
  | "tool"
  | "reasoning"
  | "error"
  | "status"
  | "final"
  | "activity"
  | "notice";

export type TurnArtifactKind = "file" | "change";
export type TurnArtifactOperation = "read" | "created" | "modified" | "deleted" | "renamed" | "unknown";

/**
 * A renderer-safe file reference supplied explicitly by the history protocol.
 *
 * Artifact paths and diffs are never inferred from a human-readable tool body.
 * This keeps the Files/Changes inspector tied to backend-owned structured data.
 */
export interface TurnArtifact {
  id: string;
  kind: TurnArtifactKind;
  path: string;
  operation: TurnArtifactOperation;
  previousPath?: string;
  diff?: string;
  title?: string;
}

export interface TurnEntry {
  id: string;
  kind: TurnKind;
  title?: string;
  body: string;
  meta?: string;
  state?: "running" | "complete" | "failed";
  artifacts?: TurnArtifact[];
}

export type PendingInteractionKind = "confirm" | "select" | "input" | "editor" | "unknown";

/** Renderer-safe shape exposed by the Rust v2 poll `pending_ui_requests` contract. */
export interface PendingInteraction {
  id: string;
  kind: PendingInteractionKind;
  title?: string;
  message?: string;
  options: string[];
  defaultIndex?: number;
  defaultValue?: string;
  requiresResponse: boolean;
}

export type InteractionResponse = boolean | number | string;

export interface DesktopSnapshot {
  accounts: AccountSummary[];
  modelCatalogByProfile: Record<string, ModelCatalog>;
  projects: ProjectSummary[];
  tasks: TaskSummary[];
  sidebar: SidebarHierarchy;
  backend: BackendSummary;
  turnsByTask: Record<string, TurnEntry[]>;
  interactionsByTask: Record<string, PendingInteraction[]>;
  uiState: DesktopUiState;
  remote: RemoteHostStatus | null;
}

export interface DesktopUiState {
  activeProfileId: string | null;
  selectedTaskId: string | null;
  sidebarView: SidebarView;
  collapsedSectionIds: string[];
  collapsedProjectIds: string[];
  sidebarWidth: number;
  theme: "light" | "dark" | "system";
  rightPanelOpen: boolean;
  bottomPanelOpen: boolean;
  sidebarOpen: boolean;
}

export interface TerminalSize {
  columns: number;
  rows: number;
}

export interface TerminalPane {
  paneId: string;
  taskId: string;
  epoch: number;
  status: "opening";
  size: TerminalSize;
}

export interface TerminalSnapshot {
  paneId: string;
  taskId: string;
  epoch: number;
  revision: number;
  status: "opening" | "running" | "exited" | "failed";
  isOpen: boolean;
  size: TerminalSize;
  lines: string[];
  cursor?: { column: number; row: number; shape: "block" | "underline" | "beam" | "hollow_block" } | null;
  mode: {
    alternateScreen: boolean;
    bracketedPaste: boolean;
    mouseReporting: boolean;
    applicationCursor: boolean;
  };
  error?: string | null;
  exit?: { code?: number | null; signaled: boolean } | null;
}

export type AuthPhase =
  | "idle"
  | "starting"
  | "waiting_browser"
  | "waiting_input"
  | "authenticated"
  | "failed"
  | "cancelled";

export type AuthType = "oauth" | "api_key";

export interface AuthPromptOption {
  id: string;
  label: string;
  description?: string;
}

export interface AuthSession {
  attemptId?: string;
  promptId?: string;
  profileId: string;
  provider: string | null;
  authType?: AuthType;
  phase: AuthPhase;
  title: string;
  message: string;
  verificationUrl?: string;
  promptKind?: string;
  promptMessage?: string;
  options?: AuthPromptOption[];
  inputLabel?: string;
  inputSecret?: boolean;
  error?: string;
}

export type DesktopEvent =
  | { type: "snapshot"; snapshot: DesktopSnapshot }
  | { type: "task-updated"; task: TaskSummary }
  | { type: "turn-added"; taskId: string; turn: TurnEntry }
  | { type: "auth-updated"; session: AuthSession }
  | { type: "remote-updated"; status: RemoteHostStatus | null }
  | { type: "menu-action"; action: "new_task" | "search" | "settings" | "toggle_sidebar" | "toggle_terminal" };

export type ThinkingLevel = "default" | "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";

export interface ComposerMessageInput {
  text: string;
  attachmentPaths: string[];
  provider: string;
  model: string;
  thinkingLevel: ThinkingLevel;
  fastMode: boolean;
}

export interface SendMessageInput extends ComposerMessageInput {
  taskId: string;
  accountId: string;
  fullAccess: boolean;
}

export interface DesktopAdapter {
  loadSnapshot(): Promise<DesktopSnapshot>;
  loadTaskData(taskId: string): Promise<DesktopSnapshot>;
  chooseProjectDirectory(): Promise<string | null>;
  chooseAttachments(): Promise<string[]>;
  createProject(name: string, directory: string): Promise<DesktopSnapshot>;
  sendMessage(input: SendMessageInput): Promise<void>;
  createTask(projectId: string | null): Promise<TaskSummary>;
  createAccount(name: string, provider?: string): Promise<DesktopSnapshot>;
  switchAccount(accountId: string): Promise<DesktopSnapshot>;
  setFullAccess(accountId: string, enabled: boolean): Promise<DesktopSnapshot>;
  beginLogin(accountId: string, provider?: string, authType?: AuthType): Promise<AuthSession>;
  getLoginStatus(accountId: string): Promise<AuthSession>;
  respondLogin(accountId: string, value: string): Promise<AuthSession>;
  cancelLogin(accountId: string): Promise<void>;
  logout(accountId: string, provider?: string): Promise<DesktopSnapshot>;
  abortTask(taskId: string): Promise<DesktopSnapshot>;
  retryTask(taskId: string): Promise<DesktopSnapshot>;
  respondInteraction(taskId: string, interactionId: string, value: InteractionResponse): Promise<void>;
  updateTask(taskId: string, patch: { pinned?: boolean; archived?: boolean; unread?: boolean }): Promise<DesktopSnapshot>;
  openTerminal(taskId: string, size: TerminalSize): Promise<TerminalPane>;
  writeTerminal(paneId: string, data: string): Promise<void>;
  resizeTerminal(paneId: string, size: TerminalSize): Promise<void>;
  getTerminalSnapshot(paneId: string): Promise<TerminalSnapshot>;
  closeTerminal(paneId: string): Promise<void>;
  updateUiState(patch: Partial<DesktopUiState>): Promise<DesktopUiState>;
  getRemoteStatus(): Promise<RemoteHostStatus | null>;
  setRemoteEnabled(enabled: boolean): Promise<RemoteHostStatus>;
  beginRemotePairing(): Promise<RemotePairing>;
  cancelRemotePairing(pairingId: string): Promise<RemoteHostStatus>;
  revokeRemoteDevice(deviceId: string): Promise<RemoteHostStatus>;
  subscribe(listener: (event: DesktopEvent) => void): () => void;
}
