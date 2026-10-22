import { readOutcome, type ApiResult } from '$lib/api.js';
import type { WorkspaceMcpToolStat, WorkspaceService } from '$lib/types.js';
import type { ToolUsage } from '$lib/metrics/metric-view.js';

type Extension = { id: string; name: string; kind: string; scope: 'global' | 'workspace'; enabled: boolean };

/** The reads the workspace Instruments page makes — injected so a spec can fail them. */
export interface InstrumentsApi {
  tryGetWorkspaceInstruments: (id: string) => Promise<ApiResult<{ tools: Extension[] }>>;
  tryGetWorkspaceMcpToolStats: (id: string) => Promise<ApiResult<{ tools: WorkspaceMcpToolStat[] }>>;
  tryListWorkspaceServices: (id: string) => Promise<ApiResult<{ services: WorkspaceService[] }>>;
  tryGetWorkspaceTools: (id: string) => Promise<ApiResult<{ tools: ToolUsage[] }>>;
}

/**
 * The workspace's instruments: its extensions (skills / commands / agents), every
 * manifest MCP tool with this workspace's usage, and its services with scope
 * resolved. Any read that fails is the page's error (#288) — not the
 * "0 tools · 0 calls" an empty default made of it.
 */
export async function loadWorkspaceInstruments(api: InstrumentsApi, id: string) {
  const [ext, mcp, services, usage] = await Promise.all([
    api.tryGetWorkspaceInstruments(id),
    api.tryGetWorkspaceMcpToolStats(id),
    api.tryListWorkspaceServices(id),
    api.tryGetWorkspaceTools(id),
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
