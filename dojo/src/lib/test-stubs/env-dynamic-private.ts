// Test stub for SvelteKit's `$env/dynamic/private` virtual module. Vitest runs
// outside the SvelteKit plugin (see vitest.config.ts), so the virtual module
// isn't generated; this alias supplies an empty env so server-only modules that
// read private env at call time (e.g. dojo-supabase's `dojoDb`) import cleanly.
// Specs that exercise those modules mock `dojoDb` itself, so no real key is used.
export const env: Record<string, string | undefined> = {};
