// Daemon port. Hardcoded — there is one daemon, one port, one source of
// truth. The previous indirection (a vite-define + a localStorage override)
// added two failure modes (stale localStorage value, stale bundle) without
// any callsite that actually needed runtime variability.
//
// A leaf module on purpose: the scan singleton (`scan.ts`) builds its
// source URL at module scope, and importing it from `appstate` would close
// an appstate → wizard-state → scan → appstate cycle.
export const DEFAULT_PORT = 7744;

/** The daemon's origin. `apiBase` in `api.ts` delegates here. */
export const daemonBaseUrl = (port: number = DEFAULT_PORT): string => `http://127.0.0.1:${port}`;
