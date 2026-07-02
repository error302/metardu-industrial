import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    // Target modern browsers — Tauri ships with a recent system webview,
    // so we can skip legacy transpilation and keep bundles lean.
    target: 'es2022',
    // Manual chunk strategy — split heavy geospatial libs into their own
    // chunks so the main app bundle stays small. OpenLayers, proj4, and
    // Tauri API only load when the workspace shell actually mounts.
    rolldownOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules')) {
            if (id.includes('/ol/') || id.includes('/proj4/')) {
              return 'ol-vendor'
            }
            if (id.includes('/react/') || id.includes('/react-dom/')) {
              return 'react-vendor'
            }
            if (id.includes('/@tauri-apps/')) {
              return 'tauri-vendor'
            }
            if (id.includes('/lucide-react/')) {
              return 'icons-vendor'
            }
            if (id.includes('/@deck.gl/') || id.includes('/@luma.gl/') || id.includes('/@math.gl/')) {
              return 'deckgl-vendor'
            }
          }
          return undefined
        },
      },
    },
    // Suppress the 500KB warning — we've intentionally split into chunks
    chunkSizeWarningLimit: 800,
  },
})
