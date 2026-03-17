import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import UnoCSS from '@unocss/vite';
import { kavach } from '@kavach/vite';

export default defineConfig({
  plugins: [
    kavach(),          // must come before sveltekit
    UnoCSS(),
    sveltekit(),
  ],
  optimizeDeps: {
    exclude: ['@rokkit/ui', '@rokkit/states', '@rokkit/actions', '@rokkit/core', '@rokkit/app']
  }
});
