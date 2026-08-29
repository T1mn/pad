import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';

const electron = vi.hoisted(() => ({
  quit: vi.fn(),
  registeredSchemes: [] as unknown[],
  ipcHandlers: new Map<string, (...args: unknown[]) => unknown>(),
  showOpenDialog: vi.fn(),
}));

vi.mock('electron', () => ({
  app: {
    requestSingleInstanceLock: () => false,
    quit: electron.quit,
    on: vi.fn(),
  },
  BrowserWindow: class {},
  dialog: { showOpenDialog: electron.showOpenDialog },
  ipcMain: {
    handle: (channel: string, handler: (...args: unknown[]) => unknown) => {
      electron.ipcHandlers.set(channel, handler);
    },
  },
  Menu: { buildFromTemplate: vi.fn(), setApplicationMenu: vi.fn() },
  nativeTheme: { shouldUseDarkColors: false },
  net: { fetch: vi.fn() },
  powerSaveBlocker: { start: vi.fn(() => 1), stop: vi.fn() },
  protocol: {
    registerSchemesAsPrivileged: (schemes: unknown) => electron.registeredSchemes.push(schemes),
    handle: vi.fn(),
  },
  screen: { getAllDisplays: vi.fn(() => []) },
  session: { defaultSession: {} },
  shell: { openExternal: vi.fn() },
}));

import type { IpcMainInvokeEvent } from 'electron';
import {
  assertTrustedIpcSender,
  installIpcHandlers,
  isAllowedRendererUrl,
  loadWindowState,
  resolveRendererFilePath,
  sanitizeWindowState,
  saveWindowState,
} from './index';

describe('packaged renderer protocol', () => {
  const rendererRoot = path.resolve('/tmp/pad-renderer-root');

  it('registers a standard secure scheme without bypassing CSP', () => {
    expect(electron.registeredSchemes).toEqual([[
      expect.objectContaining({
        scheme: 'pad-app',
        privileges: expect.objectContaining({
          standard: true,
          secure: true,
          bypassCSP: false,
          allowServiceWorkers: false,
          supportFetchAPI: false,
          allowExtensions: false,
        }),
      }),
    ]]);
  });

  it('allows only the production scheme or the configured development origin', () => {
    expect(isAllowedRendererUrl('pad-app://renderer/index.html', undefined)).toBe(true);
    expect(isAllowedRendererUrl('pad-app://renderer/assets/app.js?x=1', undefined)).toBe(true);
    expect(isAllowedRendererUrl('pad-app://renderer', undefined)).toBe(false);
    expect(isAllowedRendererUrl('pad-app://renderer.evil/index.html', undefined)).toBe(false);
    expect(isAllowedRendererUrl('https://renderer/index.html', undefined)).toBe(false);
    expect(isAllowedRendererUrl('pad-app://user@renderer/index.html', undefined)).toBe(false);

    const development = 'http://127.0.0.1:5173/renderer';
    expect(isAllowedRendererUrl('http://127.0.0.1:5173/', development)).toBe(true);
    expect(isAllowedRendererUrl('http://127.0.0.1:5173/src/main.tsx', development)).toBe(true);
    expect(isAllowedRendererUrl('http://localhost:5173/', development)).toBe(false);
    expect(isAllowedRendererUrl('http://127.0.0.1:5174/', development)).toBe(false);
    expect(isAllowedRendererUrl('https://127.0.0.1:5173/', development)).toBe(false);
    expect(isAllowedRendererUrl('http://user@127.0.0.1:5173/', development)).toBe(false);
  });

  it('requires both IPC URLs to be trusted and the sender to be the main frame', () => {
    const trustedFrame = { url: 'pad-app://renderer/index.html' };
    const trusted = {
      senderFrame: trustedFrame,
      sender: { mainFrame: trustedFrame, getURL: () => 'pad-app://renderer/index.html' },
    } as unknown as IpcMainInvokeEvent;
    expect(() => assertTrustedIpcSender(trusted, undefined)).not.toThrow();

    const cases = [
      {
        senderFrame: null,
        sender: { mainFrame: trustedFrame, getURL: () => 'pad-app://renderer/index.html' },
      },
      {
        senderFrame: { url: 'pad-app://renderer/index.html' },
        sender: { mainFrame: trustedFrame, getURL: () => 'pad-app://renderer/index.html' },
      },
      {
        senderFrame: { url: 'https://evil.invalid/' },
        sender: { mainFrame: trustedFrame, getURL: () => 'pad-app://renderer/index.html' },
      },
      {
        senderFrame: trustedFrame,
        sender: { mainFrame: trustedFrame, getURL: () => 'https://evil.invalid/' },
      },
    ];
    for (const event of cases) {
      expect(() =>
        assertTrustedIpcSender(event as unknown as IpcMainInvokeEvent, undefined),
      ).toThrow('Untrusted PAD Desktop IPC sender');
    }

    const developmentFrame = { url: 'http://127.0.0.1:5173/index.html' };
    const developmentEvent = {
      senderFrame: developmentFrame,
      sender: {
        mainFrame: developmentFrame,
        getURL: () => 'http://127.0.0.1:5173/index.html',
      },
    } as unknown as IpcMainInvokeEvent;
    expect(() =>
      assertTrustedIpcSender(developmentEvent, 'http://127.0.0.1:5173/'),
    ).not.toThrow();
    expect(() => assertTrustedIpcSender(developmentEvent, 'http://localhost:5173/')).toThrow(
      'Untrusted PAD Desktop IPC sender',
    );
  });

  it('guards both bootstrap and request IPC handlers before dispatch', () => {
    electron.ipcHandlers.clear();
    installIpcHandlers(undefined);
    const untrustedFrame = { url: 'https://evil.invalid/' };
    const untrustedEvent = {
      senderFrame: untrustedFrame,
      sender: { mainFrame: untrustedFrame, getURL: () => 'https://evil.invalid/' },
    } as unknown as IpcMainInvokeEvent;

    const bootstrap = electron.ipcHandlers.get('pad-desktop:bootstrap');
    const request = electron.ipcHandlers.get('pad-desktop:request');
    expect(bootstrap).toBeTypeOf('function');
    expect(request).toBeTypeOf('function');
    expect(() => bootstrap?.(untrustedEvent)).toThrow('Untrusted PAD Desktop IPC sender');
    expect(() => request?.(untrustedEvent, { action: 'ping' })).toThrow(
      'Untrusted PAD Desktop IPC sender',
    );
  });

  it('returns exactly one explicitly selected project directory and maps cancellation to null', async () => {
    electron.ipcHandlers.clear();
    installIpcHandlers(undefined);
    const trustedFrame = { url: 'pad-app://renderer/index.html' };
    const trustedEvent = {
      senderFrame: trustedFrame,
      sender: { mainFrame: trustedFrame, getURL: () => 'pad-app://renderer/index.html' },
    } as unknown as IpcMainInvokeEvent;
    const choose = electron.ipcHandlers.get('pad-desktop:choose-project-directory');
    expect(choose).toBeTypeOf('function');

    electron.showOpenDialog
      .mockResolvedValueOnce({ canceled: true, filePaths: [] })
      .mockResolvedValueOnce({ canceled: false, filePaths: ['/tmp/项目'] })
      .mockResolvedValueOnce({ canceled: false, filePaths: ['/tmp/one', '/tmp/two'] });

    await expect(choose?.(trustedEvent)).resolves.toBeNull();
    await expect(choose?.(trustedEvent)).resolves.toBe('/tmp/项目');
    await expect(choose?.(trustedEvent)).resolves.toBeNull();
    expect(electron.showOpenDialog).toHaveBeenCalledTimes(3);
    expect(electron.showOpenDialog).toHaveBeenCalledWith({
      properties: ['openDirectory', 'createDirectory'],
    });
  });

  it('rejects an untrusted directory-picker sender before opening native UI', async () => {
    electron.ipcHandlers.clear();
    installIpcHandlers(undefined);
    const untrustedFrame = { url: 'https://evil.invalid/' };
    const untrustedEvent = {
      senderFrame: untrustedFrame,
      sender: { mainFrame: untrustedFrame, getURL: () => 'https://evil.invalid/' },
    } as unknown as IpcMainInvokeEvent;
    const choose = electron.ipcHandlers.get('pad-desktop:choose-project-directory');

    await expect(choose?.(untrustedEvent)).rejects.toThrow(
      'Untrusted PAD Desktop IPC sender',
    );
    expect(electron.showOpenDialog).not.toHaveBeenCalled();
  });

  it('returns at most twenty unique absolute attachment paths from the native picker', async () => {
    electron.ipcHandlers.clear();
    electron.showOpenDialog.mockClear();
    installIpcHandlers(undefined);
    const trustedFrame = { url: 'pad-app://renderer/index.html' };
    const trustedEvent = {
      senderFrame: trustedFrame,
      sender: { mainFrame: trustedFrame, getURL: () => 'pad-app://renderer/index.html' },
    } as unknown as IpcMainInvokeEvent;
    const choose = electron.ipcHandlers.get('pad-desktop:choose-attachments');
    expect(choose).toBeTypeOf('function');
    const many = Array.from({ length: 24 }, (_, index) => `/tmp/file-${index}.txt`);
    electron.showOpenDialog
      .mockResolvedValueOnce({ canceled: true, filePaths: [] })
      .mockResolvedValueOnce({ canceled: false, filePaths: ['/tmp/a.txt', '/tmp/a.txt', 'relative.txt', ...many] });

    await expect(choose?.(trustedEvent)).resolves.toEqual([]);
    const selected = await choose?.(trustedEvent) as string[];
    expect(selected).toHaveLength(20);
    expect(selected[0]).toBe('/tmp/a.txt');
    expect(new Set(selected).size).toBe(selected.length);
    expect(electron.showOpenDialog).toHaveBeenLastCalledWith({
      properties: ['openFile', 'multiSelections'],
    });
  });

  it('maps only renderer-host resources beneath the renderer root', () => {
    expect(resolveRendererFilePath('pad-app://renderer/index.html', rendererRoot)).toBe(
      path.join(rendererRoot, 'index.html'),
    );
    expect(
      resolveRendererFilePath('pad-app://renderer/assets/main.js?v=1', rendererRoot),
    ).toBe(path.join(rendererRoot, 'assets/main.js'));
    expect(resolveRendererFilePath('pad-app://other/index.html', rendererRoot)).toBeNull();
    expect(resolveRendererFilePath('https://renderer/index.html', rendererRoot)).toBeNull();
    expect(resolveRendererFilePath('pad-app://user@renderer/index.html', rendererRoot)).toBeNull();
  });

  it('rejects traversal, backslashes, NULs, malformed escapes, and directory roots', () => {
    for (const requestUrl of [
      'pad-app://renderer/%2F..%2Fsecret',
      'pad-app://renderer/%5C..%5Csecret',
      'pad-app://renderer/a%00b',
      'pad-app://renderer/%ZZ',
      'pad-app://renderer/',
      'pad-app://renderer/.',
    ]) {
      expect(resolveRendererFilePath(requestUrl, rendererRoot), requestUrl).toBeNull();
    }
  });
});

describe('PAD-only persisted window state', () => {
  const workAreas = [
    { x: 0, y: 25, width: 1440, height: 875 },
    { x: -1280, y: 0, width: 1280, height: 800 },
  ];

  it('restores valid multi-display bounds and clamps oversized bounds into the visible work area', () => {
    expect(sanitizeWindowState({
      bounds: { x: -1200, y: 40, width: 1000, height: 700 },
      maximized: true,
      fullscreen: false,
    }, workAreas)).toEqual({
      bounds: { x: -1200, y: 40, width: 1000, height: 700 },
      maximized: true,
      fullscreen: false,
    });

    expect(sanitizeWindowState({
      bounds: { x: 1200, y: 700, width: 1800, height: 1000 },
      maximized: false,
      fullscreen: false,
    }, workAreas)).toEqual({
      bounds: { x: 0, y: 25, width: 1440, height: 875 },
      maximized: false,
      fullscreen: false,
    });
  });

  it('rejects off-screen, undersized, malformed and non-boolean window state', () => {
    expect(sanitizeWindowState({
      bounds: { x: 4000, y: 4000, width: 800, height: 700 },
      maximized: false,
      fullscreen: false,
    }, workAreas)).toBeNull();
    expect(sanitizeWindowState({
      bounds: { x: 0, y: 25, width: 320, height: 700 },
      maximized: false,
      fullscreen: false,
    }, workAreas)).toBeNull();
    expect(sanitizeWindowState({
      bounds: { x: 0, y: 25, width: 800, height: 700 },
      maximized: 'yes',
      fullscreen: false,
    }, workAreas)).toBeNull();
  });

  it('writes one atomic JSON file beneath the supplied PAD userData path and safely ignores corruption', () => {
    const directory = mkdtempSync(path.join(os.tmpdir(), 'pad-window-state-test-'));
    const filePath = path.join(directory, 'window-state.json');
    const state = {
      bounds: { x: 40, y: 50, width: 1000, height: 720 },
      maximized: false,
      fullscreen: true,
    };
    try {
      saveWindowState(filePath, state);
      expect(JSON.parse(readFileSync(filePath, 'utf8'))).toEqual(state);
      expect(readdirSync(directory)).toEqual(['window-state.json']);
      expect(loadWindowState(filePath, workAreas)).toEqual(state);

      writeFileSync(filePath, '{broken', 'utf8');
      expect(loadWindowState(filePath, workAreas)).toBeNull();
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});
