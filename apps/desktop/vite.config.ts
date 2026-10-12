import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => ({
  plugins: [react()],
  resolve: {
    alias: {
      "#backend": decodeURIComponent(
        new URL(
          mode === "preview"
            ? "./src/lib/backend.ts"
            : "./src/lib/backend-tauri.ts",
          import.meta.url,
        ).pathname.replace(/^\/([A-Za-z]:)/, "$1"),
      ),
    },
  },
  publicDir: "public",
  clearScreen: false,
  server: {
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
    sourcemap: mode === "diagnostics" ? "hidden" : false,
  },
}));
