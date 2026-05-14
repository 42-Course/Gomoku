import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'

// engine-wasm lives one directory up; Vite's default fs.allow only covers
// the project root, so the worker's request for engine_wasm_bg.wasm gets a
// 403. Whitelist the sibling pkg/ folder explicitly.
const enginePkgDir = path.resolve(__dirname, '../engine-wasm/pkg')

// GitHub Pages serves the site under `/<repo>/`. Set the base at build time
// via VITE_BASE so the same config can target a custom domain (base="/")
// or a forked repo with a different name without touching the file.
const base = process.env.VITE_BASE ?? '/'

export default defineConfig({
  base,
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    fs: {
      allow: [path.resolve(__dirname), enginePkgDir],
    },
  },
  optimizeDeps: {
    exclude: ['engine-wasm'],
  },
})
