import { describe, expect, it } from "vitest";
import { sanitizeDiagnostic, toUserFacingError } from "./errors";

describe("toUserFacingError", () => {
  it.each([
    ["backend_unavailable", "PAD 本地服务暂时不可用，请重新启动应用。"],
    ["auth_failed", "模型账号登录失败，请重新登录后再试。"],
    ["request_timeout", "请求超时，Pi 可能仍在后台运行，请稍后重试。"],
  ])("将 %s 映射为中文提示", (code, expected) => {
    expect(toUserFacingError({ code, message: `raw english: ${code}` }).message).toBe(expected);
  });

  it("原始英文只保留为诊断信息", () => {
    const result = toUserFacingError(new Error("ETIMEDOUT while polling Pi"));
    expect(result.message).not.toContain("ETIMEDOUT");
    expect(result.diagnostic).toContain("ETIMEDOUT");
  });

  it("诊断信息隐藏内部环境变量、凭证、会话标识与私有路径", () => {
    const raw = [
      "PI_CODING_AGENT_DIR=/Users/tim/.pad/agent",
      'session_file="/Users/tim/.codex/sessions/private.jsonl"',
      "session_id=pi-secret-123",
      "credential_ref=keychain-secret",
      "access_token=token-secret",
      "api_key=sk-private",
      "at /Users/tim/Library/Application Support/ChatGPT/session/private.json",
      "workspace /Users/tim/work/project",
    ].join("\n");

    const safe = sanitizeDiagnostic(raw);
    expect(safe).not.toContain("PI_CODING_AGENT_DIR");
    expect(safe).not.toContain("pi-secret-123");
    expect(safe).not.toContain("keychain-secret");
    expect(safe).not.toContain("token-secret");
    expect(safe).not.toContain("sk-private");
    expect(safe).not.toContain("/Users/tim");
    expect(safe).not.toContain(".pad");
    expect(safe).not.toContain(".codex");
    expect(safe).toContain("workspace ~/work/project");
  });
});
