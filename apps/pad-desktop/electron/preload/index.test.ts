import { describe, expect, it, vi } from 'vitest';

const electron = vi.hoisted(() => ({
  exposedName: '' as string,
  exposedApi: null as Record<string, unknown> | null,
  invoke: vi.fn(),
}));

vi.mock('electron', () => ({
  contextBridge: {
    exposeInMainWorld: (name: string, api: Record<string, unknown>) => {
      electron.exposedName = name;
      electron.exposedApi = api;
    },
  },
  ipcRenderer: {
    invoke: electron.invoke,
    on: vi.fn(),
    removeListener: vi.fn(),
  },
}));

import './index';

describe('PAD Desktop preload directory picker', () => {
  it('exposes a frozen API that invokes only the dedicated picker channel', async () => {
    electron.invoke.mockResolvedValueOnce('/tmp/项目');
    expect(electron.exposedName).toBe('padDesktop');
    expect(electron.exposedApi).not.toBeNull();
    expect(Object.isFrozen(electron.exposedApi)).toBe(true);

    const chooseProjectDirectory = electron.exposedApi?.chooseProjectDirectory;
    expect(chooseProjectDirectory).toBeTypeOf('function');
    await expect(
      (chooseProjectDirectory as () => Promise<string | null>)(),
    ).resolves.toBe('/tmp/项目');
    expect(electron.invoke).toHaveBeenCalledWith('pad-desktop:choose-project-directory');

    electron.invoke.mockResolvedValueOnce(['/tmp/a.txt']);
    const chooseAttachments = electron.exposedApi?.chooseAttachments;
    expect(chooseAttachments).toBeTypeOf('function');
    await expect((chooseAttachments as () => Promise<string[]>)()).resolves.toEqual(['/tmp/a.txt']);
    expect(electron.invoke).toHaveBeenCalledWith('pad-desktop:choose-attachments');
  });
});
