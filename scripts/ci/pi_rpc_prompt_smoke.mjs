#!/usr/bin/env node
// Run one real provider-backed Pi RPC prompt.  This is opt-in because it
// consumes provider quota and requires credentials configured for Pi.
import { mkdtempSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { StringDecoder } from "node:string_decoder";

const piBin = process.env.PAD_PI_BIN || "pi";
const prompt = process.env.PAD_PI_PROMPT || "Reply with exactly PAD_PI_SMOKE_OK and do not use tools.";
const timeoutMs = Number(process.env.PAD_PI_PROMPT_TIMEOUT_MS || 60_000);
const provider = process.env.PAD_PI_PROVIDER;
const model = process.env.PAD_PI_MODEL;
const root = mkdtempSync(join(tmpdir(), "pad-pi-prompt-smoke-"));
const agentRoot = join(root, "agent");
const sessionRoot = join(root, "sessions");
mkdirSync(agentRoot);
mkdirSync(sessionRoot);

const args = [
  "--mode", "rpc",
  "--session-dir", sessionRoot,
  "--no-approve",
  "--no-context-files",
  "--no-extensions",
  "--no-skills",
  "--no-prompt-templates",
  "--no-themes",
];
if (provider) args.push("--provider", provider);
if (model) args.push("--model", model);

const child = spawn(piBin, args, {
  cwd: process.cwd(),
  env: {
    ...process.env,
    PI_CODING_AGENT_DIR: agentRoot,
    PI_CODING_AGENT_SESSION_DIR: sessionRoot,
  },
  stdio: ["pipe", "pipe", "pipe"],
});

const decoder = new StringDecoder("utf8");
let stdoutBuffer = "";
let stderr = "";
let promptAccepted = false;
let finished = false;
let timeout;

function cleanup() {
  clearTimeout(timeout);
  if (!child.killed) child.kill("SIGTERM");
  try {
    rmSync(root, { recursive: true, force: true });
  } catch {
    // The temporary directory is only a diagnostic fixture; cleanup failure
    // must not mask the provider result.
  }
}

function finish(code, message) {
  if (finished) return;
  finished = true;
  cleanup();
  if (code === 0) {
    console.log(`pi_rpc_prompt_smoke: ${message}`);
  } else {
    console.error(`pi_rpc_prompt_smoke: ${message}`);
    if (stderr.trim()) console.error(stderr.trim().slice(0, 4000));
  }
  process.exitCode = code;
}

function handleLine(line) {
  if (!line.trim()) return;
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    finish(1, `Pi emitted non-JSON stdout: ${line.slice(0, 500)}`);
    return;
  }
  if (message.type === "response" && message.command === "prompt") {
    if (message.success !== true) {
      finish(3, `prompt rejected: ${message.error || "unknown Pi error"}`);
      return;
    }
    promptAccepted = true;
  }
  if (message.type === "agent_settled") {
    finish(promptAccepted ? 0 : 1, promptAccepted ? "real prompt settled" : "agent settled before prompt acceptance");
  }
}

child.on("error", (error) => finish(1, `could not start Pi: ${error.message}`));
child.stderr.on("data", (chunk) => {
  stderr += chunk.toString();
});
child.stdout.on("data", (chunk) => {
  stdoutBuffer += decoder.write(chunk);
  let newline;
  while ((newline = stdoutBuffer.indexOf("\n")) !== -1) {
    const line = stdoutBuffer.slice(0, newline);
    stdoutBuffer = stdoutBuffer.slice(newline + 1);
    handleLine(line);
    if (finished) return;
  }
});
child.on("close", (code, signal) => {
  if (!finished) finish(1, `Pi exited before agent_settled (code=${code}, signal=${signal || "none"})`);
});

timeout = setTimeout(() => finish(2, `timed out after ${timeoutMs}ms`), timeoutMs);
child.stdin.write(`${JSON.stringify({ id: "prompt-1", type: "prompt", message: prompt })}\n`);
child.stdin.flush?.();

