import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["renderer/src/test/setup.ts"],
    include: [
      "electron/**/*.test.ts",
      "renderer/src/**/*.test.ts",
      "renderer/src/**/*.test.tsx",
    ],
    clearMocks: true,
  },
});
