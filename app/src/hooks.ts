import { appState } from "$lib/appstate.svelte.js";

const ALWAYS_REACHABLE = new Set(["/health", "/logs", "/upgrade"]);

export function reroute({ url }: { url: URL }): string | undefined {
  const path = url.pathname;
  if (!ALWAYS_REACHABLE.has(path) && !appState.healthOk) return "/health";
  if (!ALWAYS_REACHABLE.has(path) && !path.startsWith("/setup") && !appState.setupOk)
    return "/setup/welcome";
  return undefined;
}
