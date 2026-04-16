import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import UnoCSS from '@unocss/vite';
import path from 'path';

const themesRoot = 'node_modules/@rokkit/themes/src';

export default defineConfig({
  plugins: [
    UnoCSS(),
    sveltekit(),
  ],
  resolve: {
    alias: {
      // rokkit#136 — @rokkit/themes doesn't ship dist/, resolve CSS to source
      '@rokkit/themes/base.css': path.resolve(`${themesRoot}/base/index.css`),
      '@rokkit/themes/rokkit.css': path.resolve(`${themesRoot}/rokkit/index.css`),
    }
  }
});
