// Deliberately narrow: only explicit installed OpenAI OAuth attempts are logged.
// Never accept provider names, context, Error objects or free-form stage text.
import fs from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';

const providers = new Set(['openai', 'openai-codex']);
const phases = new Set(['unknown', 'setup', 'authorization', 'authorization_notice_observed', 'manual_prompt_observed', 'token_exchange_observed', 'callback_validation', 'callback_server', 'token_exchange', 'credential_sync', 'cleanup', 'sdk_login_cleanup']);
const categories = new Set(['attempt', 'stage', 'unknown', 'identity_missing', 'identity_persistence', 'credential_sync', 'unsupported', 'sdk_load', 'runtime_create', 'callback_port_busy', 'callback_server_failed', 'callback_invalid', 'authorization_rejected', 'token_http_rejected', 'token_oauth_invalid_request', 'token_oauth_invalid_client', 'token_oauth_invalid_grant', 'token_oauth_unauthorized_client', 'token_oauth_unsupported_grant_type', 'token_oauth_invalid_scope', 'token_json_unrecognized', 'token_html_response', 'token_body_unrecognized', 'token_response_invalid', 'transport_error', 'local_socket_error', 'cleanup_failed', 'sdk_login_cleanup_success', 'cancelled']);
const errnos = new Set(['ENOTFOUND', 'EAI_AGAIN', 'ECONNREFUSED', 'ECONNRESET', 'ETIMEDOUT', 'UND_ERR_CONNECT_TIMEOUT', 'CERT_HAS_EXPIRED', 'UNABLE_TO_VERIFY_LEAF_SIGNATURE', 'DEPTH_ZERO_SELF_SIGNED_CERT', 'EADDRINUSE', 'EACCES', 'EADDRNOTAVAIL']);
const outcomes = new Set(['started', 'observed', 'failed', 'cancelled', 'sdk_login_cleanup_success']);
export const AUTH_DIAGNOSTIC_MAX_BYTES = 64 * 1024;
export const AUTH_DIAGNOSTIC_MAX_RECORDS = 100;

function recordOf(event, timestamp = new Date().toISOString()) {
  if (!providers.has(event?.providerId) || event.method !== 'oauth' ||
      !phases.has(event.phase) || !categories.has(event.category) || !outcomes.has(event.outcome)) return null;
  const record = { timestamp, providerId: event.providerId, method: 'oauth', phase: event.phase, category: event.category, outcome: event.outcome };
  if (Number.isInteger(event.httpStatus) && event.httpStatus >= 100 && event.httpStatus <= 599) record.httpStatus = event.httpStatus;
  if (errnos.has(event.errno)) record.errno = event.errno;
  return record;
}

async function privateDirectory(directory) {
  // Reject symlinks in every component, including the explicitly supplied root.
  const absolute = path.resolve(directory);
  let current = path.parse(absolute).root;
  for (const part of absolute.slice(current.length).split(path.sep).filter(Boolean)) {
    current = path.join(current, part);
    try { await fs.mkdir(current, { mode: 0o700 }); } catch (error) { if (error.code !== 'EEXIST') throw error; }
    const stat = await fs.lstat(current);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('Unsafe diagnostic directory');
  }
  await fs.chmod(absolute, 0o700);
}

export function createAuthDiagnostics({ dataRoot }) {
  const root = path.resolve(dataRoot);
  const directory = path.join(root, 'v1');
  const file = path.join(directory, 'auth-diagnostics.jsonl');
  let queue = Promise.resolve();
  let pending = 0;
  return (event) => {
    const record = recordOf(event);
    if (!record || pending >= AUTH_DIAGNOSTIC_MAX_RECORDS) return Promise.resolve(false);
    pending++;
    const write = queue.then(async () => {
      let handle;
      try {
        await privateDirectory(root);
        await privateDirectory(directory);
        // NONBLOCK prevents a malicious FIFO from hanging open; NOFOLLOW refuses links.
        handle = await fs.open(file, constants.O_RDWR | constants.O_CREAT | constants.O_NOFOLLOW | constants.O_NONBLOCK, 0o600);
        const stat = await handle.stat();
        if (!stat.isFile() || stat.nlink !== 1) return false;
        await handle.chmod(0o600);
        const records = [];
        if (stat.size <= AUTH_DIAGNOSTIC_MAX_BYTES) {
          const buffer = Buffer.alloc(stat.size);
          await handle.read(buffer, 0, buffer.length, 0);
          for (const line of buffer.toString('utf8').split('\n')) {
            try {
              const parsed = JSON.parse(line);
              if (typeof parsed.timestamp !== 'string' || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(parsed.timestamp)) continue;
              const safe = recordOf(parsed, parsed.timestamp);
              if (safe) records.push(JSON.stringify(safe));
            } catch { /* discard malformed/untrusted records */ }
          }
        }
        records.push(JSON.stringify(record));
        while (records.length > AUTH_DIAGNOSTIC_MAX_RECORDS || Buffer.byteLength(records.join('\n') + '\n') > AUTH_DIAGNOSTIC_MAX_BYTES) records.shift();
        const content = records.join('\n') + '\n';
        await handle.writeFile(content, 'utf8');
        await handle.truncate(Buffer.byteLength(content));
        return true;
      } catch { return false; } finally { await handle?.close().catch(() => {}); }
    });
    queue = write.catch(() => false).finally(() => { pending--; });
    return queue;
  };
}
