import { createHash, randomBytes, randomUUID, timingSafeEqual, X509Certificate } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { createServer, type Server as HttpsServer } from 'node:https';
import { hostname, networkInterfaces } from 'node:os';
import path from 'node:path';
import { WebSocket, WebSocketServer } from 'ws';
import type {
  DesktopServerEvent,
  RemoteDeviceDto,
  RemoteHostStatusDto,
  RemotePairBeginResultDto,
} from '../../shared/protocol';

interface StoredDevice extends RemoteDeviceDto {
  token_hash: string;
}

interface RemoteDiskState {
  enabled: boolean;
  devices: StoredDevice[];
}

interface PairingTicket {
  id: string;
  secret: string;
  expiresAt: number;
}

interface ConnectedClient {
  socket: WebSocket;
  deviceId: string;
  alive: boolean;
}

type RemoteExecutor = (action: string, params: Record<string, unknown>) => Promise<unknown>;

const REMOTE_ACTIONS = new Set([
  'bootstrap', 'list_sidebar', 'history', 'create_task', 'start_task', 'prompt',
  'abort', 'stop', 'stop_task', 'retry_task', 'respond_ui', 'set_task', 'runtime_snapshot',
]);
const REMOTE_PROTOCOL = 'pad.remote.v1';
const PAIRING_TTL_MS = 2 * 60_000;
const MAX_FRAME_BYTES = 1024 * 1024;

function hashToken(token: string): string {
  return createHash('sha256').update(token).digest('hex');
}

function sameHash(left: string, right: string): boolean {
  try {
    const a = Buffer.from(left, 'hex');
    const b = Buffer.from(right, 'hex');
    return a.length === b.length && timingSafeEqual(a, b);
  } catch {
    return false;
  }
}

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function localAddress(): string {
  const interfaces = networkInterfaces();
  const names = Object.keys(interfaces).sort((left, right) => {
    const rank = (name: string) => name === 'en0' ? 0 : name === 'en1' ? 1 : name.startsWith('en') ? 2 : 3;
    return rank(left) - rank(right);
  });
  for (const name of names) {
    for (const address of interfaces[name] ?? []) {
      if (address.family === 'IPv4' && !address.internal && !address.address.startsWith('169.254.')) return address.address;
    }
  }
  return '127.0.0.1';
}

function wireError(code: string, message: string): { code: string; message: string } {
  return { code, message };
}

export class RemoteGateway {
  private readonly directory: string;
  private readonly stateFile: string;
  private readonly certificateFile: string;
  private readonly privateKeyFile: string;
  private state: RemoteDiskState;
  private server: WebSocketServer | null = null;
  private httpsServer: HttpsServer | null = null;
  private port = 0;
  private fingerprint = '';
  private pairing: PairingTicket | null = null;
  private readonly clients = new Map<WebSocket, ConnectedClient>();
  private heartbeat: NodeJS.Timeout | null = null;
  private lastError: string | undefined;
  private readonly serverEpoch = randomUUID();
  private revision = 0;

  constructor(
    dataRoot: string,
    private readonly execute: RemoteExecutor,
    private readonly changed: () => void,
  ) {
    this.directory = path.join(dataRoot, 'v1', 'remote');
    this.stateFile = path.join(this.directory, 'state.json');
    this.certificateFile = path.join(this.directory, 'remote-cert.pem');
    this.privateKeyFile = path.join(this.directory, 'remote-key.pem');
    mkdirSync(this.directory, { recursive: true, mode: 0o700 });
    this.state = this.load();
  }

  async startIfEnabled(): Promise<void> {
    if (this.state.enabled) await this.enable();
  }

  async setEnabled(enabled: boolean): Promise<RemoteHostStatusDto> {
    this.state.enabled = enabled;
    this.persist();
    if (enabled) await this.enable();
    else await this.disable();
    this.changed();
    return this.status();
  }

  status(): RemoteHostStatusDto {
    const online = new Set([...this.clients.values()].map((client) => client.deviceId));
    return {
      enabled: this.state.enabled,
      state: !this.state.enabled ? 'disabled' : this.server ? (this.lastError ? 'degraded' : 'ready') : 'starting',
      display_name: hostname(),
      active_connections: this.clients.size,
      devices: this.state.devices.map(({ token_hash: _tokenHash, ...device }) => ({ ...device, online: online.has(device.id) })),
      updated_at: Date.now(),
      error_code: this.lastError,
    };
  }

  beginPairing(): RemotePairBeginResultDto {
    if (!this.server || !this.state.enabled || !this.fingerprint) throw new Error('Enable remote connection before pairing');
    const ticket: PairingTicket = {
      id: randomUUID(),
      secret: randomBytes(32).toString('base64url'),
      expiresAt: Date.now() + PAIRING_TTL_MS,
    };
    this.pairing = ticket;
    const params = new URLSearchParams({
      v: '1',
      endpoint: `wss://${localAddress()}:${this.port}`,
      fingerprint: this.fingerprint,
      pairing_id: ticket.id,
      secret: ticket.secret,
    });
    return {
      pairing: {
        pairing_id: ticket.id,
        qr_payload: `pad://remote/pair?${params.toString()}`,
        expires_at: ticket.expiresAt,
      },
    };
  }

  cancelPairing(id: string): RemoteHostStatusDto {
    if (this.pairing?.id === id) this.pairing = null;
    return this.status();
  }

  revokeDevice(id: string): RemoteHostStatusDto {
    this.state.devices = this.state.devices.filter((device) => device.id !== id);
    for (const client of this.clients.values()) {
      if (client.deviceId === id) client.socket.close(4003, 'revoked');
    }
    this.persist();
    this.changed();
    return this.status();
  }

  broadcast(payload: unknown): void {
    const event = record(payload) as DesktopServerEvent | null;
    const inner = event?.type === 'desktop_event' ? record(event.event) : null;
    const eventPayload = inner ? inner.payload : payload;
    const frame = JSON.stringify({
      type: 'event',
      server_epoch: this.serverEpoch,
      revision: ++this.revision,
      kind: 'invalidated',
      payload: eventPayload,
    });
    for (const client of this.clients.values()) {
      if (client.socket.readyState === WebSocket.OPEN) client.socket.send(frame);
    }
  }

  async stop(): Promise<void> {
    await this.disable(false);
  }

  private ensureCertificate(): void {
    if (!existsSync(this.certificateFile) || !existsSync(this.privateKeyFile)) {
      execFileSync('/usr/bin/openssl', [
        'req', '-x509', '-newkey', 'rsa:2048', '-sha256', '-nodes', '-days', '3650',
        '-subj', '/CN=PAD Desktop Remote',
        '-keyout', this.privateKeyFile,
        '-out', this.certificateFile,
        '-addext', 'subjectAltName=DNS:localhost,IP:127.0.0.1',
      ], { stdio: 'ignore' });
    }
    const certificate = readFileSync(this.certificateFile, 'utf8');
    this.fingerprint = createHash('sha256').update(new X509Certificate(certificate).raw).digest('hex');
  }

  private async enable(): Promise<void> {
    if (this.server) return;
    this.lastError = undefined;
    try {
      this.ensureCertificate();
      const httpsServer = createServer({
        cert: readFileSync(this.certificateFile),
        key: readFileSync(this.privateKeyFile),
      });
      const server = new WebSocketServer({
        server: httpsServer,
        maxPayload: MAX_FRAME_BYTES,
        handleProtocols: (protocols) => protocols.has(REMOTE_PROTOCOL) ? REMOTE_PROTOCOL : false,
      });
      this.httpsServer = httpsServer;
      this.server = server;
      server.on('connection', (socket) => this.accept(socket));
      const onError = (error: Error) => {
        this.lastError = (error as NodeJS.ErrnoException).code || 'remote_listener_failed';
        this.changed();
      };
      server.on('error', onError);
      httpsServer.on('error', onError);
      await new Promise<void>((resolve, reject) => {
        httpsServer.once('listening', resolve);
        httpsServer.once('error', reject);
        httpsServer.listen(0, '0.0.0.0');
      });
      const address = httpsServer.address();
      this.port = typeof address === 'object' && address ? address.port : 0;
      this.heartbeat = setInterval(() => {
        for (const client of this.clients.values()) {
          if (!client.alive) {
            client.socket.terminate();
            continue;
          }
          client.alive = false;
          client.socket.ping();
        }
      }, 15_000);
      this.heartbeat.unref();
    } catch (error) {
      this.server = null;
      this.httpsServer = null;
      this.lastError = (error as NodeJS.ErrnoException).code || 'remote_start_failed';
      throw error;
    }
  }

  private async disable(updateState = true): Promise<void> {
    if (updateState) {
      this.state.enabled = false;
      this.persist();
    }
    if (this.heartbeat) clearInterval(this.heartbeat);
    this.heartbeat = null;
    for (const client of this.clients.values()) client.socket.close(1001, 'host disabled');
    this.clients.clear();
    const server = this.server;
    const httpsServer = this.httpsServer;
    this.server = null;
    this.httpsServer = null;
    this.port = 0;
    this.pairing = null;
    if (server) await new Promise<void>((resolve) => server.close(() => resolve()));
    if (httpsServer?.listening) await new Promise<void>((resolve) => httpsServer.close(() => resolve()));
  }

  private accept(socket: WebSocket): void {
    let authenticated = false;
    const timeout = setTimeout(() => socket.close(4001, 'authentication timeout'), 10_000);
    socket.on('pong', () => {
      const client = this.clients.get(socket);
      if (client) client.alive = true;
    });
    socket.on('message', async (data) => {
      let message: Record<string, unknown> | null = null;
      try { message = record(JSON.parse(data.toString())); } catch { /* rejected below */ }
      if (!message) {
        this.send(socket, { type: 'error', error: wireError('invalid_message', 'Mac 收到了无效远程消息。') });
        return;
      }
      if (!authenticated) {
        const device = this.authenticate(message);
        if (!device) {
          this.send(socket, { type: 'error', error: wireError('resume_rejected', '设备凭据无效，请重新配对。') });
          socket.close(4003, 'authentication failed');
          return;
        }
        authenticated = true;
        clearTimeout(timeout);
        this.clients.set(socket, { socket, deviceId: device.id, alive: true });
        this.send(socket, device.response);
        this.changed();
        return;
      }
      if (message.type === 'ping') {
        this.send(socket, { type: 'pong' });
        return;
      }
      if (message.type === 'pong' || message.type === 'ack') return;
      const commandId = typeof message.command_id === 'string'
        ? message.command_id
        : typeof message.id === 'string' ? message.id : randomUUID();
      const action = typeof message.action === 'string' ? message.action : '';
      if (!REMOTE_ACTIONS.has(action)) {
        this.send(socket, {
          type: 'command_result', command_id: commandId, ok: false,
          error: wireError('unsupported_action', '移动端不能执行这个操作。'),
        });
        return;
      }
      try {
        const result = await this.execute(action, record(message.params) ?? {});
        this.send(socket, { type: 'command_result', command_id: commandId, ok: true, result });
      } catch (error) {
        this.send(socket, {
          type: 'command_result', command_id: commandId, ok: false,
          error: wireError('command_failed', error instanceof Error ? error.message : String(error)),
        });
      }
    });
    socket.once('close', () => {
      clearTimeout(timeout);
      const client = this.clients.get(socket);
      this.clients.delete(socket);
      if (client) {
        const device = this.state.devices.find((candidate) => candidate.id === client.deviceId);
        if (device) device.last_seen_at = Date.now();
        this.persist();
        this.changed();
      }
    });
  }

  private authenticate(message: Record<string, unknown>): { id: string; response: Record<string, unknown> } | null {
    if (message.type === 'pair') {
      const pairingId = typeof message.pairing_id === 'string' ? message.pairing_id : '';
      const secret = typeof message.secret === 'string'
        ? message.secret
        : typeof message.token === 'string' ? message.token : '';
      const ticket = this.pairing;
      if (!ticket || ticket.id !== pairingId || ticket.expiresAt < Date.now() || ticket.secret !== secret) return null;
      this.pairing = null;
      const descriptor = record(message.device);
      const deviceToken = randomBytes(32).toString('base64url');
      const device: StoredDevice = {
        id: randomUUID(),
        display_name: typeof descriptor?.display_name === 'string'
          ? descriptor.display_name.slice(0, 80)
          : typeof message.display_name === 'string' ? message.display_name.slice(0, 80) : 'iPhone',
        platform: typeof descriptor?.platform === 'string'
          ? descriptor.platform.slice(0, 40)
          : typeof message.platform === 'string' ? message.platform.slice(0, 40) : 'ios',
        online: true,
        paired_at: Date.now(),
        last_seen_at: Date.now(),
        token_hash: hashToken(deviceToken),
      };
      this.state.devices.push(device);
      this.persist();
      return {
        id: device.id,
        response: {
          type: 'paired', device_id: device.id, device_token: deviceToken,
          server_epoch: this.serverEpoch, latest_revision: this.revision, profile_available: true,
        },
      };
    }
    if (message.type === 'resume') {
      const id = typeof message.device_id === 'string' ? message.device_id : '';
      const token = typeof message.device_token === 'string' ? message.device_token : '';
      const device = this.state.devices.find((candidate) => candidate.id === id);
      if (!device || !sameHash(device.token_hash, hashToken(token))) return null;
      device.last_seen_at = Date.now();
      this.persist();
      return {
        id,
        response: {
          type: 'welcome', server_epoch: this.serverEpoch,
          latest_revision: this.revision, profile_available: true,
        },
      };
    }
    return null;
  }

  private send(socket: WebSocket, payload: Record<string, unknown>): void {
    if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(payload));
  }

  private load(): RemoteDiskState {
    if (!existsSync(this.stateFile)) return { enabled: false, devices: [] };
    try {
      const value = JSON.parse(readFileSync(this.stateFile, 'utf8')) as RemoteDiskState;
      return { enabled: value.enabled === true, devices: Array.isArray(value.devices) ? value.devices : [] };
    } catch {
      return { enabled: false, devices: [] };
    }
  }

  private persist(): void {
    const temporary = `${this.stateFile}.${process.pid}.tmp`;
    writeFileSync(temporary, `${JSON.stringify(this.state, null, 2)}\n`, { encoding: 'utf8', mode: 0o600 });
    renameSync(temporary, this.stateFile);
  }
}
