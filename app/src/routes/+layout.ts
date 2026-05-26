// SPA mode for Tauri — disable SSR and prerendering.
//
// appState.load() is intentionally NOT called here. The /health route does
// not need daemon config, and forcing a load() at the root made every
// navigation issue a /api/config round-trip — including before the daemon
// was even up. Each route group that needs config loads it in its own
// +layout.ts (see `(observatory)/+layout.ts`, `(project)/+layout.ts`,
// `(config)/+layout.ts`).
export const ssr = false;
export const prerender = false;
