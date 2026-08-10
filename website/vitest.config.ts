import { defineConfig } from 'vitest/config';

// Unit tests for the site's data model + pure view helpers. Component/render
// coverage is Playwright's job (tests/); these run with no browser.
export default defineConfig({
	test: {
		include: ['src/**/*.{test,spec}.ts'],
		environment: 'node',
	},
});
