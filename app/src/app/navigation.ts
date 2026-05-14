// Test-only stub for $app/navigation — vitest aliases $app → src/app.
// The real implementations are provided by SvelteKit at build time; tests
// either mock these or rely on the no-op default.

export const goto = (..._args: unknown[]): Promise<void> => Promise.resolve();
