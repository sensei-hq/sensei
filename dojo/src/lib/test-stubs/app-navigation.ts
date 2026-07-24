// Test stub for SvelteKit's `$app/navigation` virtual module. Vitest runs
// outside the SvelteKit plugin (see vitest.config.ts), so `goto` isn't
// generated; this alias supplies a no-op so components that call it on an
// interaction (e.g. LogoutButton's signOut → goto('/signin')) import cleanly
// under render tests. Tests assert behavior/text, not real navigation.
export function goto(_url: string | URL, _opts?: unknown): Promise<void> {
	return Promise.resolve();
}

export function invalidateAll(): Promise<void> {
	return Promise.resolve();
}
