// Environment isolation for the native workbench host.
//
// This module must not import the Pi SDK (or anything that does) so that the
// entry point can sanitize `process.env` before the SDK is dynamically loaded.
import { existsSync, readFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export const PI_PACKAGE_NAME = '@earendil-works/pi-coding-agent';

// Ambient model credentials and process injection that must never reach the
// hosted SDK. Matched against the host's own inherited environment only.
const REMOVE_EXACT = new Set(['NODE_OPTIONS', 'NODE_PATH']);
const REMOVE_PREFIX = [
  /^PI_/,
  /^AWS_/,
  /^AZURE_/,
  /^GOOGLE_/,
  /^GEMINI_/,
  /^VERTEX_/,
  /^ANTHROPIC_/,
  /^OPENAI_/,
  /^GROQ_/,
  /^XAI_/,
  /^MISTRAL_/,
  /^COHERE_/,
  /^DEEPSEEK_/,
  /^OPENROUTER_/,
  /^PERPLEXITY_/,
  /^CLAUDE_/,
  /^CODEX_/,
  /^CHATGPT_/,
  /^CLOUDFLARE_/,
];
const REMOVE_SUFFIX = /(_API_KEY|_APIKEY|_ACCESS_KEY|_ACCESS_KEY_ID|_SECRET_ACCESS_KEY|_SECRET|_TOKEN|_PASSWORD|_PASSWD|_CREDENTIALS|_CREDENTIAL)$/;

function shouldRemove(key) {
  if (REMOVE_EXACT.has(key)) return true;
  if (REMOVE_SUFFIX.test(key)) return true;
  return REMOVE_PREFIX.some((pattern) => pattern.test(key));
}

/**
 * Delete ambient provider credentials and Node injection from `env` in place.
 * `PAD_*` configuration passed by the app is intentionally preserved.
 * Returns the removed key names (never their values).
 */
export function sanitizeAmbientCredentials(env) {
  const removed = [];
  for (const key of Object.keys(env)) {
    if (!shouldRemove(key)) continue;
    delete env[key];
    removed.push(key);
  }
  return removed;
}

/** Explicit override, otherwise ~/.pad/preview (including a supplied HOME). */
export function resolveDataRoot(env = process.env, home = env?.HOME || os.homedir()) {
  const explicit = env?.PAD_TERMINAL_DATA_ROOT;
  if (typeof explicit === 'string' && explicit.trim().length > 0) return path.resolve(explicit.trim());
  return path.resolve(home, '.pad', 'preview');
}

export function resolveLegacyDataRoot(env = process.env, home = env?.HOME || os.homedir()) {
  return path.resolve(home, 'Library', 'Application Support', 'PAD Terminal Preview');
}

/**
 * Discover an already-installed Pi package. Never installs or upgrades.
 * Candidates mirror `../scripts/pi-session.mjs` so host and children agree.
 */
export function resolvePiPackage(env = process.env) {
  const candidates = typeof env?.PAD_PI_PACKAGE === 'string' && env.PAD_PI_PACKAGE.length > 0
    ? [env.PAD_PI_PACKAGE]
    : [
      `/opt/homebrew/lib/node_modules/${PI_PACKAGE_NAME}`,
      `/usr/local/lib/node_modules/${PI_PACKAGE_NAME}`,
    ];
  const packageRoot = candidates.find((candidate) => existsSync(path.join(candidate, 'package.json')));
  if (!packageRoot) throw new Error('Pi is not installed; set PAD_PI_PACKAGE to its package directory');
  let metadata;
  try {
    metadata = JSON.parse(readFileSync(path.join(packageRoot, 'package.json'), 'utf8'));
  } catch {
    throw new Error('Pi package metadata is unreadable (package.json)');
  }
  if (metadata?.name !== PI_PACKAGE_NAME) throw new Error('PAD_PI_PACKAGE must point to the Pi coding-agent package');
  return { packageRoot: path.resolve(packageRoot), version: metadata.version };
}
