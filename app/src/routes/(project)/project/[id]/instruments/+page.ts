import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  const [ext, mcp, services] = await Promise.all([
    api.getProjectInstruments(params.id),
    api.getProjectMcpToolStats(params.id),
    api.listProjectServices(params.id),
  ]);
  return {
    project,
    // Extensions (skills/commands/agents) — historical shape, kept intact.
    tools: ext.tools ?? [],
    // MCP tool aggregation — every manifest tool with per-project usage.
    mcpToolStats: mcp.tools ?? [],
    // Installed services with per-project scope resolved (T2 Slice B).
    services: services.services ?? [],
  };
};
