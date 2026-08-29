import { useEffect, useRef, useState } from "react";
import QRCode from "qrcode";
import type { RemotePairing } from "../types";
import { Icon } from "./Icons";
import { ModalSheet } from "./ModalSheet";

function epochMilliseconds(value: number): number {
  return value < 1_000_000_000_000 ? value * 1_000 : value;
}

export function RemotePairingSheet({
  onBegin,
  onCancel,
  onDismiss,
}: {
  onBegin(): Promise<RemotePairing>;
  onCancel(pairingId: string): Promise<void>;
  onDismiss(): void;
}) {
  const [pairing, setPairing] = useState<RemotePairing | null>(null);
  const [qrImage, setQrImage] = useState<string | null>(null);
  const [remainingSeconds, setRemainingSeconds] = useState<number | null>(null);
  const [expired, setExpired] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pairingIdRef = useRef<string | null>(null);
  const cancelledRef = useRef(false);
  const beginRef = useRef(onBegin);
  const cancelRef = useRef(onCancel);
  beginRef.current = onBegin;
  cancelRef.current = onCancel;

  function clearEphemeralState() {
    pairingIdRef.current = null;
    setPairing(null);
    setQrImage(null);
    setRemainingSeconds(null);
  }

  async function cancelOnce() {
    const pairingId = pairingIdRef.current;
    if (!pairingId || cancelledRef.current) return;
    cancelledRef.current = true;
    pairingIdRef.current = null;
    try {
      await cancelRef.current(pairingId);
    } catch {
      // Closing or expiry still destroys the local secret even if the host is gone.
    }
  }

  useEffect(() => {
    let alive = true;
    void beginRef.current().then((next) => {
      if (!alive) {
        void cancelRef.current(next.pairingId).catch(() => undefined);
        return;
      }
      pairingIdRef.current = next.pairingId;
      setPairing(next);
    }).catch(() => {
      if (alive) setError("无法生成配对二维码，请确认远程连接已开启后重试。");
    });
    return () => {
      alive = false;
      const pairingId = pairingIdRef.current;
      pairingIdRef.current = null;
      setQrImage(null);
      if (pairingId && !cancelledRef.current) void cancelRef.current(pairingId).catch(() => undefined);
    };
  }, []);

  useEffect(() => {
    if (!pairing) return undefined;
    let alive = true;
    // Render the opaque host payload exactly as returned; never reconstruct the URI.
    void QRCode.toDataURL(pairing.qrPayload, {
      errorCorrectionLevel: "H",
      margin: 1,
      width: 240,
      color: { dark: "#111111", light: "#ffffff" },
    }).then((image) => {
      if (alive) setQrImage(image);
    }).catch(() => {
      if (alive) setError("二维码生成失败，请关闭后重新配对。");
    });
    return () => { alive = false; };
  }, [pairing]);

  useEffect(() => {
    if (!pairing) return undefined;
    const update = () => {
      const seconds = Math.max(0, Math.ceil((epochMilliseconds(pairing.expiresAt) - Date.now()) / 1_000));
      setRemainingSeconds(seconds);
      if (seconds === 0) {
        setExpired(true);
        void cancelOnce();
        clearEphemeralState();
      }
    };
    update();
    const timer = window.setInterval(update, 250);
    return () => window.clearInterval(timer);
  }, [pairing]);

  function dismiss() {
    void cancelOnce();
    clearEphemeralState();
    onDismiss();
  }

  return (
    <ModalSheet
      labelledBy="remote-pairing-title"
      describedBy="remote-pairing-description"
      className="remote-pairing-sheet"
      onDismiss={dismiss}
    >
      <div className="auth-sheet-icon"><Icon name="layout" /></div>
      <h2 id="remote-pairing-title">连接 iPhone</h2>
      <p id="remote-pairing-description">在 PAD iOS 中扫描二维码。二维码只在本窗口内短暂保留。</p>
      <div className="remote-qr-frame" aria-live="polite">
        {qrImage && !expired ? (
          <img src={qrImage} alt="用于连接 PAD Desktop 的配对二维码" />
        ) : error ? (
          <span role="alert">{error}</span>
        ) : expired ? (
          <span role="status">二维码已过期，请关闭后重新生成。</span>
        ) : (
          <span role="status">正在安全生成二维码…</span>
        )}
      </div>
      {remainingSeconds !== null && !expired && (
        <p className="remote-pairing-countdown" role="timer" aria-live="polite">
          {remainingSeconds} 秒后过期
        </p>
      )}
      <div className="auth-sheet-actions">
        <button className="is-primary" onClick={dismiss}>关闭</button>
      </div>
    </ModalSheet>
  );
}
