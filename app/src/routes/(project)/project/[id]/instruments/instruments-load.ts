import { readOutcome, type ApiResult } from '$lib/api.js';
import type { ProjectMcpToolStat, ProjectService } from '$lib/types.js';
import type { ToolUsage } from '$lib/metrics/metric-view.js';

type Extension = { id: string; name: string; kind: string; scope: 'global' | 'project'; enabled: boolean };

/** The reads the project Instruments page makes — injected so a spec can fail them. */
export interface InstrumentsApi {
  tryGetProjectInstruments: (id: string) => Promise<ApiResult<{ tools: Extension[] }>>;
  tryGetProjectMcpToolStats: (id: string) => Promise<ApiResult<{ tools: ProjectMcpToolStat[] }>>;
  tryListProjectServices: (id: string) => Promise<ApiResult<{ services: ProjectService[] }>>;
  tryGetProjectTools: (id: string) => Promise<ApiResult<{ tools: ToolUsage[] }>>;
}

/**
 * The project's instruments: its extensions (skills / commands / agents), every
 * manifest MCP tool with this project's usage, and its services with scope
 * resolved. Any read that fails is the page's error (#288) — not the
 * "0 tools · 0 calls" an empty default made of it.
 */
export async function loadProjectInstruments(api: InstrumentsApi, id: string) {
  const [ext, mcp, services, usage] = await Promise.all([
    api.tryGetProjectInstruments(id),
    api.tryGetProjectMcpToolStats(id),
    api.tryListProjectServices(id),
    api.tryGetProjectTools(id),
  ]);
  const [e, m, s, u] = [readOutcome(ext), readOutcome(mcp), readOutcome(services), readOutcome(usage)];
  return {
    tools: e.data?.tools ?? [],
    mcpToolStats: m.data?.tools ?? [],
    services: s.data?.services ?? [],
    // Which tools the assistants used, built-ins included (#291) — its own
    // read and its own error, so a failure here does not blank the page.
    toolUsage: u.data?.tools ?? [],
    toolUsageError: u.error,
    error: e.error ?? m.error ?? s.error,
  };
}
