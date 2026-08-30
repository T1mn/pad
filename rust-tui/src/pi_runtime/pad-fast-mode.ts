import { readFileSync, writeFileSync } from "node:fs";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const MODE_FILE = process.env.PAD_PI_FAST_MODE_FILE;

function readFastMode(): boolean {
	if (!MODE_FILE) return true;
	try {
		return readFileSync(MODE_FILE, "utf8").trim().toLowerCase() !== "off";
	} catch {
		return true;
	}
}

function writeFastMode(enabled: boolean): void {
	if (!MODE_FILE) return;
	writeFileSync(MODE_FILE, enabled ? "on\n" : "off\n", { encoding: "utf8", mode: 0o600 });
}

function asPayloadRecord(value: unknown): Record<string, unknown> | null {
	return value !== null && typeof value === "object" && !Array.isArray(value)
		? value as Record<string, unknown>
		: null;
}

/** PAD-owned Pi extension: keep Fast mode native to Pi's provider request. */
export default function padFastMode(pi: ExtensionAPI) {
	pi.on("before_provider_request", (event, ctx) => {
		if (!readFastMode() || ctx.model?.provider !== "openai-codex") return;
		const payload = asPayloadRecord(event.payload);
		if (!payload) return;
		return { ...payload, service_tier: "fast" };
	});

	pi.registerCommand("fast", {
		description: "切换 OpenAI Codex 快速模式（on/off/status）",
		handler: async (args, ctx) => {
			const command = args.trim().toLowerCase();
			if (command === "on") writeFastMode(true);
			else if (command === "off") writeFastMode(false);
			else if (command && command !== "status") {
				ctx.ui.notify("用法：/fast on、/fast off 或 /fast status", "warning");
				return;
			}
			ctx.ui.notify(`快速模式已${readFastMode() ? "开启" : "关闭"}`, "info");
		},
	});
}
