import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  test: {
    include: ['src/**/*.spec.ts', 'src/**/*.spec.svelte.ts'],
    // Route component tests run in jsdom; lib tests run in node
    environmentMatchGlobs: [
      ['src/routes/**/*.spec.svelte.ts', 'jsdom'],
      ['src/routes/**/*.spec.ts', 'jsdom'],
    ],
  },
  resolve: {
    // 'browser' condition ensures Svelte resolves to its client build
    conditions: ['browser'],
    alias: {
      '$lib': '/src/lib',
      '$app': '/src/app',
    },
  },
});
