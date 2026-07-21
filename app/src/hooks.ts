import { appState } from "$lib/appstate.svelte.js";

const ALWAYS_REACHABLE = new Set(["/health", "/logs", "/upgrade"]);

// A project window loads `/project/{id}/...` in its own fresh webview heap,
// where healthState starts 'checking' (isOk=false) and setup state is
// unhydrated. It is only ever opened from an already-healthy main window, so it
// must bypass the boot gates — otherwise reroute bounces the fresh window to
// /health and the project never renders. This matches the singular `/project/`
// window routes, not the observatory's plural `/projects` list (still gated).
function isReachable(path: string): boolean {
  return ALWAYS_REACHABLE.has(path) || path.startsWith("/project/");
}

export function reroute({ url }: { url: URL }): string | undefined {
  const path = url.pathname;
  if (!isReachable(path) && !appState.healthOk) return "/health";
  if (!isReachable(path) && !path.startsWith("/setup") && !appState.setupOk)
    return "/setup/welcome";
  return undefined;
}
