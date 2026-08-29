import {
  mkdirSync,
  readFileSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { realpath, stat } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  type IpcMainInvokeEvent,
  nativeTheme,
  net,
  powerSaveBlocker,
  protocol,
  screen,
  session,
  shell,
  type Rectangle,
} from 'electron';
import { PadBackendClient } from './backend-client';
import { isDesktopAction, sanitizeDesktopParams } from './request-policy';
import {
  DESKTOP_IPC,
  type DesktopBootstrapResult,
  type DesktopRendererRequest,
} from '../../shared/protocol';
import { installContentSecurityPolicy } from './security';
import { installApplicationMenu } from './menu';
import { redactRemotePowerSignal, RemotePowerSaveCoordinator } from './remote-power-blocker';

declare const MAIN_WINDOW_VITE_DEV_SERVER_URL: string | undefined;
declare const MAIN_WINDOW_VITE_NAME: string;

const RENDERER_SCHEME = 'pad-app';
const RENDERER_HOST = 'renderer';
const RENDERER_ENTRY_URL = `${RENDERER_SCHEME}://${RENDERER_HOST}/index.html`;
const WINDOW_STATE_FILE = 'window-state.json';
const DEFAULT_WINDOW_BOUNDS = { width: 1280, height: 820 } as const;
const MIN_WINDOW_WIDTH = 480;
const MIN_WINDOW_HEIGHT = 600;

export interface PersistedWindowState {
  bounds: Rectangle;
  maximized: boolean;
  fullscreen: boolean;
}

function finiteInteger(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? Math.round(value) : null;
}

function intersectionSize(left: Rectangle, right: Rectangle): { width: number; height: number } {
  return {
    width: Math.max(0, Math.min(left.x + left.width, right.x + right.width) - Math.max(left.x, right.x)),
    height: Math.max(0, Math.min(left.y + left.height, right.y + right.height) - Math.max(left.y, right.y)),
  };
}

/** Validate and bring PAD's own saved bounds fully back inside a currently attached display. */
export function sanitizeWindowState(value: unknown, workAreas: Rectangle[]): PersistedWindowState | null {
  if (!value || typeof value !== 'object' || !workAreas.length) return null;
  const record = value as Record<string, unknown>;
  const rawBounds = record.bounds;
  if (!rawBounds || typeof rawBounds !== 'object') return null;
  const boundsRecord = rawBounds as Record<string, unknown>;
  const x = finiteInteger(boundsRecord.x);
  const y = finiteInteger(boundsRecord.y);
  const width = finiteInteger(boundsRecord.width);
  const height = finiteInteger(boundsRecord.height);
  if (
    x === null
    || y === null
    || width === null
    || height === null
    || width < MIN_WINDOW_WIDTH
    || height < MIN_WINDOW_HEIGHT
    || width > 16_384
    || height > 16_384
    || typeof record.maximized !== 'boolean'
    || typeof record.fullscreen !== 'boolean'
  ) return null;

  const candidate = { x, y, width, height };
  const ranked = workAreas
    .map((workArea) => ({ workArea, intersection: intersectionSize(candidate, workArea) }))
    .sort((left, right) => right.intersection.width * right.intersection.height - left.intersection.width * left.intersection.height);
  const target = ranked[0];
  if (!target || target.intersection.width < 64 || target.intersection.height < 64) return null;
  if (target.workArea.width < MIN_WINDOW_WIDTH || target.workArea.height < MIN_WINDOW_HEIGHT) return null;

  const safeWidth = Math.min(width, target.workArea.width);
  const safeHeight = Math.min(height, target.workArea.height);
  const safeX = Math.min(Math.max(x, target.workArea.x), target.workArea.x + target.workArea.width - safeWidth);
  const safeY = Math.min(Math.max(y, target.workArea.y), target.workArea.y + target.workArea.height - safeHeight);
  return {
    bounds: { x: safeX, y: safeY, width: safeWidth, height: safeHeight },
    maximized: record.maximized,
    fullscreen: record.fullscreen,
  };
}

export function loadWindowState(filePath: string, workAreas: Rectangle[]): PersistedWindowState | null {
  try {
    if (statSync(filePath).size > 16 * 1024) return null;
    return sanitizeWindowState(JSON.parse(readFileSync(filePath, 'utf8')) as unknown, workAreas);
  } catch {
    return null;
  }
}

/** A same-directory rename makes the tiny PAD-only JSON update atomic on macOS. */
export function saveWindowState(filePath: string, state: PersistedWindowState): void {
  const directory = path.dirname(filePath);
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  const temporaryPath = `${filePath}.${process.pid}.tmp`;
  writeFileSync(temporaryPath, `${JSON.stringify(state)}\n`, { encoding: 'utf8', mode: 0o600 });
  renameSync(temporaryPath, filePath);
}

protocol.registerSchemesAsPrivileged([
  {
    scheme: RENDERER_SCHEME,
    privileges: {
      standard: true,
      secure: true,
      bypassCSP: false,
      allowServiceWorkers: false,
      supportFetchAPI: false,
      corsEnabled: false,
      stream: false,
      codeCache: true,
      allowExtensions: false,
    },
  },
]);

const backend = new PadBackendClient();
const remotePower = new RemotePowerSaveCoordinator(powerSaveBlocker);
let mainWindow: BrowserWindow | null = null;
let quitAfterBackendStops = false;

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (!mainWindow) return;
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.show();
    mainWindow.focus();
  });

  app.whenReady().then(async () => {
    if (!MAIN_WINDOW_VITE_DEV_SERVER_URL) installRendererProtocol();
    installContentSecurityPolicy(session.defaultSession, Boolean(MAIN_WINDOW_VITE_DEV_SERVER_URL));
    installApplicationMenu();
    installIpcHandlers(MAIN_WINDOW_VITE_DEV_SERVER_URL);
    backend.on('event', (event) => {
      remotePower.observe(event);
      if (event.type === 'host_status' && event.status !== 'ready' && event.status !== 'starting') {
        remotePower.dispose();
      }
      const window = mainWindow;
      if (window && !window.isDestroyed()) {
        window.webContents.send(DESKTOP_IPC.event, redactRemotePowerSignal(event));
      }
    });
    backend.start();
    await createWindow();
  });
}

export function resolveRendererFilePath(
  requestUrl: string,
  rendererRoot: string,
): string | null {
  let parsed: URL;
  try {
    parsed = new URL(requestUrl);
  } catch {
    return null;
  }
  if (
    parsed.protocol !== `${RENDERER_SCHEME}:`
    || parsed.hostname !== RENDERER_HOST
    || parsed.port !== ''
    || parsed.username !== ''
    || parsed.password !== ''
  ) {
    return null;
  }

  let decodedPath: string;
  try {
    decodedPath = decodeURIComponent(parsed.pathname);
  } catch {
    return null;
  }
  if (decodedPath.includes('\0') || decodedPath.includes('\\')) return null;

  const relativePath = decodedPath.replace(/^\/+/, '');
  if (!relativePath) return null;
  const root = path.resolve(rendererRoot);
  const candidate = path.resolve(root, relativePath);
  const relative = path.relative(root, candidate);
  if (relative === '' || relative.startsWith(`..${path.sep}`) || relative === '..' || path.isAbsolute(relative)) {
    return null;
  }
  return candidate;
}

function isWithinRoot(root: string, candidate: string): boolean {
  const relative = path.relative(root, candidate);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

function notFoundResponse(): Response {
  return new Response('Not found', {
    status: 404,
    headers: { 'content-type': 'text/plain; charset=utf-8' },
  });
}

function installRendererProtocol(): void {
  const rendererRoot = path.join(
    app.getAppPath(),
    '.vite',
    'renderer',
    MAIN_WINDOW_VITE_NAME,
  );
  protocol.handle(RENDERER_SCHEME, async (request) => {
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      return new Response('Method not allowed', {
        status: 405,
        headers: { allow: 'GET, HEAD' },
      });
    }
    const candidate = resolveRendererFilePath(request.url, rendererRoot);
    if (!candidate) return notFoundResponse();
    try {
      const [canonicalRoot, canonicalCandidate, candidateStat] = await Promise.all([
        realpath(rendererRoot),
        realpath(candidate),
        stat(candidate),
      ]);
      if (!candidateStat.isFile() || !isWithinRoot(canonicalRoot, canonicalCandidate)) {
        return notFoundResponse();
      }
      const response = await net.fetch(pathToFileURL(canonicalCandidate).toString());
      if (request.method !== 'HEAD') return response;
      return new Response(null, {
        status: response.status,
        statusText: response.statusText,
        headers: response.headers,
      });
    } catch {
      return notFoundResponse();
    }
  });
}

async function createWindow(): Promise<void> {
  const windowStatePath = path.join(app.getPath('userData'), WINDOW_STATE_FILE);
  const restoredState = loadWindowState(
    windowStatePath,
    screen.getAllDisplays().map((display) => display.workArea),
  );
  const window = new BrowserWindow({
    width: restoredState?.bounds.width ?? DEFAULT_WINDOW_BOUNDS.width,
    height: restoredState?.bounds.height ?? DEFAULT_WINDOW_BOUNDS.height,
    x: restoredState?.bounds.x,
    y: restoredState?.bounds.y,
    minWidth: MIN_WINDOW_WIDTH,
    minHeight: MIN_WINDOW_HEIGHT,
    show: false,
    title: 'PAD Desktop',
    titleBarStyle: 'hiddenInset',
    trafficLightPosition: { x: 16, y: 16 },
    backgroundColor: nativeTheme.shouldUseDarkColors ? '#151515' : '#f7f7f5',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      webSecurity: true,
      webviewTag: false,
    },
  });
  mainWindow = window;
  if (restoredState?.maximized) window.maximize();
  if (restoredState?.fullscreen) window.setFullScreen(true);

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  const persistState = () => {
    if (window.isDestroyed()) return;
    try {
      saveWindowState(windowStatePath, {
        bounds: window.getNormalBounds(),
        maximized: window.isMaximized(),
        fullscreen: window.isFullScreen(),
      });
    } catch (error) {
      console.warn('Unable to persist PAD Desktop window state', error);
    }
  };
  const scheduleStateSave = () => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = null;
      persistState();
    }, 200);
  };
  window.on('move', scheduleStateSave);
  window.on('resize', scheduleStateSave);
  window.on('maximize', scheduleStateSave);
  window.on('unmaximize', scheduleStateSave);
  window.on('enter-full-screen', scheduleStateSave);
  window.on('leave-full-screen', scheduleStateSave);
  window.once('ready-to-show', () => window.show());
  window.on('close', () => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = null;
    persistState();
  });
  window.on('closed', () => {
    if (mainWindow === window) mainWindow = null;
  });
  window.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('https://')) void shell.openExternal(url);
    return { action: 'deny' };
  });
  window.webContents.on('will-navigate', (event, url) => {
    const current = window.webContents.getURL();
    if (current && url !== current) event.preventDefault();
  });

  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    await window.loadURL(MAIN_WINDOW_VITE_DEV_SERVER_URL);
  } else {
    await window.loadURL(RENDERER_ENTRY_URL);
  }
}

export function installIpcHandlers(developmentServerUrl: string | undefined): void {
  ipcMain.handle(DESKTOP_IPC.bootstrap, (event) => {
    assertTrustedIpcSender(event, developmentServerUrl);
    return backend.request<DesktopBootstrapResult>('bootstrap');
  });
  ipcMain.handle(DESKTOP_IPC.chooseProjectDirectory, async (event) => {
    assertTrustedIpcSender(event, developmentServerUrl);
    const selection = await dialog.showOpenDialog({
      properties: ['openDirectory', 'createDirectory'],
    });
    if (selection.canceled || selection.filePaths.length !== 1) return null;
    const [selectedPath] = selection.filePaths;
    if (!selectedPath || !path.isAbsolute(selectedPath) || selectedPath.includes('\0')) {
      return null;
    }
    return selectedPath;
  });
  ipcMain.handle(DESKTOP_IPC.chooseAttachments, async (event) => {
    assertTrustedIpcSender(event, developmentServerUrl);
    const selection = await dialog.showOpenDialog({
      properties: ['openFile', 'multiSelections'],
    });
    if (selection.canceled) return [];
    const unique = new Set<string>();
    for (const selectedPath of selection.filePaths) {
      if (unique.size >= 20) break;
      if (!selectedPath || !path.isAbsolute(selectedPath) || selectedPath.includes('\0')) continue;
      unique.add(selectedPath);
    }
    return [...unique];
  });
  ipcMain.handle(
    DESKTOP_IPC.request,
    (event, request: DesktopRendererRequest) => {
      assertTrustedIpcSender(event, developmentServerUrl);
      if (!request || !isDesktopAction(request.action)) {
        throw new Error('Unsupported PAD desktop action');
      }
      const response = backend.request(
        request.action,
        sanitizeDesktopParams(request.action, request.params),
      );
      if (!request.action.startsWith('remote_')) return response;
      return response.then((result) => {
        remotePower.observe(result);
        return result;
      });
    },
  );
}

export function isAllowedRendererUrl(
  candidateUrl: string,
  developmentServerUrl: string | undefined,
): boolean {
  let candidate: URL;
  try {
    candidate = new URL(candidateUrl);
  } catch {
    return false;
  }
  if (candidate.username !== '' || candidate.password !== '') return false;

  if (developmentServerUrl) {
    let development: URL;
    try {
      development = new URL(developmentServerUrl);
    } catch {
      return false;
    }
    if (
      !['http:', 'https:'].includes(development.protocol)
      || development.username !== ''
      || development.password !== ''
    ) {
      return false;
    }
    return candidate.origin === development.origin;
  }

  return candidate.protocol === `${RENDERER_SCHEME}:`
    && candidate.hostname === RENDERER_HOST
    && candidate.port === ''
    && candidate.pathname.startsWith('/');
}

export function assertTrustedIpcSender(
  event: IpcMainInvokeEvent,
  developmentServerUrl: string | undefined,
): void {
  const frame = event.senderFrame;
  const senderUrl = event.sender.getURL();
  if (
    frame === null
    || frame !== event.sender.mainFrame
    || !isAllowedRendererUrl(frame.url, developmentServerUrl)
    || !isAllowedRendererUrl(senderUrl, developmentServerUrl)
  ) {
    throw new Error('Untrusted PAD Desktop IPC sender');
  }
}

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) void createWindow();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('before-quit', (event) => {
  remotePower.dispose();
  if (quitAfterBackendStops) return;
  event.preventDefault();
  void backend.stop().finally(() => {
    quitAfterBackendStops = true;
    app.quit();
  });
});
