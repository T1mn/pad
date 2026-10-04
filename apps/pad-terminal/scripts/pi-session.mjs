#!/usr/bin/env node
// Development bridge: native Ghostty PTY -> Node -> Pi CLI, without Electron.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { resolveDataRoot } from '../host/host-env.mjs';
import { withStandaloneBridgeStartup } from '../host/data-root-migration.mjs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const packageName = '@earendil-works/pi-coding-agent';
const environmentKeys = [
  'HOME', 'USER', 'LOGNAME', 'SHELL', 'PATH', 'TMPDIR', 'LANG', 'LC_ALL', 'LC_CTYPE',
  'TERM', 'TERM_PROGRAM', 'TERM_PROGRAM_VERSION', 'COLORTERM', 'TERMINFO',
  'GHOSTTY_RESOURCES_DIR', 'GHOSTTY_BIN_DIR', 'XDG_RUNTIME_DIR',
  'HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'NO_PROXY',
  'http_proxy', 'https_proxy', 'all_proxy', 'no_proxy',
];

export function launchSpec({ profile = 'default', cwd = process.cwd(), source = process.env, dataRoot } = {}) {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(profile)) throw new Error('Invalid PAD profile name');
  const root = dataRoot === undefined ? resolveDataRoot(source) : path.resolve(dataRoot);
  const profileRoot = path.join(root, 'v1', 'profiles', profile);
  const agent = path.join(profileRoot, 'pi-agent');
  const sessions = path.join(profileRoot, 'pi-sessions');
  const candidates = source.PAD_PI_PACKAGE ? [source.PAD_PI_PACKAGE] : [
    `/opt/homebrew/lib/node_modules/${packageName}`,
    `/usr/local/lib/node_modules/${packageName}`,
  ];
  const packageRoot = candidates.find((candidate) => existsSync(path.join(candidate, 'package.json')));
  if (!packageRoot) throw new Error('Pi is not installed; set PAD_PI_PACKAGE to its package directory');
  let metadata;
  try { metadata = JSON.parse(readFileSync(path.join(packageRoot, 'package.json'), 'utf8')); }
  catch { throw new Error('Pi package metadata is unreadable (package.json)'); }
  if (metadata.name !== packageName) throw new Error('PAD_PI_PACKAGE must point to the Pi coding-agent package');
  const bin = typeof metadata.bin === 'string' ? metadata.bin : metadata.bin?.pi;
  if (typeof bin !== 'string') throw new Error('Pi package has no CLI entry point');
  const cli = path.resolve(packageRoot, bin);
  if (!existsSync(cli)) throw new Error('Pi CLI entry point is unavailable');
  for (const directory of [agent, sessions]) mkdirSync(directory, { recursive: true, mode: 0o700 });
  return {
    cwd: path.resolve(cwd), cli, version: metadata.version, agent, sessions,
    args: ['--session-dir', sessions, '--name', 'PAD Terminal Preview', '--offline', '--no-approve',
      '--no-extensions', '--no-skills', '--no-prompt-templates', '--no-themes'],
    env: {
      ...Object.fromEntries(environmentKeys.filter((key) => source[key] !== undefined).map((key) => [key, source[key]])),
      PATH: source.PATH ?? '/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin',
      PI_CODING_AGENT_DIR: agent,
      PI_CODING_AGENT_SESSION_DIR: sessions,
      PI_SKIP_VERSION_CHECK: '1',
      PI_TELEMETRY: '0',
      PI_OFFLINE: '1',
    },
  };
}

function main() {
  const args = process.argv.slice(2);
  let profile = 'default';
  if (args[0] === '--profile') {
    args.shift();
    profile = args.shift() ?? '';
  }
  if (args[0] === '--') args.shift();
  // Inspect and exclude migration BEFORE launchSpec creates profile directories.
  // Helpers used by the host/fixtures remain lease-free and host-owned.
  const { result: spec, release } = withStandaloneBridgeStartup((root) => launchSpec({ profile, dataRoot: root }));
  let child;
  try {
    child = spawn(process.execPath, [spec.cli, ...spec.args, ...args], {
      cwd: spec.cwd, env: spec.env, stdio: 'inherit',
    });
  } catch (error) { release(); throw error; }
  child.once('close', release);
  for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.once(signal, () => child.kill(signal));
  child.once('error', (error) => { console.error(error.message); process.exitCode = 1; });
  child.once('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
