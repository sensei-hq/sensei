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
 * Session replays are cached per session id so re-selecting a session
 * doesn't round-trip to the daemon.
 */
import { failureReason, senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type {
  ToolInsight, ToolSignal,
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
// first fetch, 'ready' once populated, 'error' if a read failed (its reason
// in the matching *Error, #288 — the fallback reads never failed, so this
// state was unreachable and a daemon failure read as empty data). The
// three main datasets each carry their own state so a signal fetch
// failing doesn't hide manifest data.
type LoadStatus = 'idle' | 'loading' | 'ready' | 'error';

let catalogStatus = $state<LoadStatus>('idle');
let insightsStatus = $state<LoadStatus>('idle');
let catalogError = $state<string | null>(null);
let insightsError = $state<string | null>(null);

let toolStats = $state<ToolStat[]>([]);
let toolSignals = $state<ToolSignal[]>([]);
let toolInsights = $state<ToolInsight[]>([]);
/** Where the current tool-signals response came from (cache vs derived). */
let signalSource = $state<'cache' | 'derived' | null>(null);

async function loadCatalog(force = false): Promise<void> {
  if (catalogStatus === 'loading') return;
  if (!force && catalogStatus === 'ready') return;
  catalogStatus = 'loading';
  const api = senseiApi(appState.port);
  const [stats, signals] = await Promise.all([api.tryGetToolUsage(), api.tryGetToolSignals()]);
  if (!stats.ok || !signals.ok) {
    const failed = [stats, signals].find((r) => !r.ok);
    catalogError = failed && !failed.ok ? failureReason(failed.error) : null;
    catalogStatus = 'error';
    return;
  }
  toolStats = stats.data.tools ?? [];
  toolSignals = signals.data.signals ?? [];
  signalSource = signals.data.source ?? null;
  catalogError = null;
  catalogStatus = 'ready';
}

async function loadInsights(force = false): Promise<void> {
  if (insightsStatus === 'loading') return;
  if (!force && insightsStatus === 'ready') return;
  insightsStatus = 'loading';
  const read = await senseiApi(appState.port).tryGetToolInsights();
  toolInsights = read.ok ? read.data.insights ?? [] : [];
  insightsError = read.ok ? null : failureReason(read.error);
  insightsStatus = read.ok ? 'ready' : 'error';
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
): Promise<{ replay: ReplayCache | null; error: string | null }> {
  if (!opts.force && sessionReplays[sessionId]) {
    return { replay: sessionReplays[sessionId], error: null };
  }
  const read = await senseiApi(appState.port).tryGetSessionReplay(sessionId, { classify: opts.classify });
  // A failed read is not cached: it is the session's error, not an empty replay (#288).
  if (!read.ok) return { replay: null, error: failureReason(read.error) };
  const cache: ReplayCache = { calls: read.data.calls, summary: read.data.summary };
  sessionReplays = { ...sessionReplays, [sessionId]: cache };
  return { replay: cache, error: null };
}

// ── Instruments · Health L1 — tool-health source grid ──────────────────

/** One row per registered source (builtin tool set or probed MCP server)
 *  from GET /api/instruments/tools-health. Drives the L1 share grid; the
 *  L2 per-tool drill still reads tool-signals / tool-insights above. */
let toolsHealth = $state<ToolHealthSource[]>([]);
let toolsHealthStatus = $state<LoadStatus>('idle');
let toolsHealthError = $state<string | null>(null);

/** Load the L1 source grid. Idempotent — a second call short-circuits once
 *  ready unless `force` is set (mirrors loadCatalog / loadInsights). */
async function loadToolsHealth(force = false): Promise<void> {
  if (toolsHealthStatus === 'loading') return;
  if (!force && toolsHealthStatus === 'ready') return;
  toolsHealthStatus = 'loading';
  const read = await senseiApi(appState.port).tryGetToolsHealth();
  toolsHealth = read.ok ? read.data.sources ?? [] : [];
  toolsHealthError = read.ok ? null : failureReason(read.error);
  toolsHealthStatus = read.ok ? 'ready' : 'error';
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
/** Why a server's tools could not be read, by server id (#288). Kept apart
 *  from the manifests so a failed read is retried on the next expand. */
let serverToolErrors = $state<Record<string, string>>({});

/** Load the discovered MCP-servers list for a workspace (or user-scope only
 *  when no workspace id). Idempotent — a subsequent call returns cached rows
 *  unless `force` is set. */
async function loadMcpServers(workspaceId?: string, force = false): Promise<McpServerRow[]> {
  if (!force && mcpServersStatus === 'ready') return mcpServers;
  if (mcpServersStatus === 'loading') return mcpServers;
  mcpServersStatus = 'loading';
  const resp = await senseiApi(appState.port).getMcpServers(workspaceId);
  if (resp.ok) {
    mcpServers = resp.data.servers;
    mcpServersStatus = 'ready';
  } else {
    // The cached rows stay; the status says they could not be refreshed.
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
): Promise<void> {
  const read = await senseiApi(appState.port).tryGetMcpServerTools(serverId, opts);
  const { [serverId]: _previous, ...others } = serverToolErrors;
  if (read.ok) {
    serverToolManifests = { ...serverToolManifests, [serverId]: read.data };
    serverToolErrors = others;
  } else {
    serverToolErrors = { ...others, [serverId]: failureReason(read.error) };
  }
}

/** Toggle a server's `enabled` flag. The row changes only once the daemon
 *  took it. Returns `null` on success, else the failure in words. */
async function toggleMcpServer(serverId: string, enabled: boolean): Promise<string | null> {
  const res = await senseiApi(appState.port).setMcpServerEnabled(serverId, enabled);
  if (!res.ok) return `The daemon did not change the server (${res.error.status} ${res.error.message})`;
  mcpServers = mcpServers.map((s) => s.id === serverId ? { ...s, enabled } : s);
  return null;
}

/** Register a server by hand (#343), then reload the list. Returns `null` on
 *  success, else the daemon's reason in words. */
async function registerMcpServer(server: {
  name: string; command: string; args: string[]; env: Record<string, string>;
}): Promise<string | null> {
  const res = await senseiApi(appState.port).tryRegisterMcpServer(server);
  if (!res.ok) return `The server was not added: ${failureReason(res.error)}`;
  await loadMcpServers(undefined, true);
  return null;
}

/** Remove a hand-registered server; the row goes only once the daemon removed it. */
async function removeMcpServer(serverId: string): Promise<string | null> {
  const res = await senseiApi(appState.port).tryRemoveMcpServer(serverId);
  if (!res.ok) return `The server was not removed (${res.error.status} ${res.error.message})`;
  mcpServers = mcpServers.filter((s) => s.id !== serverId);
  return null;
}

/** Trigger a full discovery pass across $HOME + registered workspaces, then
 *  reload the list. Returns `null` on success, else the failure in words. */
async function refreshMcpServers(): Promise<string | null> {
  const res = await senseiApi(appState.port).refreshMcpServers();
  if (!res.ok) return `The discovery scan failed (${res.error.status} ${res.error.message})`;
  await loadMcpServers(undefined, true);
  return null;
}

/** Force a fresh fetch of every catalog dataset. Session replays and server
 *  tools are re-fetched lazily on next request rather than eagerly here.
 */
async function refresh(): Promise<void> {
  catalogStatus = 'idle';
  insightsStatus = 'idle';
  toolsHealthStatus = 'idle';
  sessionReplays = {};
  serverToolManifests = {};
  serverToolErrors = {};
  mcpServersStatus = 'idle';
  await Promise.all([loadCatalog(true), loadInsights(true), loadToolsHealth(true)]);
}

/**
 * Reactive view of the store. Everything reads through the getters so
 * consumers get proper $state proxying under Svelte 5 runes.
 */
export const mcp = {
  get toolStats() { return toolStats; },
  get toolSignals() { return toolSignals; },
  get toolInsights() { return toolInsights; },
  get signalSource() { return signalSource; },
  get catalogStatus() { return catalogStatus; },
  get insightsStatus() { return insightsStatus; },
  get catalogError() { return catalogError; },
  get insightsError() { return insightsError; },
  /** Instruments · Health L1 — source grid rows, connected-first. */
  get toolsHealth() { return sortedToolsHealth(); },
  get toolsHealthStatus() { return toolsHealthStatus; },
  get toolsHealthError() { return toolsHealthError; },
  /** Header KPIs derived from the L1 grid rows. */
  get toolsHealthKpis() { return toolsHealthKpis(); },
  /** #84 T2 Slice C — cached Replay responses (calls + summary) by session id. */
  get sessionReplays() { return sessionReplays; },
  /** #84 T2 Slice A — discovered MCP servers list. */
  get mcpServers() { return mcpServers; },
  get mcpServersStatus() { return mcpServersStatus; },
  /** #84 T2 Slice B — cached tool manifests by server id. */
  get serverToolManifests() { return serverToolManifests; },
  get serverToolErrors() { return serverToolErrors; },
  /** Look up a cached insight row by tool name (undefined if not present). */
  insightFor(toolName: string): ToolInsight | undefined {
    return toolInsights.find((i) => i.toolName === toolName);
  },
  loadCatalog,
  loadInsights,
  loadToolsHealth,
  loadSessionReplay,
  loadMcpServers,
  loadServerTools,
  toggleMcpServer,
  refreshMcpServers,
  registerMcpServer,
  removeMcpServer,
  refresh,
};
