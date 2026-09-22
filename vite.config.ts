// defineConfig comes from vitest/config (a superset of vite's) so the `test`
// block below is typed rather than tolerated.
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'node:path';

// `__dirname` does not exist in ESM, and this config is ESM ("type": "module").
const here = import.meta.dirname;

// Three windows, three entry points. Each is created on demand by the Rust core
// and destroyed after use — see docs/architecture/system-design.md.
export default defineConfig({
  plugins: [react(), tailwindcss()],

  // Tauri expects a fixed port and no obscured errors.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // The Rust core has its own rebuild loop; watching it here just burns CPU.
      ignored: ['**/src-tauri/**'],
    },
  },

  build: {
    // WebView2 is evergreen Chromium, so we can target a modern baseline.
    target: 'chrome110',
    sourcemap: process.env.TAURI_ENV_DEBUG === 'true',
    minify: process.env.TAURI_ENV_DEBUG === 'true' ? false : 'esbuild',
    rollupOptions: {
      input: {
        overlay: resolve(here, 'overlay.html'),
        panel: resolve(here, 'panel.html'),
        settings: resolve(here, 'settings.html'),
      },
    },
  },

  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
