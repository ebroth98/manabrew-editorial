import path from "path"
import { defineConfig, type Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

const host = process.env.TAURI_DEV_HOST

/** Adds COOP/COEP headers to enable SharedArrayBuffer for Atomics.wait() */
function crossOriginIsolation(): Plugin {
  return {
    name: "cross-origin-isolation",
    configureServer(server) {
      server.middlewares.use((_req, res, next) => {
        res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
        res.setHeader("Cross-Origin-Embedder-Policy", "credentialless");
        next();
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), tailwindcss(), crossOriginIsolation()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  // Tauri: don't obscure rust errors
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**", '**/forge/**', '**/node_modules/**'],
    },
    // Required for SharedArrayBuffer (used by game engine Atomics.wait)
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "credentialless",
    },
  },
  // Web Worker configuration
  worker: {
    format: 'es',
  },
  // Exclude WASM from dependency optimization
  optimizeDeps: {
    exclude: ['@/wasm/forge_wasm'],
  },
  // Build configuration for WASM
  build: {
    target: 'esnext',
  },
})
