import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { launchSpec } from './pi-session.mjs';

test('isolates profile data and does not inherit credentials or Node injection', () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), 'pad-terminal-spec-'));
  try {
    const packageRoot = path.join(temporary, 'fake-pi');
    mkdirSync(packageRoot);
    writeFileSync(path.join(packageRoot, 'package.json'), JSON.stringify({
      name: '@earendil-works/pi-coding-agent', version: '1.0.2', bin: { pi: 'cli.js' },
    }));
    writeFileSync(path.join(packageRoot, 'cli.js'), '// Never executed by this test.\n');
    const spec = launchSpec({
      profile: 'demo', dataRoot: temporary, cwd: temporary,
      source: {
        PAD_PI_PACKAGE: packageRoot, HOME: temporary, PATH: '/usr/bin:/bin', TERM: 'xterm-ghostty',
        OPENAI_API_KEY: 'must-not-inherit', ANTHROPIC_API_KEY: 'must-not-inherit',
        PI_SESSION_ID: 'parent-session', PI_CODING_AGENT_DIR: '/production',
        NODE_OPTIONS: '--require=unwanted', PAD_COLLABORATION_TOKEN: 'must-not-inherit',
      },
    });
    assert.equal(spec.env.TERM, 'xterm-ghostty');
    assert.equal(spec.env.PI_CODING_AGENT_DIR, path.join(temporary, 'v1/profiles/demo/pi-agent'));
    assert.equal(spec.args[1], spec.sessions);
    for (const key of ['OPENAI_API_KEY', 'ANTHROPIC_API_KEY', 'PI_SESSION_ID', 'NODE_OPTIONS', 'PAD_COLLABORATION_TOKEN']) {
      assert.equal(spec.env[key], undefined);
    }
    assert.equal(spec.env.PI_OFFLINE, '1');
    assert.ok(spec.args.includes('--no-approve'));
    assert.ok(spec.args.includes('--no-extensions'));
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});

test('rejects profile path traversal before accessing storage', () => {
  assert.throws(() => launchSpec({ profile: '../production' }), /Invalid PAD profile/);
});
