import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  test: {
    include: ['src/**/*.spec.ts', 'src/**/*.spec.svelte.ts'],
  },
  resolve: {
    alias: {
      '$lib': '/src/lib',
      '$app': '/src/app',
    },
  },
});
