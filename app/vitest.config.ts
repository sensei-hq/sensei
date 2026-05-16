import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

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
      // SvelteKit generates `$app/*` modules at build time. Under Vitest
      // there's no SvelteKit runtime, so any component or hook that
      // imports `$app/navigation` fails Vite's resolution. Point it at a
      // local stub that lives outside src/ — see tests/stubs/app-navigation.ts.
      '$app/navigation': fileURLToPath(new URL('./tests/stubs/app-navigation.ts', import.meta.url)),
    },
  },
});
