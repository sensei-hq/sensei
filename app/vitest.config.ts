import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  plugins: [svelte()],
  // Mirror the vite-defines from `vite.config.ts` so tests see the same
  // compile-time constants the production bundle does. Without this,
  // `storage-keys.ts` would explode with `__SENSEI_NAMESPACE__ is not
  // defined` because vitest doesn't pick up the app vite.config.ts.
  define: {
    __SENSEI_NAMESPACE__:   JSON.stringify('sensei'),
    __SENSEI_APP_VERSION__: JSON.stringify('0.0.0-test'),
  },
  test: {
    include: ['src/**/*.spec.ts', 'src/**/*.spec.svelte.ts'],
    // jsdom lacks window.matchMedia, which @rokkit/chart's barrel needs at load
    // (AnimatedPlot → svelte/motion). Stub it so chart component tests can run.
    setupFiles: ['./tests/stubs/match-media.ts'],
    // Route component tests run in jsdom; lib tests run in node
    environmentMatchGlobs: [
      ['src/routes/**/*.spec.svelte.ts', 'jsdom'],
      ['src/routes/**/*.spec.ts', 'jsdom'],
    ],
    // lcov for the qlty coverage upload (paths are relative to app/, so CI
    // adds the `app` prefix). `bun run test:unit --coverage` writes ./coverage.
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      reportsDirectory: './coverage',
      include: ['src/**'],
      exclude: ['src/**/*.spec.*', 'src/**/*.harness.svelte', 'src/**/test-stubs/**'],
    },
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
