/**
 * MCP state slice.
 *
 * Owns the observatory Instruments tab's data — tool manifests, per-tool
 * usage stats, derived signals, cached tool insights, and per-session
 * replay timelines. Each screen reads from this store; the page component
 * stays pure presentation.
 *
 * Loading is idempotent: `loadCatalog()` / `loadInsights()` short-circuit
 * on subsequent calls once loaded, and `refresh()` forces a re-fetch.
 * Session timelines are cached per session id so re-selecting a session
 * doesn't round-trip to the daemon.
 */
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type {
  McpToolManifest, SessionToolCall, ToolInsight, ToolSignal,
  SessionReplayCall, McpServerRow, McpServerToolsManifest,
  ToolHealthSource,
} from '$lib/types.js';

type ToolStat = {
  tool_name: string;
  call_count: number;
  error_count: number;
  avg_duration_ms: number | null;
  last_used_at: string;
};

/** Per-tool health verdict for the Instruments · Health L2 usage table,
 *  using the mockup's vocabulary. Pure derivation from an aggregate row's
 *  call/error counts so it can be unit-tested without the store:
 *  - `unused`  — no recorded calls in the window
 *  - `warn`    — error rate above 10%
 *  - `healthy` — calls recorded with zero errors
 *  - `ok`      — calls recorded with a tolerable (≤10%) error rate
 */
export type ToolVerdict = 'healthy' | 'ok' | 'warn' | 'unused';

export function toolVerdict(callCount: number, errorCount: number): ToolVerdict {
  if (callCount <= 0) return 'unused';
  if (errorCount / callCount > 0.1) return 'warn';
  if (errorCount === 0) return 'healthy';
  return 'ok';
}

// Loading lifecycle — 'idle' before first load, 'loading' during the
// first fetch, 'ready' once populated, 'error' if the fetch threw. The
// three main datasets each carry their own state so a signal fetch
// failing doesn't hide manifest data.
type LoadStatus = 'idle' | 'loading' | 'ready' | 'error';

let catalogStatus = $state<LoadStatus>('idle');
let insightsStatus = $state<LoadStatus>('idle');

let tools = $state<McpToolManifest[]>([]);
let toolStats = $state<ToolStat[]>([]);
let toolSignals = $state<ToolSignal[]>([]);
let toolInsights = $state<ToolInsight[]>([]);
/** Where the current tool-signals response came from (cache vs derived). */
let signalSource = $state<'cache' | 'derived' | null>(null);

// Session timelines cache — keyed by session id so replay flips stay
// instant after the first fetch. Cleared on refresh() alongside the
// primary datasets.
let sessionTimelines = $state<Record<string, SessionToolCall[]>>({});

async function loadCatalog(force = false): Promise<void> {
  if (catalogStatus === 'loading') return;
  if (!force && catalogStatus === 'ready') return;
  catalogStatus = 'loading';
  try {
    const api = senseiApi(appState.port);
    const [catalog, stats, signals] = await Promise.all([
      api.mcpListTools(),
      api.getToolUsage(),
      api.getToolSignals(),
    ]);
    tools = catalog.tools;
    toolStats = stats.tools ?? [];
    toolSignals = signals.signals ?? [];
    signalSource = signals.source ?? null;
    catalogStatus = 'ready';
  } catch {
    catalogStatus = 'error';
  }
}

async function loadInsights(force = false): Promise<void> {
  if (insightsStatus === 'loading') return;
  if (!force && insightsStatus === 'ready') return;
  insightsStatus = 'loading';
  try {
    const api = senseiApi(appState.port);
    const resp = await api.getToolInsights();
    toolInsights = resp.insights ?? [];
    insightsStatus = 'ready';
  } catch {
    insightsStatus = 'error';
  }
}

async function loadSessionTimeline(sessionId: string, force = false): Promise<SessionToolCall[]> {
  if (!force && sessionTimelines[sessionId]) {
    return sessionTimelines[sessionId];
  }
  const api = senseiApi(appState.port);
  const resp = await api.getSessionToolTimeline(sessionId, 200);
  sessionTimelines = { ...sessionTimelines, [sessionId]: resp.calls };
  return resp.calls;
}

// ── #84 T2 Slice C — Replay with #90 verdicts ───────────────────────────

/** Cached replay timelines keyed by session id, including per-call verdicts
 *  and the session-level summary. Cleared alongside primary datasets on
 *  `refresh()`. */
type ReplayCache = {
  calls: SessionReplayCall[];
  summary: { used: number; partial: number; ignored: number; total: number };
};
let sessionReplays = $state<Record<string, ReplayCache>>({});

/** Fetch the Replay endpoint (paired timeline + verdict join + summary).
 *  Passing `classify: true` runs the classifier before the read so a
 *  first-open populates verdicts in one round-trip. */
async function loadSessionReplay(
  sessionId: string,
  opts: { force?: boolean; classify?: boolean } = {},
): Promise<ReplayCache> {
  if (!opts.force && sessionReplays[sessionId]) {
    return sessionReplays[sessionId];
  }
  const api = senseiApi(appState.port);
  const resp = await api.getSessionReplay(sessionId, { classify: opts.classify });
  const cache: ReplayCache = { calls: resp.calls, summary: resp.summary };
  sessionReplays = { ...sessionReplays, [sessionId]: cache };
  return cache;
}

// ── Instruments · Health L1 — tool-health source grid ──────────────────

/** One row per registered source (builtin tool set or probed MCP server)
 *  from GET /api/instruments/tools-health. Drives the L1 share grid; the
 *  L2 per-tool drill still reads tool-signals / tool-insights above. */
let toolsHealth = $state<ToolHealthSource[]>([]);
let toolsHealthStatus = $state<LoadStatus>('idle');

/** Load the L1 source grid. Idempotent — a second call short-circuits once
 *  ready unless `force` is set (mirrors loadCatalog / loadInsights). */
async function loadToolsHealth(force = false): Promise<void> {
  if (toolsHealthStatus === 'loading') return;
  if (!force && toolsHealthStatus === 'ready') return;
  toolsHealthStatus = 'loading';
  try {
    const api = senseiApi(appState.port);
    const resp = await api.getToolsHealth();
    toolsHealth = resp.sources ?? [];
    toolsHealthStatus = 'ready';
  } catch {
    toolsHealthStatus = 'error';
  }
}

/** Grid order — connected sources first, then by 14d call volume desc so
 *  the surfaces earning their keep lead. */
function sortedToolsHealth(): ToolHealthSource[] {
  return [...toolsHealth].sort((a, b) =>
    a.connected === b.connected
      ? b.calls_14d - a.calls_14d
      : (a.connected ? -1 : 1),
  );
}

/** Header KPIs derived from the same grid rows. Tool coverage counts only
 *  probed sources (tools_registered known); it is null when nothing has been
 *  probed rather than a fabricated 0%. */
function toolsHealthKpis(): {
  total: number; connected: number;
  sumRegistered: number; sumInvoked: number;
  coverage: number | null; totalCalls: number;
} {
  const total = toolsHealth.length;
  const connected = toolsHealth.filter((s) => s.connected).length;
  const probed = toolsHealth.filter((s) => s.tools_registered != null);
  const sumRegistered = probed.reduce((acc, s) => acc + (s.tools_registered ?? 0), 0);
  const sumInvoked = probed.reduce((acc, s) => acc + s.tools_invoked_14d, 0);
  const coverage = sumRegistered > 0 ? sumInvoked / sumRegistered : null;
  const totalCalls = toolsHealth.reduce((acc, s) => acc + s.calls_14d, 0);
  return { total, connected, sumRegistered, sumInvoked, coverage, totalCalls };
}

// ── #84 T2 Slice A/B — discovered MCP servers + tool manifests ──────────

let mcpServers = $state<McpServerRow[]>([]);
let mcpServersStatus = $state<LoadStatus>('idle');
let serverToolManifests = $state<Record<string, McpServerToolsManifest>>({});

/** Load the discovered MCP-servers list for a project (or user-scope only
 *  when no project id). Idempotent — a subsequent call returns cached rows
 *  unless `force` is set. */
async function loadMcpServers(projectId?: string, force = false): Promise<McpServerRow[]> {
  if (!force && mcpServersStatus === 'ready') return mcpServers;
  if (mcpServersStatus === 'loading') return mcpServers;
  mcpServersStatus = 'loading';
  try {
    const api = senseiApi(appState.port);
    const resp = await api.getMcpServers(projectId);
    mcpServers = resp.servers;
    mcpServersStatus = 'ready';
  } catch {
    mcpServersStatus = 'error';
  }
  return mcpServers;
}

/** Fetch a server's cached tool manifest. `refresh: true` forces a re-probe
 *  even if the cache is fresh; `probe: false` returns stale-badged cache
 *  without spawning the subprocess. */
async function loadServerTools(
  serverId: string,
  opts: { refresh?: boolean; probe?: boolean } = {},
): Promise<McpServerToolsManifest> {
  const api = senseiApi(appState.port);
  const manifest = await api.getMcpServerTools(serverId, opts);
  serverToolManifests = { ...serverToolManifests, [serverId]: manifest };
  return manifest;
}

/** Toggle a server's `enabled` flag. Fires the daemon endpoint and updates
 *  the in-memory row optimistically so the UI stays snappy. */
async function toggleMcpServer(serverId: string, enabled: boolean): Promise<void> {
  const api = senseiApi(appState.port);
  await api.setMcpServerEnabled(serverId, enabled);
  mcpServers = mcpServers.map((s) => s.id === serverId ? { ...s, enabled } : s);
}

/** Trigger a full discovery pass across $HOME + registered projects. */
async function refreshMcpServers(): Promise<{ discovered: number; pruned: number }> {
  const api = senseiApi(appState.port);
  const stats = await api.refreshMcpServers();
  // Force a reload so the store reflects the fresh scan.
  await loadMcpServers(undefined, true);
  return stats;
}

/** Force a fresh fetch of every catalog dataset. Session timelines
 *  are re-fetched lazily on next request rather than eagerly here.
 */
async function refresh(): Promise<void> {
  catalogStatus = 'idle';
  insightsStatus = 'idle';
  toolsHealthStatus = 'idle';
  sessionTimelines = {};
  sessionReplays = {};
  serverToolManifests = {};
  mcpServersStatus = 'idle';
  await Promise.all([loadCatalog(true), loadInsights(true), loadToolsHealth(true)]);
}

/**
 * Reactive view of the store. Everything reads through the getters so
 * consumers get proper $state proxying under Svelte 5 runes.
 */
export const mcp = {
  get tools() { return tools; },
  get toolStats() { return toolStats; },
  get toolSignals() { return toolSignals; },
  get toolInsights() { return toolInsights; },
  get signalSource() { return signalSource; },
  get catalogStatus() { return catalogStatus; },
  get insightsStatus() { return insightsStatus; },
  /** Instruments · Health L1 — source grid rows, connected-first. */
  get toolsHealth() { return sortedToolsHealth(); },
  get toolsHealthStatus() { return toolsHealthStatus; },
  /** Header KPIs derived from the L1 grid rows. */
  get toolsHealthKpis() { return toolsHealthKpis(); },
  get sessionTimelines() { return sessionTimelines; },
  /** #84 T2 Slice C — cached Replay responses (calls + summary) by session id. */
  get sessionReplays() { return sessionReplays; },
  /** #84 T2 Slice A — discovered MCP servers list. */
  get mcpServers() { return mcpServers; },
  get mcpServersStatus() { return mcpServersStatus; },
  /** #84 T2 Slice B — cached tool manifests by server id. */
  get serverToolManifests() { return serverToolManifests; },
  /** Look up a cached insight row by tool name (undefined if not present). */
  insightFor(toolName: string): ToolInsight | undefined {
    return toolInsights.find((i) => i.toolName === toolName);
  },
  loadCatalog,
  loadInsights,
  loadToolsHealth,
  loadSessionTimeline,
  loadSessionReplay,
  loadMcpServers,
  loadServerTools,
  toggleMcpServer,
  refreshMcpServers,
  refresh,
};
