// Test alias for `@kavach/ui` — the real components need the kavach context +
// @rokkit auth stack; specs assert DojoSignIn renders, so a light AuthProvider
// stub is enough. See vitest.config.ts.
export { default as AuthProvider } from './AuthProviderStub.svelte';
