import { healthState } from "$lib/health-state.svelte.ts";

export function reroute({ url }: { url: URL }): string | undefined {
  if (url.pathname !== "/health" && !healthState.isOk) return "/health";
  return undefined;
}
