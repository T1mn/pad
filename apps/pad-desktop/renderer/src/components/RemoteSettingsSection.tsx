import { useEffect, useState } from "react";
import type { RemoteHostState, RemoteHostStatus, RemotePairing } from "../types";
import { Icon } from "./Icons";
import { RemotePairingSheet } from "./RemotePairingSheet";

const stateLabels: Record<RemoteHostState, string> = {
  disabled: "已关闭",
  starting: "正在启动",
  ready: "可以连接",
  degraded: "连接受限",
  failed: "暂时不可用",
};

function epochMilliseconds(value: number): number {
  return value < 1_000_000_000_000 ? value * 1_000 : value;
}

function seenLabel(value?: number): string {
  if (!value) return "从未连接";
  const elapsed = Math.max(0, Date.now() - epochMilliseconds(value));
  if (elapsed < 60_000) return "刚刚在线";
  if (elapsed < 3_600_000) return `${Math.floor(elapsed / 60_000)} 分钟前在线`;
  if (elapsed < 86_400_000) return `${Math.floor(elapsed / 3_600_000)} 小时前在线`;
  return new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric" }).format(epochMilliseconds(value));
}

export function RemoteSettingsSection({
  capabilities,
  status,
  onRefresh,
  onEnabledChange,
  onBeginPairing,
  onCancelPairing,
  onRevokeDevice,
}: {
  capabilities: string[];
  status: RemoteHostStatus | null;
  onRefresh(): Promise<void>;
  onEnabledChange(enabled: boolean): Promise<void>;
  onBeginPairing(): Promise<RemotePairing>;
  onCancelPairing(pairingId: string): Promise<void>;
  onRevokeDevice(deviceId: string): Promise<void>;
}) {
  const remoteSupported = capabilities.includes("remote_gateway_v1");
  const pairingSupported = capabilities.includes("remote_pairing");
  const deviceManagementSupported = capabilities.includes("remote_device_management");
  const [busy, setBusy] = useState(false);
  const [pairingOpen, setPairingOpen] = useState(false);
  const [confirmDeviceId, setConfirmDeviceId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!remoteSupported || status) return;
    void onRefresh().catch(() => setError("无法读取远程连接状态，请稍后重试。"));
  }, [remoteSupported, status, onRefresh]);

  async function run(action: () => Promise<void>, fallback: string) {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch {
      setError(fallback);
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <div className="settings-heading">
        <h1>远程连接</h1>
        <p>让已配对的 iPhone 实时查看并继续这台 Mac 上的 PAD 对话；断线后会自动恢复。</p>
      </div>
      {!remoteSupported ? (
        <div className="settings-inline-notice" role="status">
          <Icon name="archive" />
          <span>当前 PAD 控制面没有远程网关能力。升级本地服务后，这里的开关与配对入口才会启用。</span>
        </div>
      ) : (
        <>
          <section className="settings-card remote-status-card">
            <div className="settings-row">
              <div><strong>允许远程连接</strong><p>仅接受已配对设备；关闭后立即停止新的连接。</p></div>
              <div className="settings-control">
                <label className={`switch${busy || !status ? " is-disabled" : ""}`} aria-label="允许远程连接">
                  <input
                    type="checkbox"
                    checked={status?.enabled ?? false}
                    disabled={busy || !status}
                    onChange={(event) => void run(() => onEnabledChange(event.target.checked), "远程连接设置失败，请重试。")}
                  />
                  <span />
                </label>
              </div>
            </div>
            <div className="settings-row">
              <div><strong>连接状态</strong><p>{status?.displayName ?? "正在读取这台 Mac 的状态…"}</p></div>
              <div className="settings-control remote-status-actions">
                <span className={`healthy-state remote-state-${status?.state ?? "starting"}`}><span />{status ? stateLabels[status.state] : "正在读取"}</span>
                <button className="settings-secondary-button" disabled={busy} onClick={() => void run(onRefresh, "刷新远程连接状态失败，请重试。")}>刷新</button>
              </div>
            </div>
            <div className="settings-row">
              <div><strong>实时连接</strong><p>在线设备保持低延迟会话，网络切换后自动续接。</p></div>
              <div className="settings-control"><span className="settings-badge">{status?.activeConnections ?? 0} 台在线</span></div>
            </div>
          </section>
          {error && <div className="settings-inline-notice remote-error" role="alert"><Icon name="archive" /><span>{error}</span></div>}
          {status?.errorCode && <div className="settings-inline-notice remote-error" role="status"><Icon name="archive" /><span>远程服务暂时不可用，请关闭后重新开启或稍后重试。</span></div>}

          <div className="remote-section-toolbar">
            <div><h2>已配对设备</h2><p>撤销后，该设备必须重新扫描二维码才能连接。</p></div>
            <button
              className="settings-primary-button"
              disabled={busy || !pairingSupported || !status?.enabled || status.state !== "ready"}
              onClick={() => setPairingOpen(true)}
            ><Icon name="plus" />连接 iPhone</button>
          </div>
          {!pairingSupported && <p className="remote-capability-note">当前控制面尚未提供设备配对能力。</p>}
          <ul className="remote-device-list" aria-label="已配对设备">
            {(status?.devices ?? []).map((device) => (
              <li key={device.id}>
                <span className={`remote-device-dot${device.online ? " is-online" : ""}`} aria-hidden="true" />
                <div>
                  <strong>{device.displayName}</strong>
                  <p>{device.platform} · {device.online ? "当前在线" : seenLabel(device.lastSeenAt)}</p>
                </div>
                {confirmDeviceId === device.id ? (
                  <div className="remote-revoke-confirm" role="group" aria-label={`确认撤销 ${device.displayName}`}>
                    <button disabled={busy} onClick={() => setConfirmDeviceId(null)}>取消</button>
                    <button
                      className="is-danger"
                      disabled={busy}
                      onClick={() => void run(async () => {
                        await onRevokeDevice(device.id);
                        setConfirmDeviceId(null);
                      }, "撤销设备失败，请重试。")}
                    >确认撤销</button>
                  </div>
                ) : (
                  <button
                    className="settings-secondary-button is-danger"
                    disabled={busy || !deviceManagementSupported}
                    onClick={() => setConfirmDeviceId(device.id)}
                    aria-label={`撤销 ${device.displayName}`}
                  >撤销</button>
                )}
              </li>
            ))}
            {status && status.devices.length === 0 && <li className="remote-empty-device"><span>尚未配对设备</span><p>开启远程连接后，用 PAD iOS 扫描二维码即可。</p></li>}
          </ul>
        </>
      )}
      {pairingOpen && (
        <RemotePairingSheet
          onBegin={onBeginPairing}
          onCancel={onCancelPairing}
          onDismiss={() => setPairingOpen(false)}
        />
      )}
    </>
  );
}
