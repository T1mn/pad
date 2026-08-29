import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RemoteHostStatus } from "../types";
import { RemoteSettingsSection } from "./RemoteSettingsSection";

const qr = vi.hoisted(() => ({ toDataURL: vi.fn((_payload: string, _options?: unknown) => Promise.resolve("data:image/png;base64,cXI=")) }));
vi.mock("qrcode", () => ({ default: qr }));

const readyStatus: RemoteHostStatus = {
  enabled: true,
  state: "ready",
  displayName: "Tim 的 Mac",
  activeConnections: 1,
  devices: [{
    id: "iphone-1",
    displayName: "Tim 的 iPhone",
    platform: "iOS",
    online: true,
    pairedAt: 1_800_000_000,
    lastSeenAt: 1_800_000_000,
  }],
  updatedAt: 1_800_000_000,
};

afterEach(() => vi.useRealTimers());

describe("RemoteSettingsSection", () => {
  it("后端能力缺失时明确禁用，不伪装为可用", () => {
    renderRemote({ capabilities: [], status: null });
    expect(screen.getByText(/没有远程网关能力/)).toBeInTheDocument();
    expect(screen.queryByLabelText("允许远程连接")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "连接 iPhone" })).not.toBeInTheDocument();
  });

  it("远程开关写回真实控制面，并为设备撤销提供二次确认", async () => {
    const onEnabledChange = vi.fn().mockResolvedValue(undefined);
    const onRevokeDevice = vi.fn().mockResolvedValue(undefined);
    renderRemote({ onEnabledChange, onRevokeDevice });
    const user = userEvent.setup();

    await user.click(screen.getByLabelText("允许远程连接"));
    expect(onEnabledChange).toHaveBeenCalledWith(false);

    await user.click(screen.getByRole("button", { name: "撤销 Tim 的 iPhone" }));
    const confirmation = screen.getByRole("group", { name: "确认撤销 Tim 的 iPhone" });
    expect(confirmation).toBeInTheDocument();
    expect(onRevokeDevice).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "确认撤销" }));
    expect(onRevokeDevice).toHaveBeenCalledWith("iphone-1");
  });

  it("只把原始 payload 交给 QR renderer，Escape 会取消并清除 Sheet", async () => {
    const payload = "pad-remote://pair?ticket=opaque-secret";
    const onCancelPairing = vi.fn().mockResolvedValue(undefined);
    renderRemote({
      onBeginPairing: vi.fn().mockResolvedValue({ pairingId: "pair-1", qrPayload: payload, expiresAt: Date.now() + 30_000 }),
      onCancelPairing,
    });
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "连接 iPhone" }));

    expect(await screen.findByRole("dialog", { name: "连接 iPhone" })).toBeInTheDocument();
    await waitFor(() => expect(qr.toDataURL).toHaveBeenCalled());
    expect(qr.toDataURL.mock.calls.at(-1)?.[0]).toBe(payload);
    expect(screen.getByAltText("用于连接 PAD Desktop 的配对二维码")).toBeInTheDocument();
    expect(document.body.innerHTML).not.toContain(payload);

    await user.keyboard("{Escape}");
    expect(onCancelPairing).toHaveBeenCalledWith("pair-1");
    expect(screen.queryByRole("dialog", { name: "连接 iPhone" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "连接 iPhone" })).toHaveFocus();
  });

  it("倒计时归零后取消服务端 pairing，并移除二维码", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-08-30T00:00:00Z"));
    const onCancelPairing = vi.fn().mockResolvedValue(undefined);
    renderRemote({
      onBeginPairing: vi.fn().mockResolvedValue({ pairingId: "pair-expiring", qrPayload: "opaque", expiresAt: Date.now() + 1_000 }),
      onCancelPairing,
    });
    fireEvent.click(screen.getByRole("button", { name: "连接 iPhone" }));
    await act(async () => { await Promise.resolve(); });
    expect(screen.getByRole("timer")).toHaveTextContent("1 秒后过期");

    act(() => { vi.advanceTimersByTime(1_250); });
    expect(screen.getByText("二维码已过期，请关闭后重新生成。")).toBeInTheDocument();
    expect(screen.queryByAltText("用于连接 PAD Desktop 的配对二维码")).not.toBeInTheDocument();
    expect(onCancelPairing).toHaveBeenCalledWith("pair-expiring");
  });
});

function renderRemote(overrides: Partial<React.ComponentProps<typeof RemoteSettingsSection>> = {}) {
  const props: React.ComponentProps<typeof RemoteSettingsSection> = {
    capabilities: ["remote_gateway_v1", "remote_pairing", "remote_device_management"],
    status: readyStatus,
    onRefresh: vi.fn().mockResolvedValue(undefined),
    onEnabledChange: vi.fn().mockResolvedValue(undefined),
    onBeginPairing: vi.fn().mockResolvedValue({ pairingId: "pair-1", qrPayload: "opaque", expiresAt: Date.now() + 30_000 }),
    onCancelPairing: vi.fn().mockResolvedValue(undefined),
    onRevokeDevice: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  return render(<RemoteSettingsSection {...props} />);
}
