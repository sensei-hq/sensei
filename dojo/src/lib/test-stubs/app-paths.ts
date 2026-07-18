// Test stub for SvelteKit's `$app/paths` virtual module. Vitest runs outside the
// SvelteKit plugin (see vitest.config.ts), so `resolve` isn't generated; this alias
// supplies a pass-through that returns the route id as the href, which is all a
// component render test needs (it asserts behavior + text, not exact URLs).
export function resolve(id: string): string {
	return id;
}
