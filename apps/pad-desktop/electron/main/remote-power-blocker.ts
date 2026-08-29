export interface PowerSaveBlockerPort {
  start(type: 'prevent-app-suspension'): number;
  stop(id: number): void;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function remoteRecord(value: unknown): Record<string, unknown> | null {
  if (!isRecord(value)) return null;
  if (isRecord(value.remote)) return value.remote;
  if (value.type === 'backend_event') return remoteRecord(value.payload);
  if (isRecord(value.event) && value.event.kind === 'remote_changed') {
    return remoteRecord(value.event.payload);
  }
  if (value.kind === 'remote_changed') return remoteRecord(value.payload);
  return null;
}

function globalOnlineSignal(value: unknown): boolean | null {
  if (!isRecord(value)) return null;
  if (typeof value.has_online_remote === 'boolean') return value.has_online_remote;
  if (value.type === 'backend_event') return globalOnlineSignal(value.payload);
  if (isRecord(value.event) && value.event.kind === 'remote_changed') {
    return globalOnlineSignal(value.event.payload);
  }
  if (value.kind === 'remote_changed') return globalOnlineSignal(value.payload);
  return null;
}

/** Remove host-global liveness metadata before a Profile-scoped event reaches renderer code. */
export function redactRemotePowerSignal<T>(value: T): T {
  if (!isRecord(value)) return value;
  if (Object.prototype.hasOwnProperty.call(value, 'has_online_remote')) {
    const { has_online_remote: _privateSignal, ...safe } = value;
    return safe as T;
  }
  if (value.type === 'backend_event' && isRecord(value.payload)) {
    return { ...value, payload: redactRemotePowerSignal(value.payload) } as T;
  }
  if (isRecord(value.event) && value.event.kind === 'remote_changed') {
    return {
      ...value,
      event: { ...value.event, payload: redactRemotePowerSignal(value.event.payload) },
    } as T;
  }
  if (value.kind === 'remote_changed' && isRecord(value.payload)) {
    return { ...value, payload: redactRemotePowerSignal(value.payload) } as T;
  }
  return value;
}

/**
 * Keeps the Mac active only while a paired device is online. Electron's
 * `prevent-app-suspension` still allows the display to turn off.
 */
export class RemotePowerSaveCoordinator {
  private blockerId: number | null = null;
  private hasOnlineRemote: boolean | null = null;

  constructor(private readonly blocker: PowerSaveBlockerPort) {}

  observe(value: unknown): void {
    const remote = remoteRecord(value);
    if (!remote || typeof remote.enabled !== 'boolean') return;
    const globalOnline = globalOnlineSignal(value);
    if (globalOnline !== null) this.hasOnlineRemote = globalOnline;
    const activeConnections = typeof remote.active_connections === 'number'
      && Number.isFinite(remote.active_connections)
      ? Math.max(0, Math.trunc(remote.active_connections))
      : 0;
    const hasOnlineDevice = Array.isArray(remote.devices)
      && remote.devices.some((device) => isRecord(device) && device.online === true);
    if (!remote.enabled) this.hasOnlineRemote = false;
    const active = this.hasOnlineRemote ?? (activeConnections > 0 || hasOnlineDevice);
    this.setActive(remote.enabled && active);
  }

  dispose(): void {
    this.hasOnlineRemote = null;
    this.setActive(false);
  }

  private setActive(active: boolean): void {
    if (active && this.blockerId === null) {
      try {
        this.blockerId = this.blocker.start('prevent-app-suspension');
      } catch {
        this.blockerId = null;
      }
      return;
    }
    if (!active && this.blockerId !== null) {
      const id = this.blockerId;
      this.blockerId = null;
      try {
        this.blocker.stop(id);
      } catch {
        // The OS may already have released the blocker during shutdown.
      }
    }
  }
}
