import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: [sveltekit()],

  // Mode `mock` ersetzt @tauri-apps/api/core durch die Mock-Schicht
  // (src/lib/mocks/). Aktivierung über `pnpm dev:mock`. Erlaubt es,
  // die App ohne Tauri-Backend im Browser zu fahren — für Playwright
  // und manuelle Frontend-Arbeit.
  resolve:
    mode === "mock"
      ? {
          alias: {
            "@tauri-apps/api/core": fileURLToPath(
              new URL("./src/lib/mocks/index.ts", import.meta.url),
            ),
            "@tauri-apps/api/event": fileURLToPath(
              new URL("./src/lib/mocks/event.ts", import.meta.url),
            ),
            // plugin-os reads window.__TAURI_OS_PLUGIN_INTERNALS__ synchronously
            // and throws in a plain browser; the shim lets the app boot in mock.
            "@tauri-apps/plugin-os": fileURLToPath(
              new URL("./src/lib/mocks/plugin-os.ts", import.meta.url),
            ),
          },
        }
      : undefined,

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
