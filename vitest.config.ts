import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  test: {
    include: ['src/**/*.test.ts', 'src/**/*.svelte.test.ts'],
  },
  resolve: {
    alias: {
      '$lib': '/src/lib',
      '$app': '/src/app',
    },
  },
});
