import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import unocss from '@unocss/vite';

export default defineConfig({
  plugins: [unocss(), sveltekit()],
  optimizeDeps: {
    exclude: ['@rokkit/ui', '@rokkit/states', '@rokkit/actions', '@rokkit/core', '@rokkit/app']
  }
});
