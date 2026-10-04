#!/usr/bin/env node
// Run explicitly from apps/pad-terminal; never invoked by PAD startup.
import { migrateDataRoot } from '../host/data-root-migration.mjs';

try {
  const args = process.argv.slice(2);
  if (args.length !== 1 || args[0] !== '--confirm-closed') {
    throw new Error('Usage: node scripts/migrate-data-root.mjs --confirm-closed\nThis confirms ALL PAD and standalone Pi clients for the legacy root are closed.');
  }
  const receipt = migrateDataRoot({ confirmClosed: true });
  console.log(`Migrated PAD data to ${receipt.destination}. Source retained at ${receipt.source}.`);
  console.log('Do not reuse old binaries against the retained source: it may diverge. Default startup will then require an explicit root choice.');
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
