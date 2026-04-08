import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import UnoCSS from '@unocss/vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [UnoCSS(), sveltekit()],

  // Tauri: don't open browser, use Tauri window
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 5183 } : undefined,
  },

  // Tauri expects a inlined build with no CDN
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },

  optimizeDeps: {
    exclude: ['@rokkit/ui', '@rokkit/states', '@rokkit/actions', '@rokkit/core', '@rokkit/app'],
  },
});
