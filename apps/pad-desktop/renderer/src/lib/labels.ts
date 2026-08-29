import type { PermissionMode, TaskStatus, TurnEntry } from "../types";

export function permissionModeLabel(mode: PermissionMode | null): string {
  if (mode === "guarded") return "受保护";
  if (mode === "workspace_full") return "工作区完全访问";
  if (mode === "system_full") return "系统完全访问";
  return "继承默认值";
}

export function taskStatusLabel(status: TaskStatus | string): string {
  if (status === "idle") return "空闲";
  if (status === "running") return "运行中";
  if (status === "attention") return "需要处理";
  if (status === "complete" || status === "completed") return "已完成";
  if (status === "failed") return "失败";
  return "状态未知";
}

export function backendStatusLabel(status: string): string {
  if (status === "ready") return "已连接";
  if (status === "starting") return "连接中";
  if (status === "stopped") return "已停止";
  if (status === "failed") return "连接失败";
  if (status === "unavailable") return "不可用";
  return "状态未知";
}

export function toolStateLabel(state: TurnEntry["state"]): string {
  if (state === "running") return "运行中";
  if (state === "failed") return "失败";
  if (state === "complete") return "完成";
  return "状态未知";
}

export function localizePiAuthPrompt(value: string | null | undefined, hasOptions = false): string | undefined {
  const text = value?.trim();
  if (!text || /[\u3400-\u9fff]/u.test(text)) return text || undefined;
  if (/(?:select|choose).*(?:login|sign[ -]?in|auth).*(?:method|option|account)/i.test(text)
    || /how (?:would|do) you .*?(?:login|sign[ -]?in|authenticate)/i.test(text)) {
    return "请选择登录方式。";
  }
  if (/(?:enter|paste|provide).*(?:api[ _-]?key)/i.test(text)) return "请输入 API 密钥。";
  if (/(?:enter|paste|provide).*(?:verification|device|authori[sz]ation|auth).*code/i.test(text)) return "请输入验证码。";
  if (/(?:open|continue).*(?:browser|web)/i.test(text)) return "请在浏览器中完成授权，然后返回 PAD。";
  if (/press (?:the )?enter.*continue/i.test(text)) return "按回车键继续。";
  return hasOptions ? "请选择一个选项。" : "请完成 Pi 提供的登录验证步骤。";
}

export function localizePiAuthOption(value: string | null | undefined): string | undefined {
  const text = value?.trim();
  if (!text || /[\u3400-\u9fff]/u.test(text)) return text || undefined;
  if (/(?:browser|web).*(?:login|sign[ -]?in)/i.test(text)) {
    return /default|recommended/i.test(text) ? "浏览器登录（默认）" : "浏览器登录";
  }
  if (/device.*code.*(?:login|sign[ -]?in|auth)/i.test(text)) {
    return /headless|remote|terminal/i.test(text) ? "设备码登录（无界面环境）" : "设备码登录";
  }
  if (/(?:login|sign[ -]?in).*(?:chatgpt)/i.test(text)) return "使用 ChatGPT 登录";
  if (/api[ _-]?key/i.test(text)) return /login|sign[ -]?in|auth/i.test(text) ? "使用 API 密钥登录" : "API 密钥";
  if (/recommended.*(?:most|user)/i.test(text)) return "推荐大多数用户使用";
  if (/managed by.*organi[sz]ation/i.test(text)) return "由组织管理";
  if (/(?:remote|headless|terminal).*(?:environment|machine|server)/i.test(text)) return "适用于远程或无界面环境";
  if (/(?:open|continue).*(?:browser|web)/i.test(text)) return "在浏览器中完成授权";
  return text;
}
