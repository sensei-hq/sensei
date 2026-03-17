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
      // kavach packages export .ts source — alias to JS entry so Vite SSR can resolve them
      'kavach': '/Users/Jerry/Developer/kavach/packages/auth/src/index.ts',
      '@kavach/adapter-supabase': '/Users/Jerry/Developer/kavach/adapters/supabase/src/index.ts',
      '@kavach/query': '/Users/Jerry/Developer/kavach/packages/query/src/index.js',
    }
  },
  optimizeDeps: {
    exclude: ['@rokkit/ui', '@rokkit/states', '@rokkit/actions', '@rokkit/core', '@rokkit/app']
  }
});
