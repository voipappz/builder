import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

// VoipAppz Builder's chat, built into ../client/: one client.js and one
// client.css (Builder's page loads exactly these two), assets beside them.
// core/build.rs embeds the folder; Builder's server serves it at /builder/.
export default defineConfig({
  plugins: [react()],
  base: './',
  publicDir: false,
  server: { cors: true },
  build: {
    target: 'es2022',
    outDir: resolve(import.meta.dirname, '../client'),
    emptyOutDir: true,
    sourcemap: false,
    cssCodeSplit: false,
    chunkSizeWarningLimit: 10000,
    // The audio worklet (botCall.js loads it with new URL(..., import.meta.url))
    // must stay its own same-origin file: the page's CSP allows no data: or
    // blob: scripts, so nothing is inlined.
    assetsInlineLimit: 0,
    rollupOptions: {
      input: resolve(import.meta.dirname, 'src/main.jsx'),
      output: {
        entryFileNames: 'client.js',
        inlineDynamicImports: true,
        assetFileNames: (asset) =>
          (asset.names?.[0] ?? asset.name ?? '').endsWith('.css') ? 'client.css' : 'assets/[name]-[hash][extname]'
      }
    }
  }
});
