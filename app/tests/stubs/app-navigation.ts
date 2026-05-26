// Vitest stub for SvelteKit's `$app/navigation`.
//
// SvelteKit generates `$app/*` modules at build time. Under Vitest there's
// no SvelteKit runtime, so any component or hook that imports
// `$app/navigation` fails Vite's import resolution. This stub satisfies
// the resolver with no-op exports.
//
// `vitest.config.ts` aliases `$app/navigation` → this file. Production
// builds (Tauri / dev / release) use SvelteKit's real `$app/navigation`
// and never touch this file.

export const goto =          (..._a: unknown[]): Promise<void> => Promise.resolve();
export const invalidate =    (..._a: unknown[]): Promise<void> => Promise.resolve();
export const invalidateAll = (..._a: unknown[]): Promise<void> => Promise.resolve();
export const preloadData =   (..._a: unknown[]): Promise<unknown> => Promise.resolve({});
export const preloadCode =   (..._a: unknown[]): Promise<void> => Promise.resolve();
export const beforeNavigate = (..._a: unknown[]): void => {};
export const afterNavigate =  (..._a: unknown[]): void => {};
export const pushState =      (..._a: unknown[]): void => {};
export const replaceState =   (..._a: unknown[]): void => {};
