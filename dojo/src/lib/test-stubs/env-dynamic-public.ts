// Test stub for SvelteKit's `$env/dynamic/public` virtual module. Vitest runs
// outside the SvelteKit plugin (see vitest.config.ts), so the virtual module
// isn't generated; this alias supplies an empty env so the triage API client's
// base-URL fallback path is exercised.
export const env: Record<string, string | undefined> = {};
