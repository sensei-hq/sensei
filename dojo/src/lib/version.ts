// Single source of truth for the build-time version stamp — imported by both
// the console footer (ConsoleNav.svelte) and GET /version so a live deploy is
// verifiable. The three values are injected by vite.config.ts `define`
// (__DOJO_VERSION__ / __DOJO_GIT_SHA__ / __DOJO_BUILD_TIME__), declared ambient
// in src/app.d.ts.
//
// The `typeof … === 'undefined'` guards keep this module importable under vitest,
// which doesn't run the SvelteKit/vite `define` step (see vitest.config.ts) — the
// globals are simply absent there, so version.spec.ts asserts the exported shape
// rather than exact build values.
export const dojoBuild = {
	version: typeof __DOJO_VERSION__ === 'undefined' ? 'unknown' : __DOJO_VERSION__,
	gitSha: typeof __DOJO_GIT_SHA__ === 'undefined' ? 'unknown' : __DOJO_GIT_SHA__,
	builtAt: typeof __DOJO_BUILD_TIME__ === 'undefined' ? 'unknown' : __DOJO_BUILD_TIME__
} as const;
