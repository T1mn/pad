#!/usr/bin/env node
// PAD native workbench host entry: stdin/stdout JSONL protocol for the Swift app.
//
// Ordering matters: sanitize ambient model credentials and NODE_OPTIONS before
// the Pi SDK (or anything that could import it) is loaded. Then install the
// matching public Undici fetch/dispatcher before loading workbench modules.
import { sanitizeAmbientCredentials } from './host-env.mjs';

sanitizeAmbientCredentials(process.env);
// Pin the host SDK to offline mode even though PI_* was just stripped.
process.env.PI_OFFLINE = '1';

const { createFrameDecoder, isRecord, parseJson, writeFrame } = await import('./jsonl.mjs');
const { WorkbenchStore } = await import('./workbench-store.mjs');
const { withDataRootStartup } = await import('./data-root-migration.mjs');
const { initializeHttpClient, reportProxyWarning } = await import('./http-client.mjs');

const COMMAND_TIMEOUT_MS = 120_000;

function diagnostic(message) {
  process.stderr.write(`pad-workbench: ${message}\n`);
}

function emit(frame) {
  try {
    writeFrame(process.stdout, frame);
  } catch (error) {
    diagnostic(`dropped a frame that exceeded the size budget: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function respond(id, command, data) {
  const frame = { type: 'response', command, success: true, data: data ?? {} };
  if (typeof id === 'string') frame.id = id;
  emit(frame);
}

function respondError(id, command, error) {
  const frame = { type: 'response', command, success: false, error: typeof error === 'string' ? error : String(error) };
  if (typeof id === 'string') frame.id = id;
  emit(frame);
}

let root;
let store;
let host;
let httpClient;
try {
  reportProxyWarning(process.env, emit);
  httpClient = await initializeHttpClient({ env: process.env, diagnostic });
  // HTTP globals must be installed before any SDK-capable module is imported.
  const { WorkbenchHost } = await import('./workbench.mjs');
  withDataRootStartup((selected) => {
    root = selected;
    store = new WorkbenchStore({ root });
    store.open();
  }, { env: process.env });
  host = new WorkbenchHost({ root, store, emit, diagnostic });
} catch {
  // No raw module/config errors: they can contain proxy credentials or paths.
  diagnostic('Host startup failed; check Pi installation, proxy configuration and PAD data root');
  store?.releaseLock();
  await httpClient?.close();
  process.exit(1);
}

let exiting = false;
function exitAfterFlush(code) {
  process.exitCode = code;
  const fallback = setTimeout(() => process.exit(code), 1_000);
  fallback.unref?.();
  process.stdout.write('', () => process.exit(code));
}

async function shutdown(code) {
  if (exiting) return;
  exiting = true;
  let timer;
  try {
    // close() synchronously cancels auth/task tokens before awaiting task exits.
    // A stuck child exit must not prevent releasing our dispatcher and lock.
    await Promise.race([
      host.close(),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error('shutdown timeout')), 6_000);
      }),
    ]);
  } catch {
    diagnostic('Host shutdown incomplete');
  } finally {
    clearTimeout(timer);
  }
  store.releaseLock();
  await httpClient.close();
  exitAfterFlush(code);
}

async function dispatch(message) {
  const id = typeof message.id === 'string' ? message.id : undefined;
  const command = typeof message.command === 'string' ? message.command : '';
  if (command === 'shutdown') {
    respond(id, command, {});
    await shutdown(0);
    return;
  }
  if (command.length === 0) {
    respondError(id, command, 'Missing command');
    return;
  }
  let settled = false;
  const timer = setTimeout(() => {
    if (settled) return;
    settled = true;
    respondError(id, command, 'Command timed out');
  }, COMMAND_TIMEOUT_MS);
  timer.unref?.();
  try {
    const data = await host.execute(command, message);
    if (settled) return;
    settled = true;
    clearTimeout(timer);
    respond(id, command, data);
  } catch (error) {
    if (settled) return;
    settled = true;
    clearTimeout(timer);
    respondError(id, command, error instanceof Error ? error.message : String(error));
  }
}

function onFrameText(text) {
  const parsed = parseJson(text);
  if (!parsed.ok) {
    respondError(undefined, 'parse', `Failed to parse command: ${parsed.error.message}`);
    return;
  }
  if (!isRecord(parsed.value)) {
    respondError(undefined, 'parse', 'Failed to parse command: expected a JSON object');
    return;
  }
  void dispatch(parsed.value);
}

const decoder = createFrameDecoder({
  onFrame: onFrameText,
  onOversize: (bytes) => diagnostic(`dropped an oversized request frame (${bytes} bytes)`),
});

process.stdin.on('data', (chunk) => decoder.feed(chunk));
process.stdin.on('end', () => void shutdown(0));
process.stdin.on('error', () => void shutdown(0));
process.stdin.resume();

for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
  process.on(signal, () => void shutdown(0));
}
