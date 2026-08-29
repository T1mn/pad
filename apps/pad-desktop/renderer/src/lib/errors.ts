export interface UserFacingError {
  message: string;
  diagnostic?: string;
}

const messages: Array<{ pattern: RegExp; message: string }> = [
  {
    pattern: /backend[_ -]?unavailable|host[_ -]?(?:unavailable|stopped|failed)|bridge.*(?:missing|unavailable)|安全桥未加载|econnrefused/i,
    message: "PAD 本地服务暂时不可用，请重新启动应用。",
  },
  {
    pattern: /auth[_ -]?(?:failed|error)|authentication.*(?:failed|missing)|unauthenticated|provider.*auth|invalid[_ -]?credential/i,
    message: "模型账号登录失败，请重新登录后再试。",
  },
  {
    pattern: /request[_ -]?timeout|timed?\s*out|deadline[_ -]?exceeded|etimedout/i,
    message: "请求超时，Pi 可能仍在后台运行，请稍后重试。",
  },
  {
    pattern: /profile[_ -]?mismatch|forbidden|permission[_ -]?denied/i,
    message: "当前账号无权访问这项内容，请切换账号后重试。",
  },
];

const sensitiveDiagnosticAssignment = /["']?(?:(?:PI|PAD|CODEX|CHATGPT)_[A-Z0-9_]+|pi_session_id|session_(?:id|file)|credential(?:_ref)?|(?:access_|refresh_)?token|api_key)["']?\s*[:=]\s*(?:"[^"]*"|'[^']*'|[^\s,;}\]]+)/gi;
const sensitiveDiagnosticKey = /\b(?:(?:PI|PAD|CODEX|CHATGPT)_[A-Z0-9_]+|pi_session_id|session_(?:id|file)|credential(?:_ref)?|(?:access_|refresh_)?token|api_key)\b/gi;
const privateDotDirectory = /(?:\/Users\/[^/\s]+\/|~\/)(?:\.pad|\.pi|\.codex|\.chatgpt)(?:\/[^\s,;)}\]]*)?/gi;
const privateApplicationSupport = /(?:\/Users\/[^/\s]+\/)?Library\/Application Support\/(?:PAD Desktop|Pi|Codex|ChatGPT)(?:\/[^\n,;)}\]]*)?/gi;
const homeDirectoryPrefix = /\/Users\/[^/\s]+\//g;

export function sanitizeDiagnostic(detail: string): string {
  return detail
    .replace(sensitiveDiagnosticAssignment, "<敏感诊断字段已隐藏>")
    .replace(sensitiveDiagnosticKey, "<敏感诊断字段已隐藏>")
    .replace(privateDotDirectory, "<应用私有路径已隐藏>")
    .replace(privateApplicationSupport, "<应用私有路径已隐藏>")
    .replace(homeDirectoryPrefix, "~/")
    .slice(0, 2_000);
}

export function toUserFacingError(error: unknown, fallback = "操作未完成，请重试。"): UserFacingError {
  const code = errorCode(error);
  const detail = errorDetail(error);
  const searchable = `${code} ${detail}`;
  const matched = messages.find((entry) => entry.pattern.test(searchable));
  const safeDetail = sanitizeDiagnostic(detail);
  return {
    message: matched?.message ?? fallback,
    diagnostic: safeDetail && safeDetail !== matched?.message ? safeDetail : undefined,
  };
}

function errorCode(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    const value = (error as { code?: unknown }).code;
    if (typeof value === "string") return value;
  }
  return "";
}

function errorDetail(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error !== null && "message" in error) {
    const value = (error as { message?: unknown }).message;
    if (typeof value === "string") return value;
  }
  return typeof error === "string" ? error : "";
}
