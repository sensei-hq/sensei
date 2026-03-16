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
  resolve: {
    alias: {
      // kavach exports point to .ts source — alias directly so Vite SSR runner can transform it
      'kavach': '/Users/Jerry/Developer/kavach/packages/auth/src/index.ts'
    }
  },
  optimizeDeps: {
    exclude: ['@rokkit/ui', '@rokkit/states', '@rokkit/actions', '@rokkit/core', '@rokkit/app']
  }
});
