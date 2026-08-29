import react from '@vitejs/plugin-react';
import path from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  root: path.resolve('renderer'),
  plugins: [react()],
  resolve: {
    alias: {
      '@shared': path.resolve('shared'),
    },
  },
  build: {
    sourcemap: true,
    // Forge's default outDir is relative to `root`; keep all packaged assets
    // under the application-level .vite directory instead of renderer/.vite.
    outDir: path.resolve('.vite/renderer/main_window'),
  },
});
