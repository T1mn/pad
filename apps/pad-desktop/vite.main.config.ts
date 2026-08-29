import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    sourcemap: true,
    lib: {
      entry: 'electron/main/index.ts',
      fileName: () => 'main.js',
      formats: ['cjs'],
    },
  },
});
