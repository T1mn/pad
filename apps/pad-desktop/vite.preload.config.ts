import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    sourcemap: true,
    rollupOptions: {
      input: 'electron/preload/index.ts',
      output: {
        format: 'cjs',
        inlineDynamicImports: true,
        entryFileNames: 'preload.js',
        chunkFileNames: '[name].js',
        assetFileNames: '[name].[ext]',
      },
    },
  },
});
