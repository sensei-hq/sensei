import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

// Standalone vitest config for component/state specs. Kept separate from
// vite.config.ts so the kavach + SvelteKit plugins (which need the virtual
// $kavach/* modules and a running dev server) don't load during unit tests.
export default defineConfig({
	plugins: [svelte(), svelteTesting()],
	test: {
		environment: 'jsdom',
		include: ['src/**/*.{test,spec}.{js,ts}'],
		globals: true,
		setupFiles: []
	},
	resolve: {
		alias: {
			$lib: new URL('./src/lib', import.meta.url).pathname,
			// $env/dynamic/public isn't generated outside the SvelteKit plugin; point
			// it at a stub so the triage API client imports cleanly under vitest.
			'$env/dynamic/public': new URL(
				'./src/lib/test-stubs/env-dynamic-public.ts',
				import.meta.url
			).pathname,
			// $app/paths (resolve) likewise isn't generated under vitest; a pass-through
			// stub lets components that build hrefs (nav, top bar) render in specs.
			'$app/paths': new URL('./src/lib/test-stubs/app-paths.ts', import.meta.url).pathname
		}
	}
});
