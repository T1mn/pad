import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import type { ForgeConfig } from '@electron-forge/shared-types';
import { MakerZIP } from '@electron-forge/maker-zip';
import { FusesPlugin } from '@electron-forge/plugin-fuses';
import { VitePlugin } from '@electron-forge/plugin-vite';
import { FuseV1Options, FuseVersion } from '@electron/fuses';

const resourceRoot = process.env.PAD_ELECTRON_RESOURCE_DIR;
const electronZipDir = process.env.PAD_ELECTRON_ZIP_DIR;
const signingIdentity = process.env.PAD_DESKTOP_SIGN_IDENTITY?.trim() || '-';
const isDeveloperIdSigning = signingIdentity !== '-';
const releaseCommands = new Set(['package', 'make', 'publish']);
const isReleaseCommand = process.argv.some((argument) => releaseCommands.has(argument));
const AD_HOC_JIT_ENTITLEMENTS = [
  'com.apple.security.cs.allow-jit',
  // Hardened-runtime library validation cannot establish a shared Team ID
  // for ad-hoc signatures. Electron's main and helper processes all link the
  // separately signed Electron Framework, so each executable needs this one
  // local-build exception.
  'com.apple.security.cs.disable-library-validation',
] as const;
const AD_HOC_PLUGIN_ENTITLEMENTS = [
  'com.apple.security.cs.allow-unsigned-executable-memory',
  'com.apple.security.cs.disable-library-validation',
] as const;
const DEVELOPER_ID_JIT_ENTITLEMENTS = ['com.apple.security.cs.allow-jit'] as const;
const DEVELOPER_ID_PLUGIN_ENTITLEMENTS = [
  'com.apple.security.cs.allow-unsigned-executable-memory',
] as const;

function signingEntitlements(filePath: string): string[] | undefined {
  if (!filePath.endsWith('.app')) return undefined;
  const plugin = filePath.includes('(Plugin).app');
  if (isDeveloperIdSigning) {
    return plugin
      ? [...DEVELOPER_ID_PLUGIN_ENTITLEMENTS]
      : [...DEVELOPER_ID_JIT_ENTITLEMENTS];
  }
  return plugin ? [...AD_HOC_PLUGIN_ENTITLEMENTS] : [...AD_HOC_JIT_ENTITLEMENTS];
}

function requirePath(candidate: string, kind: 'file' | 'directory', executable = false): void {
  const stat = fs.lstatSync(candidate);
  if (stat.isSymbolicLink()) {
    throw new Error(`Release resource must not be a top-level symlink: ${candidate}`);
  }
  if (kind === 'file' ? !stat.isFile() : !stat.isDirectory()) {
    throw new Error(`Release resource is not a ${kind}: ${candidate}`);
  }
  if (executable) fs.accessSync(candidate, fs.constants.X_OK);
}

function resolveReleaseResources(): string[] {
  if (!resourceRoot) {
    if (isReleaseCommand) {
      throw new Error(
        'Release packaging requires PAD_ELECTRON_RESOURCE_DIR. Use scripts/package-electron-app.sh.',
      );
    }
    return [];
  }

  const root = path.resolve(resourceRoot);
  requirePath(root, 'directory');
  requirePath(path.join(root, 'bin'), 'directory');
  for (const name of ['bun', 'node', 'pi']) {
    requirePath(path.join(root, 'bin', name), 'file', true);
  }
  requirePath(path.join(root, 'pi'), 'directory');
  requirePath(path.join(root, 'release-evidence'), 'directory');
  for (const relative of ['package.json', 'dist/bun/cli.js', 'dist/bundle/cli.js']) {
    requirePath(path.join(root, 'pi', relative), 'file');
  }
  for (const name of [
    'runtime-manifest.json',
    'runtime-sbom.spdx.json',
    'runtime-SHA256SUMS.txt',
  ]) {
    requirePath(path.join(root, 'release-evidence', name), 'file');
  }

  return ['bin', 'pi', 'lib', 'release-evidence']
    .map((name) => path.join(root, name))
    .filter((candidate) => fs.existsSync(candidate));
}

function resolveSingleAppBundle(buildPath: string): string {
  const resolvedBuildPath = path.resolve(buildPath);
  const buildStat = fs.lstatSync(resolvedBuildPath);
  if (
    buildStat.isDirectory() &&
    !buildStat.isSymbolicLink() &&
    path.extname(resolvedBuildPath) === '.app'
  ) {
    return resolvedBuildPath;
  }
  if (!buildStat.isDirectory() || buildStat.isSymbolicLink()) {
    throw new Error(`Electron hook path is not a directory: ${resolvedBuildPath}`);
  }

  const candidates = fs
    .readdirSync(resolvedBuildPath, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && !entry.isSymbolicLink() && entry.name.endsWith('.app'))
    .map((entry) => path.join(resolvedBuildPath, entry.name));
  if (candidates.length !== 1) {
    throw new Error(
      `Expected exactly one top-level app bundle in ${resolvedBuildPath}, found ${candidates.length}`,
    );
  }
  return candidates[0];
}

function prepareUnsignedMacBundle(appPath: string): void {
  const infoPath = path.join(appPath, 'Contents', 'Info.plist');
  const resourcesPath = path.join(appPath, 'Contents', 'Resources');
  requirePath(infoPath, 'file');
  requirePath(path.join(resourcesPath, 'bin', 'bun'), 'file', true);
  requirePath(path.join(resourcesPath, 'bin', 'node'), 'file', true);
  requirePath(path.join(resourcesPath, 'bin', 'pi'), 'file', true);
  requirePath(path.join(resourcesPath, 'pi', 'package.json'), 'file');
  requirePath(path.join(resourcesPath, 'pi', 'dist', 'bun', 'cli.js'), 'file');

  for (const key of [
    'NSAppTransportSecurity',
    'NSAudioCaptureUsageDescription',
    'NSBluetoothAlwaysUsageDescription',
    'NSBluetoothPeripheralUsageDescription',
    'NSCameraUsageDescription',
    'NSMicrophoneUsageDescription',
  ]) {
    try {
      execFileSync('/usr/libexec/PlistBuddy', ['-c', `Delete :${key}`, infoPath]);
    } catch {
      // Electron versions differ in their default plist keys.
    }
  }
  execFileSync('/usr/bin/plutil', [
    '-replace',
    'LSMinimumSystemVersion',
    '-string',
    '13.0',
    infoPath,
  ]);
  // electron-packager derives CFBundleDisplayName from executableName after
  // merging extendInfo.  Re-assert the user-facing product name in the final
  // bundle so Finder, the menu bar, and accessibility all agree.
  for (const key of ['CFBundleDisplayName', 'CFBundleName']) {
    execFileSync('/usr/bin/plutil', [
      '-replace',
      key,
      '-string',
      'PAD Desktop',
      infoPath,
    ]);
  }
  execFileSync('/usr/bin/plutil', ['-lint', infoPath]);
}

function ignoreNonMachOBundledPiFile(candidate: string): boolean {
  const piMarker = `${path.sep}Contents${path.sep}Resources${path.sep}pi${path.sep}`;
  if (!candidate.includes(piMarker)) return false;
  let descriptor: number | undefined;
  try {
    const stat = fs.statSync(candidate);
    if (!stat.isFile()) return false;
    descriptor = fs.openSync(candidate, 'r');
    const header = Buffer.alloc(4);
    if (fs.readSync(descriptor, header, 0, header.length, 0) !== header.length) return true;
    const magic = header.readUInt32BE(0);
    return !new Set([
      0xfeedface,
      0xcefaedfe,
      0xfeedfacf,
      0xcffaedfe,
      0xcafebabe,
      0xbebafeca,
      0xcafebabf,
      0xbfbafeca,
    ]).has(magic);
  } catch {
    return true;
  } finally {
    if (descriptor !== undefined) fs.closeSync(descriptor);
  }
}

const extraResource = resolveReleaseResources();

if (isDeveloperIdSigning && !signingIdentity.startsWith('Developer ID Application:')) {
  throw new Error(
    'PAD_DESKTOP_SIGN_IDENTITY must name a Developer ID Application certificate.',
  );
}

if (resourceRoot && !electronZipDir) {
  throw new Error(
    'Release packaging requires PAD_ELECTRON_ZIP_DIR. Use scripts/package-electron-app.sh.',
  );
}

const config: ForgeConfig = {
  packagerConfig: {
    name: 'PAD Desktop',
    executableName: 'PADDesktop',
    appBundleId: 'cn.ghostcloud.pad.desktop',
    appCategoryType: 'public.app-category.developer-tools',
    icon: path.resolve('Resources/PADDesktop.icns'),
    extendInfo: {
      CFBundleDisplayName: 'PAD Desktop',
      CFBundleName: 'PAD Desktop',
      CFBundleIconFile: 'PADDesktop.icns',
      LSMinimumSystemVersion: '13.0',
      NSLocalNetworkUsageDescription: 'PAD Desktop 使用本地网络，让已配对的 iPhone 实时连接这台 Mac。',
    },
    asar: true,
    derefSymlinks: false,
    extraResource,
    electronZipDir,
    // Fuses run in packageAfterCopy. Extra resources and plist edits then
    // complete before electron-packager performs this final recursive sign.
    osxSign: {
      identity: signingIdentity,
      identityValidation: isDeveloperIdSigning,
      preAutoEntitlements: false,
      preEmbedProvisioningProfile: false,
      ignore: ignoreNonMachOBundledPiFile,
      optionsForFile: (filePath) => ({
        entitlements: signingEntitlements(filePath),
        hardenedRuntime: true,
        timestamp: isDeveloperIdSigning ? undefined : 'none',
      }),
    },
    afterCopyExtraResources: [
      (buildPath, _electronVersion, platform, arch, done) => {
        try {
          if (platform !== 'darwin' || arch !== 'arm64') {
            throw new Error(`PAD Desktop release only supports darwin/arm64, got ${platform}/${arch}`);
          }
          prepareUnsignedMacBundle(resolveSingleAppBundle(buildPath));
          done();
        } catch (error) {
          done(error instanceof Error ? error : new Error(String(error)));
        }
      },
    ],
    afterComplete: [
      (buildPath, _electronVersion, platform, _arch, done) => {
        try {
          if (platform !== 'darwin') {
            done();
            return;
          }
          const appPath = resolveSingleAppBundle(buildPath);
          execFileSync('/usr/bin/codesign', [
            '--verify',
            '--deep',
            '--strict',
            '--verbose=2',
            appPath,
          ], { stdio: 'inherit' });
          done();
        } catch (error) {
          done(error instanceof Error ? error : new Error(String(error)));
        }
      },
    ],
  },
  rebuildConfig: {},
  makers: [new MakerZIP({}, ['darwin'])],
  plugins: [
    new VitePlugin({
      build: [
        {
          entry: 'electron/main/index.ts',
          config: 'vite.main.config.ts',
          target: 'main',
        },
        {
          entry: 'electron/preload/index.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
      ],
      renderer: [
        {
          name: 'main_window',
          config: 'vite.renderer.config.ts',
        },
      ],
    }),
    new FusesPlugin({
      version: FuseVersion.V1,
      [FuseV1Options.RunAsNode]: false,
      // PAD never stores authentication in Chromium cookies. Enabling this
      // fuse would synchronously request the user's login Keychain before the
      // first window exists, which breaks clean/managed Macs and adds an
      // unrelated confirmation prompt.
      [FuseV1Options.EnableCookieEncryption]: false,
      [FuseV1Options.EnableNodeOptionsEnvironmentVariable]: false,
      [FuseV1Options.EnableNodeCliInspectArguments]: false,
      [FuseV1Options.EnableEmbeddedAsarIntegrityValidation]: true,
      [FuseV1Options.OnlyLoadAppFromAsar]: true,
      [FuseV1Options.LoadBrowserProcessSpecificV8Snapshot]: false,
      [FuseV1Options.GrantFileProtocolExtraPrivileges]: false,
    }),
  ],
};

export default config;
