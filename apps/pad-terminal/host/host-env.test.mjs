import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { PI_PACKAGE_NAME, resolveDataRoot, resolvePiPackage, sanitizeAmbientCredentials } from './host-env.mjs';
import { launchSpec } from '../scripts/pi-session.mjs';
import { withDataRootStartup } from './data-root-migration.mjs';

test('isolates ambient credentials and resolves the Preview data root and Pi package', () => {
  const env = {
    HOME: '/Users/example',
    PATH: '/usr/bin:/bin',
    PAD_TERMINAL_DATA_ROOT: '/tmp/pad-root',
    PAD_PI_PACKAGE: '/tmp/fake-pi',
    NODE_OPTIONS: '--require=unwanted',
    NODE_PATH: '/tmp/inject',
    OPENAI_API_KEY: 'must-not-survive',
    ANTHROPIC_API_KEY: 'must-not-survive',
    AWS_SECRET_ACCESS_KEY: 'must-not-survive',
    PI_CODING_AGENT_DIR: '/production/agent',
    CODEX_HOME: '/codex',
    GOOGLE_APPLICATION_CREDENTIALS: '/gac.json',
    MY_SERVICE_TOKEN: 'must-not-survive',
  };
  const removed = sanitizeAmbientCredentials(env);
  for (const key of [
    'NODE_OPTIONS', 'NODE_PATH', 'OPENAI_API_KEY', 'ANTHROPIC_API_KEY', 'AWS_SECRET_ACCESS_KEY',
    'PI_CODING_AGENT_DIR', 'CODEX_HOME', 'GOOGLE_APPLICATION_CREDENTIALS', 'MY_SERVICE_TOKEN',
  ]) {
    assert.equal(env[key], undefined, `${key} should be removed`);
  }
  assert.ok(removed.includes('OPENAI_API_KEY'));
  assert.equal(env.PAD_PI_PACKAGE, '/tmp/fake-pi');
  assert.equal(env.PAD_TERMINAL_DATA_ROOT, '/tmp/pad-root');
  assert.equal(env.PATH, '/usr/bin:/bin');
  assert.equal(resolveDataRoot(env), path.resolve('/tmp/pad-root'));
  assert.equal(
    resolveDataRoot({}, '/Users/example'),
    path.join('/Users/example', '.pad', 'preview'),
  );

  const temporary = mkdtempSync(path.join(os.tmpdir(), 'pad-host-env-'));
  try {
    const packageRoot = path.join(temporary, 'pi');
    mkdirSync(packageRoot);
    writeFileSync(path.join(packageRoot, 'package.json'), JSON.stringify({ name: PI_PACKAGE_NAME, version: '1.0.2', bin: 'cli.mjs' }));
    writeFileSync(path.join(packageRoot, 'cli.mjs'), '// offline fixture');
    const found = resolvePiPackage({ PAD_PI_PACKAGE: packageRoot });
    assert.equal(found.packageRoot, path.resolve(packageRoot));
    assert.equal(found.version, '1.0.2');
    const home = path.join(temporary, 'home');
    const source = { HOME: home, PAD_PI_PACKAGE: packageRoot };
    assert.equal(resolveDataRoot(source), path.join(home, '.pad', 'preview'));
    assert.equal(withDataRootStartup((root) => root, { env: source }), path.join(home, '.pad', 'preview'));
    assert.equal(launchSpec({ source }).agent, path.join(home, '.pad', 'preview', 'v1/profiles/default/pi-agent'));
    source.PAD_TERMINAL_DATA_ROOT = path.join(temporary, 'override');
    assert.equal(launchSpec({ source }).sessions, path.join(source.PAD_TERMINAL_DATA_ROOT, 'v1/profiles/default/pi-sessions'));
    assert.equal(launchSpec({ source, dataRoot: path.join(temporary, 'argument') }).sessions,
      path.join(temporary, 'argument', 'v1/profiles/default/pi-sessions'));
    assert.throws(() => resolvePiPackage({ PAD_PI_PACKAGE: path.join(temporary, 'missing') }), /not installed/);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});
