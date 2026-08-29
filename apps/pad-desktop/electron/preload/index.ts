import { contextBridge, ipcRenderer } from 'electron';
import {
  DESKTOP_IPC,
  type DesktopAction,
  type DesktopBootstrapResult,
  type DesktopEvent,
  type DesktopRequestParams,
  type PadDesktopApi,
} from '../../shared/protocol';

const api: PadDesktopApi = {
  bootstrap: () => ipcRenderer.invoke(DESKTOP_IPC.bootstrap) as Promise<DesktopBootstrapResult>,
  chooseProjectDirectory: () =>
    ipcRenderer.invoke(DESKTOP_IPC.chooseProjectDirectory) as Promise<string | null>,
  chooseAttachments: () =>
    ipcRenderer.invoke(DESKTOP_IPC.chooseAttachments) as Promise<string[]>,
  request: <A extends DesktopAction, T = unknown>(
    action: A,
    params: DesktopRequestParams[A],
  ) => ipcRenderer.invoke(DESKTOP_IPC.request, { action, params }) as Promise<T>,
  subscribe: (listener: (event: DesktopEvent) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, payload: DesktopEvent) => listener(payload);
    ipcRenderer.on(DESKTOP_IPC.event, handler);
    return () => ipcRenderer.removeListener(DESKTOP_IPC.event, handler);
  },
};

contextBridge.exposeInMainWorld('padDesktop', Object.freeze(api));
