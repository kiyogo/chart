import { defineConfig } from 'vite';
import solidPlugin from 'vite-plugin-solid';

// Detect if running inside Tauri build
const isTauri = process.env.TAURI_ENV_PLATFORM !== undefined
  || process.env.TAURI_DEBUG !== undefined;

export default defineConfig({
  plugins: [solidPlugin()],

  // Tauri needs a relative base path for bundled assets
  base: isTauri ? './' : '/',

  // Prevent Vite from obscuring Rust panics
  clearScreen: false,

  server: {
    host: '127.0.0.1',
    port: 1420,
    // Tauri expects a fixed port; fail if it's not available
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8085',
        changeOrigin: true,
        ws: true,
      },
    },
  },

  // Expose env variables to the frontend
  envPrefix: ['VITE_', 'TAURI_'],

  build: {
    target: 'esnext',
    outDir: 'dist',
    emptyOutDir: true,
  },
});
