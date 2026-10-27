import type {
  ServerWorkspace,
  GraphSymbolNode, GraphCallEdge,
  WorkspaceGraphResponse, SolutionAnalysis,
  FunctionMatch, CommunityInfo,
  LibEntry, LibCapability, LibPage, SessionData, SessionsDigest,
  WorkspaceMemory, DriftItem, PatternEntry, Recommendation,
  CallFlowModule, CallFlowCall,
  WorkspaceListItem,
  KnowledgeSource, NewKnowledgeSourceBody, SyncStats,
  DojoMembership, DojoBindingSuggestion, DojoUpgradesResponse, CollectivePreferences,
  ShareReviewResponse, PublishBatchOutcome,
  McpToolManifest, MemoryShareBatch, ImpactVerdictEntry,
  WorkspaceMcpToolStat, ToolSignal, WorkspaceService, ToolInsight, ToolsHealth,
  SessionReplayResponse, McpServerRow, McpServerToolsManifest,
  ObservatoryToday, ObservatoryFtr, WorkspaceOverview,
  InsightsBoard, LogRow, ScheduledTask,
  MetricStatusResponse, MetricStatusSummary, MetricActivationOutcome,
  SyncStateResponse,
  IntakeGuide, PlaybookRecommendation,
  ProvisionModel, ProvisionPhase,
  StructureLevel, StructurePayload,
  LayeringLevel, LayeringPayload,
  WorldGroupBy, WorldPayload,
  NeighbourhoodPayload,
  SchemaPayload,
  RootRemovalDecision, RootRemovalResult, RootRepository, PruneResult,
  TranscriptSourceConsent,
  TrackerConnection, TrackerLevel, TrackerSyncCounts,
  SprintUsageRow, TrackerTaskRow, TrackerTaskDetail, UnlinkedSession, BacklogUsage,
} from './types.js';
import type {
  MemoryListResponse, MemoryDetail, ContextResponse,
  ProposalCreateBody, MemoryCreateBody, OutcomeBody, OutcomesBatchResponse,
  DaemonWatchRoot, DaemonWorkspace,
} from './setup/contracts.js';
import type {
  ConsolidatedRuleset, ConsolidateResult,
} from '../routes/(observatory)/consolidation/consolidation-view.js';
import type {
  WorkspaceMetricRow, RegistryMetric, MetricSeriesPoint, MetricsNarrative,
  DaySessions, DrilldownSession, ToolUsage,
} from './metrics/metric-view.js';
import type { WorkspaceHealth } from './metrics/health-radar.js';
import type { WorkspaceImpactWire } from './impact.js';

export type { DaySessions, DrilldownSession, ToolUsage };

import type { PersonaWire } from './personas.svelte.js';

/** `body` is the daemon's parsed error body, when a caller needs more than the
 *  code in `message` (e.g. which workspace already owns a board). */
export type ApiError =
  | { status: number; message: string; body?: Record<string, unknown> }
  | { status: 0; message: string };

export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

/** What accepting a recommendation would write (`…/recommendations/{id}/preview`). */
export interface RecommendationPreview {
  materializable: boolean; kind?: string; action_type?: string; reason?: string;
  title?: string; body?: string; impact?: string | null; gov_scope?: string; enforcement?: string;
  // file-class (write_skill/create_agent) preview fields:
  name?: string; path?: string; content?: string; consent_required?: boolean;
}

/** Why a read failed, for the screen to say: the status text, else the status. */
export const failureReason = (error: ApiError): string => error.message || `HTTP ${error.status}`;

/** A failure in words for a screen: an unreachable daemon is said as such,
 *  else the status the daemon returned. */
export const daemonFailure = (error: ApiError): string =>
  error.status === 0
    ? `Could not reach the daemon — ${error.message}`
    : `The daemon returned ${error.status} ${error.message}`;

/** A read's data, or — for a caller that cannot go on without it — a throw
 *  naming what could not be read (#288). */
export function dataOrThrow<T>(read: ApiResult<T>, what: string): T {
  if (!read.ok) throw new Error(`reading the ${what} failed: ${failureReason(read.error)}`);
  return read.data;
}

/** A read as a loader hands it to its page: the data, or why it could not be
 *  read — never a default standing in for the failure (#288). */
export function readOutcome<T>(r: ApiResult<T>): { data: T | null; error: string | null } {
  return r.ok ? { data: r.data, error: null } : { data: null, error: failureReason(r.error) };
}

/** The daemon's origin for a given port. The single source of truth for the
 *  loopback base URL, shared by `senseiApi` and any non-fetch consumer (e.g.
 *  an `<img src>` that streams bytes from the daemon, like a workspace icon). */
export const apiBase = (port: number): string => `http://127.0.0.1:${port}`;

/** Create a typed API client for the sensei Rust daemon. */
/** `?persona=…` when one is named; nothing, so the daemon resolves the default
 *  identity (#296), when not. */
const personaQuery = (persona?: string) => (persona ? `?persona=${encodeURIComponent(persona)}` : '');

export function senseiApi(port: number) {
  const base = apiBase(port);

  // Every read is `tryGet` and every write a try- helper: the daemon's answer or
  // an ApiResult error, never a default standing in for a failure (#288, #304).
  async function tryPut(path: string, body: unknown): Promise<ApiResult<void>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) return { ok: true, data: undefined };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  async function tryDelete(path: string): Promise<ApiResult<void>> {
    try {
      const res = await fetch(`${base}${path}`, { method: 'DELETE' });
      if (res.ok) return { ok: true, data: undefined };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  /** Error-propagating DELETE that returns the parsed body — for a removal
   *  that reports what it removed. */
  async function tryDeleteJson<T>(path: string): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, { method: 'DELETE' });
      if (res.ok) return { ok: true, data: await res.json() as T };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  async function tryGet<T>(path: string): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`);
      if (res.ok) return { ok: true, data: await res.json() as T };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  async function tryPost<T>(path: string, body: unknown): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) return { ok: true, data: await res.json() as T };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  // Error-propagating request that returns the parsed response body. On a
  // non-ok it surfaces the daemon's `{ "error": "..." }` code as the message
  // when present (else the status text) and keeps the whole parsed body beside
  // it, for refusals that carry more than a code. A 204 resolves `undefined`.
  async function tryJson<T>(method: string, path: string, body?: unknown): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method,
        ...(body === undefined
          ? {}
          : { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),
      });
      if (res.status === 204) return { ok: true, data: undefined as T };
      if (res.ok) return { ok: true, data: await res.json() as T };
      try {
        const j = await res.json() as Record<string, unknown> | null;
        if (j && typeof j === 'object') {
          const code = typeof j.error === 'string' && j.error ? j.error : res.statusText;
          return { ok: false, error: { status: res.status, message: code, body: j } };
        }
      } catch { /* non-JSON error body — keep the status text */ }
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  // The whole-object full-replace endpoints (e.g. collective preferences) echo
  // the saved object back and the caller adopts it.
  const tryPutJson = <T>(path: string, body: unknown) => tryJson<T>('PUT', path, body);

  // On a refusal the endpoint's own text is the useful part (a dōjō 403/404
  // travels through the daemon's 502 as a message), and collapsing it to
  // "Bad Gateway" would strip the reason.
  const tryPatchJson = <T>(path: string, body: unknown) => tryJson<T>('PATCH', path, body);

  return {
    // ── Auth ──────────────────────────────────────────────────────────────
    // Result-based on purpose. A failed read of the credential's standing must
    // NOT collapse into a fallback body: `{}` is indistinguishable from a
    // daemon reporting a shape we do not understand, and either way the caller
    // would be told something about a credential we never asked about.
    // No persona named → the daemon resolves the DEFAULT identity (#296), the
    // one owning the global git email; never the literal `default` slot.
    tryGetAuthStatus: (persona?: string) =>
      tryGet<Record<string, unknown>>(`/api/auth/status${personaQuery(persona)}`),

    /** Every identity and what each one needs. Result-based: a failed read must
     *  not become `[]`, which reads as "you have no identities". */
    tryGetPersonas: () =>
      tryGet<{ personas: PersonaWire[]; default?: string | null; defaultError?: string | null }>('/api/auth/personas'),

    /** Begin the PKCE flow. The daemon returns a URL rather than opening one —
     *  it may be headless — so the caller owns the browser.
     *
     *  `expectedLogin` is passed through to GitHub so it preselects the right
     *  account. Without it a browser already signed in to another account
     *  quietly re-links THAT one — a success as the wrong person. */
    tryStartSignIn: (persona?: string, expectedLogin: string | null = null) =>
      tryPost<{ authorizeUrl?: string }>(
        `/api/auth/signin${personaQuery(persona)}` +
          (expectedLogin ? `${persona ? '&' : '?'}github_login=${encodeURIComponent(expectedLogin)}` : ''),
        {},
      ),

    /** Let the daemon read a persona's stored session, with the Keychain's
     *  dialog allowed — the user clicked, so it has someone to answer it (#283). */
    tryApproveKeychain: (persona: string) =>
      tryPost<{ ok: boolean; error?: string }>(
        `/api/auth/keychain/approve?persona=${encodeURIComponent(persona)}`,
        {},
      ),

    // ── Health ────────────────────────────────────────────────────────────
    tryGetHealth: () => tryGet<Record<string, unknown>>('/health'),

    /** Every watch root, paused ones included (#247). Result-based: the
     *  settings screen must tell "you have no roots" from "the daemon is down". */
    tryGetScanRoots: () => tryGet<DaemonWatchRoot[]>('/api/scan/roots'),


    // ── Repos (individual git repos) ────────────────────────────────────

    // Every write is a try-write: done, or why not (#304).
    tryRegisterRepo: (repoId: string, name: string, path: string) =>
      tryPost<{ ok: boolean }>('/api/repos', { repoId, name, path }),

    tryUpdateRepo: (repoId: string, patch: Partial<ServerWorkspace>) =>
      tryPut(`/api/repos/${enc(repoId)}`, patch),

    tryDeleteRepo: (repoId: string) => tryDelete(`/api/repos/${enc(repoId)}`),

    tryExcludeRepo: (repoId: string) =>
      tryPost<{ ok: boolean }>(`/api/repos/${enc(repoId)}/exclude`, {}),

    tryAddRepoTag: (repoId: string, tag: string) =>
      tryPost<{ ok: boolean }>(`/api/repos/${enc(repoId)}/tags`, { tag }),

    tryRemoveRepoTag: (repoId: string, tag: string) =>
      tryDelete(`/api/repos/${enc(repoId)}/tags/${enc(tag)}`),

    // ── Workspaces (groups of 1+ repos) ───────────────────────────────────

    tryCreateWorkspace: (workspace: object) =>
      tryPost<{ ok: boolean; id?: string }>('/api/workspaces', workspace),

    /** A workspace edit — saved, or why not (#304). The About-tab metadata
     *  form drives a saveStatus (idle/saving/saved/error) lifecycle off the
     *  Result, mirroring trySetConfig on the settings form. The daemon's
     *  PUT /api/workspaces/{id} does a partial (COALESCE) update of the editable
     *  identity subset (name/description/client/goal/maturity/…). */
    tryUpdateWorkspace: (id: string, patch: object): Promise<ApiResult<void>> =>
      tryPut(`/api/workspaces/${enc(id)}`, patch),

    tryDeleteWorkspace: (id: string) => tryDelete(`/api/workspaces/${enc(id)}`),

    /** Error-propagating reads/writes for Settings · Workspaces (#271): a failed
     *  load is an error state, never an empty list, and a role save that fails
     *  is rolled back and reported rather than silently kept on screen. */
    // One read of the list, typed as the wire carries it: the settings and
    // setup view (`DaemonWorkspace`) and the workspaces-index view (`WorkspaceListItem`).
    tryListWorkspaces: () => tryGet<Array<DaemonWorkspace & WorkspaceListItem>>('/api/workspaces'),
    /** Update a single folder. Currently only `role` is honored daemon-side. */
    tryUpdateFolder: (id: string, patch: { role?: string | null }) =>
      tryPut(`/api/folders/${enc(id)}`, patch),

    tryAddWorkspaceRepo: (workspaceId: string, repo: { repoId: string; role?: string }) =>
      tryPost<{ ok: boolean }>(`/api/workspaces/${enc(workspaceId)}/repos`, repo),

    tryRemoveWorkspaceRepo: (workspaceId: string, repoId: string) =>
      tryDelete(`/api/workspaces/${enc(workspaceId)}/repos/${enc(repoId)}`),

    tryAnalyzeWorkspace: (id: string) =>
      tryPost<SolutionAnalysis>(`/api/workspaces/${enc(id)}/analyze`, {}),

    // ── Workspace detail (new multi-window endpoints) ───────────────────
    /** A workspace's FTR headline (14d / prior / trend / 7d). */
    tryGetWorkspaceFtr: (id: string) =>
      tryGet<{ ftr14d: number | null; ftr14dPrev: number | null; ftrTrend: number[]; sessions7d: number }>(
        `/api/workspaces/${enc(id)}/ftr`,
      ),

    // ── Workspace metrics (registry-driven) ─────────────────────────────
    // Error-propagating (tryGet): a fetch failure must be distinguishable from a
    // workspace with no computed metrics yet, so the screen shows an error state
    // rather than an empty grid that hides a broken daemon (no-fabrication).
    getWorkspaceMetrics: (id: string) =>
      tryGet<{ metrics: WorkspaceMetricRow[]; count: number; narrative?: MetricsNarrative | null }>(
        `/api/workspaces/${enc(id)}/metrics`,
      ),

    // The catalog — the only surface that carries each metric's `family`, joined
    // client-side to group the per-workspace values into sections.
    getMetricsRegistry: () =>
      tryGet<{ metrics: RegistryMetric[]; count: number }>(`/api/metrics/registry`),

    // Composite health + every included metric's 0-5 rating and weight — the
    // radar's spokes and the score, from one row so they always agree (spec I5).
    // 404 when the daemon rated nothing for this workspace: honest-empty, and the
    // Result lets the caller tell that apart from a fetch failure.
    getWorkspaceHealth: (id: string) => tryGet<WorkspaceHealth>(`/api/workspaces/${enc(id)}/health`),
    /** Which metrics move together, across every workspace. Portfolio-wide on
     *  purpose: per workspace the paired-day count rarely clears the daemon's n>=20
     *  gate, so a workspace-scoped call is usually empty. */
    getMetricCorrelations: () =>
        tryGet<{
            cells: number;
            min_pairs: number;
            min_rho: number;
            correlations: Array<{ a: string; b: string; rho: number; n: number }>;
        }>(`/api/metrics/correlations`),

    // Per-metric time series for the card sparklines / trend view.
    getWorkspaceMetricSeries: (
      id: string,
      key: string,
      grain: 'daily' | 'weekly' | 'monthly' | 'quarterly' = 'daily',
    ) =>
      tryGet<{
        metric: string;
        grain: string;
        // The metric's `formula` (registry "how it's calculated") — travels with the
        // series so the detail screen renders it. Honest-null for an unknown key.
        formula: string | null;
        series: MetricSeriesPoint[];
        count: number;
      }>(`/api/workspaces/${enc(id)}/metrics/${enc(key)}?grain=${grain}`),

    // The measurable sessions behind ONE daily datapoint (the datapoint→sessions
    // drill-down). Error-propagating (tryGet): a fetch failure — including a 404
    // on a daemon that predates the endpoint — surfaces as `{ ok: false, error }`
    // so the drill-down renders an explicit "not available"/error STATE, never a
    // fabricated session list. `day` is `YYYY-MM-DD`.
    getWorkspaceMetricDaySessions: (id: string, key: string, day: string) =>
      tryGet<DaySessions>(
        `/api/workspaces/${enc(id)}/metrics/${enc(key)}/sessions?day=${enc(day)}`,
      ),

    // Per-tool usage breakdown (which tools the ACPs invoked) — the tool-usage
    // bubble view. Honest-empty ({tools:[]}) on a fetch error, never fabricated.
    /** Per-tool usage for a workspace (#288: a failure is an error, not "no tools"). */
    tryGetWorkspaceTools: (id: string) => tryGet<{ tools: ToolUsage[]; count: number }>(`/api/workspaces/${enc(id)}/tools`),

    tryGetWorkspaceRepos: (id: string) =>
      tryGet<{ repos: Array<{ id: string; name: string; path: string; kind: string; role?: string }> }>(
        `/api/workspaces/${enc(id)}/repos`,
      ),

    // Error-propagating (mockup-drift-audit F8): a fetch failure must be
    // distinguishable from a workspace that genuinely has no libraries. tryGet
    // surfaces {ok:false,error} so the screen shows error-with-Retry instead of
    // an empty list that hides a broken daemon.
    tryGetWorkspaceLibraries: (id: string) =>
      tryGet<{ libraries: Array<{
        id: string; name: string; ecosystem: string;
        scope: 'global' | 'workspace'; enabled: boolean;
        description?: string | null;
        hasDocs?: boolean; pageCount?: number;
        localSource?: string | null;
      }> }>(
        `/api/workspaces/${enc(id)}/libraries`,
      ),

    // T1a version-conflict view — surfaces libraries pinned to multiple
    // versions across the workspace's folders. One row per (workspace, library)
    // pair with the distinct versions + folders where each was seen.
    // Error-propagating (F8) — see tryGetWorkspaceLibraries.
    tryGetWorkspaceLibraryVersionConflicts: (id: string) =>
      tryGet<{ conflicts: Array<{ library_id: string; library_name: string; ecosystem: string; versions: string[]; folders: string[] }> }>(
        `/api/workspaces/${enc(id)}/library-version-conflicts`,
      ),

    tryGetWorkspaceInstruments: (id: string) =>
      tryGet<{ tools: Array<{ id: string; name: string; kind: string; scope: 'global' | 'workspace'; enabled: boolean }> }>(
        `/api/workspaces/${enc(id)}/instruments`,
      ),

    // Per-workspace MCP tool aggregation — calls / errors / avg duration / FTR
    // scoped to the workspace. Returns EVERY manifest tool with usage overlaid
    // (zero-call rows included) so the UI can render the full catalogue.
    tryGetWorkspaceMcpToolStats: (id: string) =>
      tryGet<{ tools: WorkspaceMcpToolStat[] }>(`/api/workspaces/${enc(id)}/mcp-tool-stats`),

    // Services (MCP servers, inference providers) with per-workspace scope
    // resolved (scoped > global > default true).
    tryListWorkspaceServices: (id: string) =>
      tryGet<{ services: WorkspaceService[] }>(`/api/workspaces/${enc(id)}/services`),

    /** Turn a service on or off for one workspace — done, or why not (#304). */
    trySetWorkspaceServiceScope: (id: string, serviceId: string, enabled: boolean) =>
      tryPut(`/api/workspaces/${enc(id)}/services/${enc(serviceId)}/scope`, { enabled }),

    tryGetWorkspaceMemories: (id: string) =>
      tryGet<{ active: WorkspaceMemory[]; total: number }>(`/api/workspaces/${enc(id)}/memories`),

    // Memory share batches — the proposal / review / verdict lifecycle for
    // grouping memories before federating them out to a hive-mind.
    tryListMemoryShareBatches: (id: string, status?: string) =>
      tryGet<{ batches: MemoryShareBatch[] }>(
        `/api/workspaces/${enc(id)}/memory-batches${status ? `?status=${enc(status)}` : ''}`,
      ),

    /** Propose a share batch — its id, or why it was not created. */
    tryCreateMemoryShareBatch: (id: string, memoryIds: string[], note?: string) =>
      tryPost<{ id: string }>(`/api/workspaces/${enc(id)}/memory-batches`, { memory_ids: memoryIds, note }),

    /** Approve, reject or withdraw a share batch — or why the daemon refused. */
    tryDecideMemoryShareBatch: (id: string, batchId: string, status: 'approved' | 'rejected' | 'withdrawn', note?: string) =>
      tryPut(`/api/workspaces/${enc(id)}/memory-batches/${enc(batchId)}`, { status, note }),

    // Impact verdicts (manual log) — user-logged retrospectives about
    // shipped changes. Independent of the automatic recommendation
    // verdicts on /api/workspaces/{id}/impact.
    /** The user-logged impact log (T3 Slice 3), optionally one verdict. */
    tryListImpactVerdicts: (id: string, verdict?: string) =>
      tryGet<{ verdicts: ImpactVerdictEntry[] }>(
        `/api/workspaces/${enc(id)}/impact-verdicts${verdict ? `?verdict=${enc(verdict)}` : ''}`,
      ),

    /** A workspace's measured recommendations (#288: a failure is an error, not []). */
    tryGetWorkspaceImpact: (id: string) => tryGet<WorkspaceImpactWire[]>(`/api/workspaces/${enc(id)}/impact`),

    /** Run the doc-drift detector now; resolves to what the scan found. */
    tryScanWorkspaceDrift: (id: string) =>
      tryPost<{ scannedDocs: number; newBroken: number; resolved: number }>(`/api/workspaces/${enc(id)}/drift/scan`, {}),

    /** A workspace's doc-drift items with their counts (#288: a failure is an error). */
    tryGetWorkspaceDrift: (id: string) =>
      tryGet<{ items: DriftItem[]; total: number; drifted: number; broken: number; indexedDocs?: number }>(`/api/workspaces/${enc(id)}/drift`),

    /** Log an impact by hand — its id, or why it was not logged. */
    tryCreateImpactVerdict: (id: string, title: string, note?: string, sessionId?: string) =>
      tryPost<{ id: string }>(`/api/workspaces/${enc(id)}/impact-verdicts`, { title, note, session_id: sessionId }),

    /** Record a logged impact's outcome — or why the daemon refused. */
    tryDecideImpactVerdict: (id: string, verdictId: string, verdict: 'success' | 'mixed' | 'failure', note?: string) =>
      tryPut(`/api/workspaces/${enc(id)}/impact-verdicts/${enc(verdictId)}`, { verdict, note }),

    // Measured impact reports — the analyzer's before/after FTR + MOE-style
    // consensus for recommendations that carry a reasoning trace or a
    // non-pending verdict. Distinct from the manual `listImpactVerdicts`
    // retrospectives above.
    /** #205 — the Structure diagram payload.
     *
     *  Result-based deliberately. This screen must tell "no edges here" apart
     *  from "the daemon failed", and the fallback-returning `get` collapses
     *  both into an empty graph — which is the one thing the spec names as a
     *  wrong gate, being indistinguishable from an unindexed workspace. */
    tryGetWorkspaceStructure: (id: string, level: StructureLevel, kinds: string[]) =>
      tryGet<StructurePayload>(
        `/api/workspaces/${enc(id)}/diagrams/structure?level=${enc(level)}&kinds=${enc(kinds.join(','))}`
      ),

    /** #232 — the Layers/Cycles payload, one request serving both screens.
     *
     *  ONE CALL, TWO SCREENS. The layering, the cycles and the coverage all
     *  come out of a single `analyse` over one read of `structure_edges`, and
     *  that read is the whole cost (1.5 s to 74 s per workspace). Splitting it
     *  into a layering endpoint and a cycles endpoint would pay it twice for
     *  answers that have to agree.
     *
     *  Result-based for the reason the Structure one is: this screen must tell
     *  "no cycles here" apart from "the daemon failed", and an empty canvas
     *  reads as a clean architecture. */
    tryGetWorkspaceLayering: (id: string, level: LayeringLevel, kinds: string[]) =>
      tryGet<LayeringPayload>(
        `/api/workspaces/${enc(id)}/diagrams/layering?level=${enc(level)}&kinds=${enc(kinds.join(','))}`
      ),

    /** #219 — the World payload: every indexed declaration, nested.
     *
     *  CROSS-WORKSPACE. The workspace in the path names the view and does not scope
     *  it — the picture is all indexed code, with workspace as its outermost ring.
     *
     *  Result-based like its siblings: this screen must tell "nothing indexed"
     *  apart from "the daemon failed", and on a picture whose whole subject is
     *  how much there is, an empty canvas is the worse lie. */
    tryGetWorkspaceWorld: (id: string, groupBy: WorldGroupBy) =>
      tryGet<WorldPayload>(
        `/api/workspaces/${enc(id)}/diagrams/world?groupBy=${enc(groupBy)}`
      ),

    /** #220 — one symbol, its callers and callees, `depth` rings out.
     *  `focus` is a node id; a missing one is a 400, never a default symbol. */
    tryGetWorkspaceNeighbourhood: (id: string, focus: string, depth: number) =>
      tryGet<NeighbourhoodPayload>(
        `/api/workspaces/${enc(id)}/diagrams/neighbourhood?focus=${enc(focus)}&depth=${depth}`
      ),

    /** #221 — the workspace's database structure: tables, keys, views and
     *  routines, and what depends on what (dbd `SchemaModel` v2's ER half).
     *
     *  Result-based like its siblings: "no database described here" and "the
     *  daemon failed" must stay apart, and an empty canvas reads as the first. */
    tryGetWorkspaceSchema: (id: string) =>
      tryGet<SchemaPayload>(`/api/workspaces/${enc(id)}/diagrams/schema`),

    /** A failed read is an error, never "no patterns" (#85). */
    tryGetWorkspacePatterns: (id: string) =>
      tryGet<{ followed: PatternEntry[]; antiPatterns: PatternEntry[] }>(`/api/workspaces/${enc(id)}/patterns`),

    // Result-based variant: a recommendations-only fetch failure (the /metrics
    // leg can succeed independently) must SURFACE, not collapse to [] — otherwise
    // a 500/timeout is indistinguishable from a genuinely-empty list on a
    // governance-facing screen.
    tryGetWorkspaceRecommendations: (id: string, status?: string) =>
      tryGet<Recommendation[]>(
        `/api/workspaces/${enc(id)}/recommendations${status ? `?status=${enc(status)}` : ''}`,
      ),

    // P-A: rule-class accept that MATERIALIZES a governance rule (spec 2026-08-20).
    // `preview` renders what would be written (no write); `materialize` accepts +
    // writes the rule at the chosen scope/enforcement (defaults: workspace / recommended).
    tryPreviewRecommendation: (id: string, recId: string) =>
      tryGet<RecommendationPreview>(`/api/workspaces/${enc(id)}/recommendations/${enc(recId)}/preview`),
    tryMaterializeRecommendation: (
      id: string, recId: string,
      opts: { gov_scope?: string; namespace_id?: string; enforcement?: string; title?: string; body?: string },
    ) =>
      tryPost<{ ok: boolean; materialized?: unknown }>(
        `/api/workspaces/${enc(id)}/recommendations/${enc(recId)}/materialize`, opts,
      ),
    // The accept/reject flow that gives MeasureVerdicts work to measure. Each
    // resolves { ok } on success, or an error — 409 CONFLICT when the rec was
    // already decided.
    tryAcceptWorkspaceRecommendation: (id: string, recId: string) =>
      tryPost<{ ok: boolean }>(`/api/workspaces/${enc(id)}/recommendations/${enc(recId)}/accept`, {}),
    tryRejectWorkspaceRecommendation: (id: string, recId: string) =>
      tryPost<{ ok: boolean }>(`/api/workspaces/${enc(id)}/recommendations/${enc(recId)}/reject`, {}),
    // ── Observatory · Insights (Slot 5) ─────────────────────────────────
    // Server-side triage aggregator: bundles recs, memories, patterns and
    // corrections pre-bucketed into Now / Soon / Settled. `workspace` scopes
    // all three columns to one workspace (name-or-uuid). Fallback is the empty
    // board so a daemon hiccup renders the quiet-state, never a broken screen.
    /** The cross-workspace triage board, or one workspace's (#288: a failure is
     *  an error, never the all-quiet empty board). */
    tryGetInsights: (workspace?: string) =>
      tryGet<InsightsBoard>(`/api/insights${workspace ? `?workspace=${enc(workspace)}` : ''}`),

    // ── Front door · Intake ─────────────────────────────────────────────
    // The guide (frame + axis prompts + catalog) for the intake screen. A
    // failure is an error the screen shows, not an empty guide (#288).
    tryGetIntakeGuide: () => tryGet<IntakeGuide>('/api/playbook/guide'),

    // Classify + recommend a playbook. `{ chunk, preview: true }` previews
    // (no row written); `{ lifecycle, intent, risk, confirm: true }` records
    // the confirmed run. tryPost so the form can surface errors.
    recommendPlaybook: (body: Record<string, unknown>) =>
      tryPost<PlaybookRecommendation>('/api/playbook/recommend', body),

    // ── Workspace window · Overview (Slot 4) ──────────────────────────────
    // Server-assembled landing pane: header + top rec + stats + recent
    // sessions in one call. A failed read is an error the page shows, never an
    // all-quiet default that reads as a calm workspace (#85).
    tryGetWorkspaceOverview: (id: string) =>
      tryGet<WorkspaceOverview>(`/api/workspaces/${enc(id)}/overview`),

    // ── Observatory · Today (home screen) ───────────────────────────────
    // The daemon assembles the whole screen — greeting, maturity gate, hero
    // koan, insights, adopted lane, recent sessions. The screen renders it;
    // it does not decide early-vs-mature. A failed read is an error the screen
    // shows, never the early "still listening" state (#288).
    tryGetObservatoryToday: () => tryGet<ObservatoryToday>('/api/observatory/today'),

    /** Holistic FTR headline (14d / prior / trend / 7d) → Today header chip. */
    tryGetObservatoryFtr: () => tryGet<ObservatoryFtr>('/api/observatory/ftr'),

    // ── Observatory chart data ──────────────────────────────────────────

    /** Where edits concentrated over the last `days` (#85). */
    tryGetWorkspaceHotspots: (id: string, days = 7) =>
      tryGet<{ hotspots: Array<{ folder: string; file_path: string; edit_count: number; correction_count: number }> }>(
        `/api/workspaces/${enc(id)}/hotspots?days=${days}`,
      ),

    tryGetWorkspaceQualitySignals: (id: string) =>
      tryGet<{ ftr_7d: number; pattern_compliance: number | null; open_drift_count: number; test_pass_rate: number | null }>(
        `/api/workspaces/${enc(id)}/quality-signals`,
      ),

    tryGetToolUsage: () =>
      tryGet<{ tools: Array<{ tool_name: string; call_count: number; error_count: number; avg_duration_ms: number | null; last_used_at: string }> }>(
        '/api/observatory/tool-usage',
      ),

    tryGetToolSignals: () =>
      tryGet<{ signals: ToolSignal[]; source?: 'cache' | 'derived' }>('/api/observatory/tool-signals'),

    // Cached per-tool snapshots (T2 Slice D) — full metrics + optional
    // signal card per tool. Populated by the AggregateToolInsights task.
    tryGetToolInsights: () => tryGet<{ insights: ToolInsight[] }>('/api/observatory/tool-insights'),

    // L1 source grid for Instruments · Health — one card per registered
    // source (builtin tool set or probed MCP server) with the share of
    // registered tools actually invoked in the last 14 days.
    tryGetToolsHealth: () => tryGet<ToolsHealth>('/api/instruments/tools-health'),

    /** Where a library is used (#288: a failure is an error, not "unused"). */
    /** What a library provides — read by its name or one of its packages' (#341). */
    tryGetLibrarySkills: (name: string) =>
      tryGet<LibCapability[]>(`/api/libs/${enc(name)}/skills`),
    tryGetLibraryAgents: (name: string) =>
      tryGet<LibCapability[]>(`/api/libs/${enc(name)}/agents`),
    tryGetLibraryDocs: (name: string) =>
      tryGet<LibPage[]>(`/api/libs/${enc(name)}/docs`),
    tryGetLibraryUsage: (id: string) =>
      tryGet<{ usage: Array<{ library_name: string; folder: string; version_used: string | null; import_count: number }> }>(
        `/api/libs/${enc(id)}/usage`,
      ),

    // ── Indexing ─────────────────────────────────────────────────────────
    tryIndexRepo: (repoId: string, repoPath: string, force = false) =>
      tryPost<{ ok: boolean; queued: boolean; taskId: number }>('/api/index', { repoId, repoPath, force }),

    /** The index task queue (#288: a failure is an error — it used to read as an
     *  idle queue, which the scan's done poller took for "finished"). */
    tryGetIndexStatus: () =>
      tryGet<{ queue: { pending: number; blocked: number; running: number; completed: number }; repos: Record<string, { total: number; pending: number; running: number }> }>(
        '/api/index/status',
      ),

    /** Subscribe to index progress via SSE. Returns an EventSource. */
    subscribeIndexProgress: (): EventSource =>
      new EventSource(`${base}/api/index/progress`),

    // ── Graph ────────────────────────────────────────────────────────────
    /** Functions and methods whose name or signature contains `q`.
     *
     *  Result-based, unlike the `get(…, [])` it replaces: a picker that shows
     *  "no matches" when the daemon is down sends the reader looking for a
     *  symbol that exists. */
    trySearchFunctions: (repoId: string, q: string) =>
      tryGet<FunctionMatch[]>(`/api/graph/functions?repoId=${enc(repoId)}&q=${enc(q)}`),

    // Atlas (#288): each read is the daemon's answer or why it failed — never
    // an empty graph, zero calls or a 0-repo roll-up standing in for a failure.
    tryGetGraphNodes: (repoId: string) =>
      tryGet<{ nodes: GraphSymbolNode[]; edges: GraphCallEdge[] }>(`/api/graph/nodes?repoId=${enc(repoId)}`),

    tryGetCommunities: (repoId: string) =>
      tryGet<CommunityInfo[]>(`/api/graph/communities/info?repoId=${enc(repoId)}`),

    /** Refresh the repo's clustering, so the communities read after it is current. */
    tryDetectCommunities: (repoId: string) =>
      tryPost<{ ok: boolean; communities: number }>('/api/graph/communities', { repoId }),

    tryGetCallFlow: (repoId: string) =>
      tryGet<{ modules: CallFlowModule[]; calls: CallFlowCall[]; moduleCount: number; exportCount: number; callCount: number }>(
        `/api/graph/call-flow?repoId=${enc(repoId)}`,
      ),

    /** The workspace's solution roll-up across its repos. */
    tryGetWorkspaceGraph: (id: string) => tryGet<WorkspaceGraphResponse>(`/api/workspaces/${enc(id)}/graph`),

    // ── Libraries ────────────────────────────────────────────────────────
    /** Every detected library (#288: a failure is an error, not an empty index). */
    tryGetLibs: (params?: { repoId?: string; workspaceId?: string; shared?: boolean }) => {
      const qs = new URLSearchParams();
      if (params?.repoId) qs.set('repoId', params.repoId);
      if (params?.workspaceId) qs.set('workspaceId', params.workspaceId);
      if (params?.shared) qs.set('shared', 'true');
      return tryGet<{ total: number; libs: LibEntry[] }>(`/api/libs?${qs}`);
    },

    tryIndexLib: (libName: string, url: string, version?: string) =>
      tryPost<{ ok: boolean }>('/api/libs/index', { libName, url, version }),

    // ── Unified Query ────────────────────────────────────────────────────
    tryQuery: (q: string, repoId?: string, workspaceId?: string) =>
      tryPost<Record<string, unknown>>('/api/query', { q, repoId, workspaceId }),

    // ── Sessions ─────────────────────────────────────────────────────────
    /** Every captured session (#288: a failure is an error, not an empty list). */
    tryGetSessions: () => tryGet<SessionData>('/api/sessions'),

    // Observatory · Sessions digest (Slot 6). Same `/api/sessions` handler,
    // now scoped by an optional `range` (7d|30d|90d) and/or `workspace`
    // (name-or-uuid). Both are additive server-side. A failure is an error,
    // not the empty digest that read as a quiet week (#288).
    tryGetSessionsDigest: (range?: string, workspace?: string) => {
      const p = new URLSearchParams();
      if (range) p.set('range', range);
      if (workspace) p.set('workspace', workspace);
      const qs = p.toString() ? `?${p.toString()}` : '';
      return tryGet<SessionsDigest>(`/api/sessions${qs}`);
    },

    // ── Observatory · Logs (activity logs) ──────────────────────────────
    // Structured daemon/cli/mcp/app log rows for the Logs screen. All filters
    // are optional server query params: exact `level` / `source` / `module`
    // match, `since` (relative `15m|1h|24h|7d`, RFC-3339, or `all`), and a row
    // `limit` (clamped daemon-side to 1000). Newest-first. A failed read is an
    // ApiResult error — it once fell back to [], which read as "no entries".
    tryGetLogs: (q: { level?: string; source?: string; module?: string; since?: string; limit?: number } = {}) => {
      const p = new URLSearchParams();
      if (q.level) p.set('level', q.level);
      if (q.source) p.set('source', q.source);
      if (q.module) p.set('module', q.module);
      if (q.since) p.set('since', q.since);
      if (q.limit !== undefined) p.set('limit', String(q.limit));
      const qs = p.toString() ? `?${p.toString()}` : '';
      return tryGet<LogRow[]>(`/api/logs${qs}`);
    },

    // Background-worker registry (#96). A failed read is an error state — it
    // once fell back to an empty list, which read as "no background tasks".
    tryGetScheduledTasks: () => tryGet<{ tasks: ScheduledTask[] }>('/api/tasks/scheduled'),

    /** The last `lines` log lines a worker's passes wrote today (#96). */
    tryGetScheduledLog: (name: string, lines = 50) =>
      tryGet<{ worker: string; file: string; lines: string[] }>(
        `/api/tasks/scheduled/${enc(name)}/log?lines=${lines}`,
      ),

    // ── Scan ─────────────────────────────────────────────────────────────
    /** Add a root to the DB immediately (synchronous). Does not start scanning.
     *  The stored root, or why it was not added (#304). */
    tryAddWatchRoot: (path: string) =>
      tryPost<{ ok: boolean; id: string; path: string }>('/api/scan/roots', { path }),

    /** Remove a watch root (#247). The decision is REQUIRED: `keep` pauses it
     *  and leaves its data readable, `remove` prunes everything it held. Result-
     *  based, because the old fire-and-forget `del` let the screen drop a root
     *  whose removal had failed. */
    tryRemoveWatchRoot: (id: string, repositories: RootRemovalDecision) =>
      tryDeleteJson<RootRemovalResult>(
        `/api/scan/roots/${enc(id)}?repositories=${enc(repositories)}`,
      ),

    /** #218 — which sources sensei may read conversation history from. */
    tryGetTranscriptConsent: () =>
      tryGet<TranscriptSourceConsent[]>('/api/transcripts/consent'),

    trySetTranscriptConsent: (source: string, consented: boolean) =>
      tryPutJson<{ ok: boolean; source: string; consented: boolean }>(
        `/api/transcripts/consent/${enc(source)}`, { consented },
      ),

    // ── Issue trackers (spec R8) ─────────────────────────────────────────
    // Result-based with the error body kept: a refusal names its code and, for
    // some, the values the screen shows (the owning workspace, the board asked for).
    tryListTrackers: (workspaceId: string) =>
      tryJson<TrackerConnection[]>('GET', `/api/workspaces/${enc(workspaceId)}/trackers`),
    tryConnectTracker: (
      workspaceId: string,
      kind: string,
      level: TrackerLevel,
      body: Record<string, unknown>,
    ) =>
      tryJson<{ tracker: TrackerConnection; first_sync: { pending: boolean } }>(
        'POST', `/api/workspaces/${enc(workspaceId)}/trackers`, { kind, level, ...body },
      ),
    trySyncTracker: (workspaceId: string, trackerId: string) =>
      tryJson<{ counts: TrackerSyncCounts }>(
        'POST', `/api/workspaces/${enc(workspaceId)}/trackers/${enc(trackerId)}/sync`,
      ),
    tryDisconnectTracker: (workspaceId: string, trackerId: string) =>
      tryJson<void>('DELETE', `/api/workspaces/${enc(workspaceId)}/trackers/${enc(trackerId)}`),
    // Usage reads (spec R7/R8), one board each (D40). Result-based: a failed read
    // must never pass for "no tracker" or an empty sprint.
    tryListSprints: (workspaceId: string, trackerId: string) =>
      tryJson<SprintUsageRow[]>(
        'GET', `/api/workspaces/${enc(workspaceId)}/trackers/${enc(trackerId)}/sprints`,
      ),
    tryListTrackerTasks: (workspaceId: string, trackerId: string, sprintId?: string) =>
      tryJson<TrackerTaskRow[]>(
        'GET',
        `/api/workspaces/${enc(workspaceId)}/trackers/${enc(trackerId)}/tasks${sprintId ? `?sprint=${enc(sprintId)}` : ''}`,
      ),
    tryGetBacklog: (workspaceId: string, trackerId: string) =>
      tryJson<BacklogUsage>(
        'GET', `/api/workspaces/${enc(workspaceId)}/trackers/${enc(trackerId)}/backlog`,
      ),
    tryListBacklogTasks: (workspaceId: string, trackerId: string) =>
      tryJson<TrackerTaskRow[]>(
        'GET', `/api/workspaces/${enc(workspaceId)}/trackers/${enc(trackerId)}/tasks?backlog=true`,
      ),
    tryGetTrackerTask: (workspaceId: string, key: string) =>
      tryJson<TrackerTaskDetail>('GET', `/api/workspaces/${enc(workspaceId)}/tracker-tasks/${enc(key)}`),
    tryListUnlinkedSessions: (workspaceId: string, trackerId: string, sprintId: string) =>
      tryJson<UnlinkedSession[]>(
        'GET',
        `/api/workspaces/${enc(workspaceId)}/trackers/${enc(trackerId)}/sessions/unlinked?sprint=${enc(sprintId)}`,
      ),

    /** What stops syncing if this root is removed. */
    tryGetRootRepositories: (id: string) =>
      tryGet<RootRepository[]>(`/api/scan/roots/${enc(id)}/repositories`),

    /** Remove one repository and everything associated with it, and add it to
     *  its root's exclusions so a rescan does not bring it back. */
    tryPruneRepository: (path: string) => tryPost<PruneResult>('/api/scan/prune', { path }),

    /**
     * Replace one root's scan exclusions.
     *
     * A FULL REPLACE, which is what the endpoint is: it diffs the new list
     * against the stored one to decide which subtrees to prune and which to
     * re-scan. A delta would leave it unable to tell a removal from an omission.
     *
     * Error-propagating: an exclusion that appears set but was not stored means
     * the sweep continues while the screen says it stopped.
     */
    updateWatchRoot: (id: string, excluded: string[]) =>
      tryPutJson<{ ok: boolean; excluded: string[] }>(
        `/api/scan/roots/${enc(id)}`, { excluded },
      ),

    /** Start scanning a root — accepted, or why not (#304). */
    tryScanFolder: (root: string, maxDepth = 3) =>
      tryPost<{ ok: boolean; scanning: boolean }>('/api/scan', { root, max_depth: maxDepth }),

    // ── MCP Tool Proxy ────────────────────────────────────────────────
    tryMcpListTools: () => tryGet<{ tools: McpToolManifest[] }>('/api/mcp/tools'),

    /** Call a tool from the Playground — its answer, or why there is none. */
    tryMcpCallTool: (tool: string, params: Record<string, string>) =>
      tryPost<Record<string, unknown>>('/api/mcp/call', { tool, params }),

    // #84 T2 Slice C — Replay timeline with #90 verdicts joined + session
    // summary. Set `classify: true` to run the classifier before the read
    // (idempotent — safe on every open) so verdicts populate on first view.
    tryGetSessionReplay: (sessionId: string, opts: { limit?: number; classify?: boolean } = {}) => {
      const p = new URLSearchParams();
      p.set('limit', String(opts.limit ?? 200));
      if (opts.classify) p.set('classify', 'true');
      return tryGet<SessionReplayResponse>(`/api/sessions/${enc(sessionId)}/replay?${p.toString()}`);
    },

    // #84 T2 Slice A — discovered MCP servers (Claude / Cursor / Zed configs).
    // Error-propagating: an empty list is "nothing discovered", so a failed
    // read must not come back as one; nor a refused toggle as a done one.
    getMcpServers: (workspaceId?: string) => {
      const q = workspaceId ? `?workspace_id=${enc(workspaceId)}` : '';
      return tryGet<{ servers: McpServerRow[] }>(`/api/instruments/mcp-servers${q}`);
    },
    setMcpServerEnabled: (id: string, enabled: boolean) =>
      tryPut(`/api/instruments/mcp-servers/${enc(id)}/enabled`, { enabled }),
    refreshMcpServers: () =>
      tryPost<{ discovered: number; pruned: number }>('/api/instruments/mcp-servers/refresh', {}),

    // #84 T2 Slice B — cached tool manifest per server. `?refresh=true`
    // forces a re-probe; `?probe=false` returns stale-badged cache without
    // spawning.
    tryGetMcpServerTools: (
      serverId: string,
      opts: { refresh?: boolean; probe?: boolean } = {},
    ) => {
      const p = new URLSearchParams();
      if (opts.refresh) p.set('refresh', 'true');
      if (opts.probe === false) p.set('probe', 'false');
      const qs = p.toString() ? `?${p.toString()}` : '';
      return tryGet<McpServerToolsManifest>(`/api/instruments/mcp-servers/${enc(serverId)}/tools${qs}`);
    },

    // ── Marketplace ──────────────────────────────────────────────────
    tryMarketplaceInstall: (target: string, marketplacePath: string, item?: string, scope?: string) =>
      tryPost<{ ok: boolean }>('/api/marketplace/install', { target, marketplacePath, item, scope }),

    // ── Config (user preferences) ──────────────────────────────────────
    /** A config write — stored, or why not (#304). */
    trySetConfig: (config: Record<string, string>) =>
      tryPut('/api/config', config),

    /** Error-propagating variant of getConfig — used by appState.load() to
     *  distinguish "daemon unreachable" (don't touch cache) from "daemon says
     *  empty config" (clear cache). */
    tryGetConfig: () => tryGet<Record<string, string>>('/api/config'),

    tryDeleteConfig: (key: string) => tryDelete(`/api/config/${enc(key)}`),

    // ── Assistants ────────────────────────────────────────────────────────
    /** Detected assistants (#288: a failure is an error, not "none detected"). */
    tryDetectAssistants: () => tryGet<import('./types').AssistantStatus[]>('/api/assistants/detect'),

    /** Assistant families the daemon detects (#288: a failure is an error). */
    tryDetectAssistantFamilies: () => tryGet<import('./types').AssistantFamily[]>('/api/assistants/families'),

    /** Configure assistants — the daemon's per-assistant result, or why the
     *  request failed (#304: a failure was an empty result, read as success). */
    tryConfigureAssistants: (assistants: string[]) =>
      tryPost<import('./types').AssistantConfigureResult>('/api/assistants/configure', { acps: assistants }),

    tryRemoveAssistants: (assistants: string[] = []) =>
      tryPost<import('./types').AssistantRemoveResult>('/api/assistants/remove', { acps: assistants }),

    // ── Instruments (MCP registry — setup wizard Instruments stage) ───────
    tryListInstruments: () =>
      tryGet<{ total: number; mcps: import('./setup/contracts').DaemonMcpEntry[]; stack: string[] }>('/api/instruments'),

    // ── Installer ───────────────────────────────────────────────────────
    tryInstallAll: (assistants: string[], scope = 'global') =>
      tryPost<import('./types').InstallResult>('/api/install', { acps: assistants, scope }),

    tryInstallHooks: () =>
      tryPost<{ ok: boolean; count: number }>('/api/install/hooks', {}),

    tryInstallItem: (name: string, kind: string) =>
      tryPost<{ ok: boolean; path?: string; error?: string }>('/api/install/item', { name, kind }),

    tryRemoveItem: (name: string, kind: string) =>
      tryPost<{ ok: boolean }>('/api/install/item/remove', { name, kind }),

    /** Installed skills and commands (#288: a failure is an error, not "none installed"). */
    tryGetInstalledItems: () => tryGet<import('./types').InstalledItem[]>('/api/install/installed'),

    /** Toggle a skill or command. The daemon moves the .md file between
     *  `~/.claude/<kind>s/` and its `disabled/` sibling — Claude Code
     *  scans only the live folder. Idempotent server-side (the daemon
     *  returns `changed: false` when the item was already in the target
     *  state, but we discard that here — re-fetch the list to reflect
     *  new state).
     *  Error codes: 400 unknown kind, 404 unknown item or ambiguous
     *  state (item present in both live + disabled folders). */
    setInstalledItemEnabled: (name: string, kind: string, enabled: boolean) =>
      tryPut(
        `/api/install/installed/${enc(name)}/enabled`,
        { kind, enabled },
      ),

    tryRemoveAll: (purge = false) =>
      tryPost<import('./types').RemoveResult>('/api/remove', { purge }),

    // ── Gateway routers ──────────────────────────────────────────────────
    /** A failed read is an error, never "no routers" (#286). */
    tryListGatewayRouters: () =>
      tryGet<{ routers: import('./setup/contracts').DaemonRouter[] }>('/api/gateway/routers'),

    setGatewayRouterKey: (id: string, key: string) =>
      tryPost<{ ok: boolean; configured: boolean }>(
        `/api/gateway/routers/${enc(id)}/key`,
        { key },
      ),

    clearGatewayRouterKey: (id: string) =>
      tryDelete(`/api/gateway/routers/${enc(id)}/key`),

    // Model Assignments — chains carry an optional `role` column; a
    // chain-with-a-role IS the role assignment. Utility chains
    // (consensus-* / classify) stay null and never surface in the picker.
    /** The gateway's fallback chains (#288: a failure is an error, not "no chains"). */
    tryListGatewayChains: () => tryGet<{ chains: import('./setup/contracts').DaemonChain[] }>('/api/gateway/chains'),

    /** Assign or clear a chain's sensei inference role. Pass `null` to
     *  unassign. Returns error status codes: 400 unknown role, 404
     *  unknown chain, 409 role already owned by another chain. */
    setGatewayChainRole: (id: string, role: import('./setup/contracts').SenseiRole | null) =>
      tryPut(
        `/api/gateway/chains/${enc(id)}/role`,
        { role },
      ),

    /** Append a (model, router) pair to the end of a chain's ordered
     *  list. Returns the new member id + assigned sequence order. */
    addGatewayChainModel: (chainId: string, modelId: string, routerId: string) =>
      tryPost<{ ok: boolean; memberId: string; sequenceOrder: number }>(
        `/api/gateway/chains/${enc(chainId)}/models`,
        { model_id: modelId, router_id: routerId },
      ),

    /** Remove a chain member row. Compacts remaining sequence orders
     *  server-side so the list stays contiguous. */
    removeGatewayChainModel: (chainId: string, memberId: string) =>
      tryDelete(`/api/gateway/chains/${enc(chainId)}/models/${enc(memberId)}`),

    /** Swap a chain member with its neighbour. direction = -1 (up) /
     *  +1 (down). `moved: false` in the response means at a boundary,
     *  not an error — the UI dims the arrow. */
    moveGatewayChainModel: (chainId: string, memberId: string, direction: -1 | 1) =>
      tryPut(
        `/api/gateway/chains/${enc(chainId)}/models/${enc(memberId)}/move`,
        { direction },
      ),

    tryGenerateImage: (body: {
      prompt: string;
      output_path?: string;
      model?: string;
      router?: string;
      size?: string;
      quality?: string;
      style?: string;
      n?: number;
    }) =>
      tryPost<{ ok: boolean; paths: string[]; model?: string; router?: string }>(
        '/api/gateway/image/generate',
        body,
      ),

    // ── On-demand local-model provisioning ───────────────────────────────
    // The provisionable catalog with each model's current phase. Fallback is
    // the empty list so a daemon hiccup (or a non-embedded build) renders the
    // quiet state, never a broken panel; the UI can poll unconditionally.
    /** Local model catalog with each model's pull phase (#288: a failure is an
     *  error, not an empty catalog). */
    tryProvisionStatus: () => tryGet<{ models: ProvisionModel[] }>('/api/gateway/models/provision/status'),

    // Begin (or join) an on-demand pull of a local model, returning its initial
    // phase. `tryPost` so failures propagate: a 501 (embedded provisioning not
    // available in this build) surfaces as `{ ok: false, error: { status: 501,
    // … } }` and the UI shows the not-available notice rather than crashing.
    provisionModel: (id: string) =>
      tryPost<{ model: string; phase: ProvisionPhase }>(
        '/api/gateway/models/' + enc(id) + '/provision', {},
      ),

    // ── Knowledge plane ──────────────────────────────────────────────────
    /** Memories by status/scope/workspace (#288: a failure is an error, not an empty tab). */
    tryListMemories: (q: { status?: string; scope?: string; workspace_id?: string; limit?: number } = {}) => {
      const p = new URLSearchParams();
      for (const [k, v] of Object.entries(q)) if (v !== undefined) p.set(k, String(v));
      return tryGet<MemoryListResponse>(`/api/knowledge/memories?${p.toString()}`);
    },

    getMemoryDetail: (id: string) =>
      tryGet<MemoryDetail>(`/api/knowledge/memories/${encodeURIComponent(id)}`),

    proposeMemory: (body: ProposalCreateBody) =>
      tryPost<{ id: string; status: 'proposed' }>('/api/knowledge/proposals', body),

    saveMemory: (body: MemoryCreateBody) =>
      tryPost<{ id: string; status: 'active' }>('/api/knowledge/memories', body),

    // Ready-to-share lane — rewrite a workspace-scoped memory into a portable,
    // workspace-agnostic rule. `tryPost` so the 503 honest-degrade path (model
    // unavailable / returned nothing usable) surfaces as `{ ok: false, error }`
    // instead of a fabricated rewrite — the UI shows the error, keeps the
    // original. On success the daemon flips `generalised = true` and stores the
    // rewrite, so the caller re-fetches to reflect the new chip state.
    // `example` is a SYNTHETIC illustration minted by the same rewrite; it is
    // null when the model produced none — an absent illustration is not a failed
    // generalisation, and the daemon never substitutes one.
    generaliseMemory: (id: string) =>
      tryPost<{ id: string; original: string; generalised: string; example: string | null }>(
        `/api/knowledge/memories/${enc(id)}/generalise`, {},
      ),

    // Widen a proven memory up the scope ladder (workspace → user → dōjō →
    // collective). Reuses the EXISTING promote route — it creates a `proposed`
    // copy on the target namespace (origin=promoted); accepting it through the
    // normal proposal flow is the governance gate, so it never auto-applies.
    // `gov_scope` names the target scope ("user" | "organization" | "global"); the other
    // fields stay optional for namespace/folder-resolved promotions.
    promoteMemory: (
      id: string,
      body: { gov_scope?: string; namespace_id?: string; folder?: string; enforcement?: string } = {},
    ) =>
      tryPost<{ id: string; status: string; origin: string }>(
        `/api/knowledge/memories/${enc(id)}/promote`, body,
      ),

    // ── Memory lifecycle (triage → active → archive) ─────────────────────
    // Deterministic status transitions for an existing memory. All POST, no
    // body (except merge). `tryPost` so the daemon's `{ "error": "..." }` /
    // 409-terminal path surfaces as `{ ok: false, error }` and the UI can
    // degrade gracefully instead of crashing — mirrors promote/generalise.

    // Retire a memory. → `archived`.
    archiveMemory: (id: string) =>
      tryPost<{ id: string; status: 'archived' }>(
        `/api/knowledge/memories/${enc(id)}/archive`, {},
      ),

    // Strengthen a memory that proved out. → still active, `reinforced: true`.
    reinforceMemory: (id: string) =>
      tryPost<{ id: string; reinforced: boolean }>(
        `/api/knowledge/memories/${enc(id)}/reinforce`, {},
      ),

    // Contest a memory. → `challenged` (stays in the active set). 409 when the
    // memory is already in a terminal state (archived / rejected).
    challengeMemory: (id: string) =>
      tryPost<{ id: string; status: 'challenged' }>(
        `/api/knowledge/memories/${enc(id)}/challenge`, {},
      ),

    // Reject a memory outright. → `rejected`. 409 when already terminal.
    dismissMemory: (id: string) =>
      tryPost<{ id: string; status: 'rejected' }>(
        `/api/knowledge/memories/${enc(id)}/dismiss`, {},
      ),

    // Fold this memory into a surviving one (`into`). The folded memory is
    // archived. 400 on a missing/self merge, 404 when the survivor is unknown.
    // No merge-target picker exists on the triage screen yet, so this is not
    // wired to a button — kept here for when that affordance lands.
    mergeMemory: (id: string, into: string) =>
      tryPost<{ id: string; into: string; status: 'archived' }>(
        `/api/knowledge/memories/${enc(id)}/merge`, { into },
      ),

    acceptProposal: (id: string) =>
      tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/accept`, {}),

    rejectProposal: (id: string, reason?: string) =>
      tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/reject`, { reason }),

    recordOutcomes: (outcomes: OutcomeBody[]) =>
      tryPost<OutcomesBatchResponse>('/api/knowledge/outcomes', { outcomes }),

    // ── Governance Tier-2 · ruleset consolidation ────────────────────────
    // Tier-1 gathers a scope's raw rules; Tier-2 asks a model to merge them into
    // one coherent, deduped ruleset; a human approves. The current merged
    // ruleset (approved if present, else latest proposed) or `null` when a
    // consolidation has never run. Fallback is `null` so a daemon hiccup renders
    // the empty state, never a broken screen.
    // Error-propagating (mockup-drift-audit F8): `null` means "never consolidated"
    // (honest-empty). A fetch FAILURE must not collapse to that same null — tryGet
    // surfaces {ok:false,error} so the screen shows error-with-Retry instead.
    tryGetConsolidatedRuleset: () =>
      tryGet<ConsolidatedRuleset | null>('/api/knowledge/rules/consolidated'),

    // Run the merge now → a fresh `proposed` version, or `{ skipped, reason }`
    // when there's nothing to merge / the Tier-1 input is unchanged. `tryPost` so
    // a 502 (merge model unavailable) surfaces as `{ ok: false, error }` for the
    // screen to show, rather than being absorbed into a fallback.
    consolidateRules: () =>
      tryPost<ConsolidateResult>('/api/knowledge/rules/consolidate', {}),

    // Approve a proposed consolidated ruleset (the approval gate). Supersedes the
    // prior approved version and re-materializes the global rules.md. `tryPost` so
    // a 404 (unknown ruleset) surfaces to the screen. On success the page
    // re-fetches to reflect the approved state.
    approveConsolidatedRuleset: (id: string) =>
      tryPost<{ id: string; status: 'approved'; scope: string }>(
        `/api/knowledge/rules/consolidate/${enc(id)}/approve`, {},
      ),

    // ── Knowledge plane — federation sources ─────────────────────────────
    /** Registered knowledge sources (#288: a failure is an error, not "none"). */
    tryListKnowledgeSources: () => tryGet<{ sources: KnowledgeSource[] }>('/api/knowledge/sources'),

    createKnowledgeSource: (body: NewKnowledgeSourceBody) =>
      tryPost<KnowledgeSource>('/api/knowledge/sources', body),

    deleteKnowledgeSource: (id: string) =>
      tryDelete(`/api/knowledge/sources/${encodeURIComponent(id)}`),

    syncKnowledgeSource: (id: string) =>
      tryPost<SyncStats>(`/api/knowledge/sources/${encodeURIComponent(id)}/sync`, {}),

    // ── Dōjō connections (memberships) ───────────────────────────────────
    // Mirrors the federation knowledge-sources surface. GET returns a
    // top-level array (empty when no Dōjō is connected — the honest empty
    // state). `credential_ref` is never exposed; the device token on connect
    // is write-only. Fallback is the empty array so a daemon hiccup renders
    // the empty state, never a broken screen.
    // A failed read is an error to show, never the empty list (#262).
    tryGetDojoMemberships: () => tryGet<DojoMembership[]>('/api/dojo/memberships'),

    // Replace the git-remote owner slugs a membership covers (org-tagging) —
    // drives the R3 infer-at-detect auto-bind. `tryPut` so a 404 (unknown
    // membership) surfaces to the form.
    setMembershipOrgs: (membershipId: string, orgSlugs: string[]) =>
      tryPut(`/api/dojo/memberships/${enc(membershipId)}/orgs`, { org_slugs: orgSlugs }),

    // R3 infer-at-detect: the inferred (confirm-inferred) workspace→Dōjō binding
    // for the About-panel chip. `{ suggestion: null }` when already bound / no
    // git owner / no covering membership. Read-only — suggests, never binds.
    tryGetDojoSuggestion: (workspaceId: string) =>
      tryGet<{ suggestion: DojoBindingSuggestion | null }>(
        `/api/workspaces/${enc(workspaceId)}/dojo-suggestion`,
      ),

    // Confirm a workspace→Dōjō binding (the user accepting the inferred chip, or
    // an explicit bind). Fails closed server-side if the membership is unknown.
    bindWorkspaceDojo: (workspaceId: string, membershipId: string) =>
      tryPost<{ ok: boolean; dojo_id: string }>(
        `/api/workspaces/${enc(workspaceId)}/dojo-binding`,
        { membership_id: membershipId },
      ),

    // ── Dōjō downstream inbox (Observatory · Upgrades · C7) ──────────────
    // The daemon's inbox of approved Dōjō / Collective artifacts pulled back
    // for the user to review. Muted are hidden unless `includeMuted`; pinned
    // float to the top; `unread_count` counts the still-pending items. Fallback
    // is the empty inbox so a daemon that predates the route (404) degrades to
    // the empty state, never a broken screen.
    /** The Dōjō downstream inbox (#288: a failure is an error, not an empty inbox). */
    tryGetUpgrades: (includeMuted = false) =>
      tryGet<DojoUpgradesResponse>(`/api/upgrades${includeMuted ? '?include_muted=1' : ''}`),

    // Apply / Mute / Pin one artifact. `tryPost` so a failure surfaces as
    // `{ ok: false, error }` for the row to show and retry, rather than being
    // absorbed into a fallback — the screen re-loads on success via
    // `invalidateAll`. Apply lands a principle/pattern as a dojo memory
    // (skill/agent/prompt/guard defer); Mute hides it locally; Pin floats it to
    // the top and lets it outrank ambiguous local alternatives.
    applyUpgrade: (id: string) =>
      tryPost<Record<string, unknown>>(`/api/upgrades/${enc(id)}/apply`, {}),
    muteUpgrade: (id: string) =>
      tryPost<{ id: string; state: string }>(`/api/upgrades/${enc(id)}/mute`, {}),
    pinUpgrade: (id: string) =>
      tryPost<{ id: string; state: string }>(`/api/upgrades/${enc(id)}/pin`, {}),

    // ── Share review (Observatory · Share review · C11) ──────────────────
    // The upstream publish-gate. GET returns the next approved-but-unsent batch
    // with its per-item dereference PREVIEW, or `{ batch: null }` when nothing is
    // pending; the `get` fallback is that empty state so a daemon that predates
    // the route (404) degrades to the empty screen, never a broken one. Publish
    // runs the confidentiality-gated contribute path (client-work dereference is
    // mandatory and cannot be overridden); `tryPost` so a missing (404) or
    // not-approved (409) batch surfaces as `{ ok: false, error }` for the screen
    // to show, rather than being absorbed into a fallback. On success the screen
    // re-loads via `invalidateAll` and the returned outcome is shown.
    // Error-propagating (mockup-drift-audit F8): `{ batch: null }` means nothing
    // is pending (honest-empty). A daemon hiccup must NOT masquerade as that —
    // tryGet surfaces the error so the screen shows error-with-Retry.
    tryGetShareReviewBatch: () =>
      tryGet<ShareReviewResponse>('/api/share-review/next-batch'),

    publishBatch: (batchId: string) =>
      tryPost<PublishBatchOutcome>(`/api/share-review/${enc(batchId)}/publish`, {}),

    // ── Collective sharing preferences (Observatory · Collective · C9) ───────
    // GET always returns a FULL object on success (the stored row, or the daemon's
    // defaults when unset) — so a FAILURE here is a real error, never "empty".
    // Error-propagating (mockup-drift-audit F8): tryGet surfaces {ok:false,error}
    // so the screen shows error-with-Retry rather than fabricating the defaults
    // and hiding a broken daemon. PUT is a whole-object full-replace that echoes
    // the saved object back (fresh updated_at); `tryPutJson` so a 400 (bad enum /
    // unknown category / non-boolean) surfaces with the daemon's message.
    tryGetCollectivePreferences: () =>
      tryGet<CollectivePreferences>('/api/preferences/collective'),

    putCollectivePreferences: (body: CollectivePreferences) =>
      tryPutJson<CollectivePreferences>('/api/preferences/collective', body),

    // ── Metric computation status + activation (Settings · Metrics) ──────────
    // Two reads, because "every repository × every metric" is a cross join and
    // unbounded (10.9M rows in sensei_test): the SUMMARY is aggregated in SQL and
    // bounded by repository count; the per-metric rows are one repository at a
    // time. Both carry the reason vocabulary, so no row's code renders as a slug.
    //
    // Error-propagating throughout: a failed read here is a real error, and
    // showing an empty metric list would read as "nothing to configure".
    tryGetMetricStatusSummary: () =>
      tryGet<MetricStatusSummary>('/api/metrics/status/summary'),

    /** `repo` is a repo_key OR a repository uuid — a local-only repository has
     *  no key, so the uuid is the only handle that always works. */
    tryGetMetricStatus: (repo: string) =>
      tryGet<MetricStatusResponse>(`/api/metrics/status?repo=${enc(repo)}`),

    /**
     * Switch one metric on or off for one repository under one tenant.
     *
     * A PROXY, not a local write: `dojo.metric_activations` is the tenant's
     * record, and the daemon only reads the consequence back through the sync
     * plan. So the response is the dōjō's re-read ruling, and the local
     * `deactivated` column does not change until the next sync.
     *
     * `persona` is the KEYCHAIN SESSION SLOT (`personas.session_slot`), not a
     * display label — signing against the label addresses a different credential
     * or none. The tenant is deliberately NOT a parameter: the dōjō derives it
     * from the caller's own repositories, so one dōjō's member cannot write
     * another's cost decision.
     */
    patchMetricActivation: (
      persona: string,
      repoKey: string,
      metric: string,
      enabled: boolean,
    ) =>
      tryPatchJson<MetricActivationOutcome>('/api/dojo/metric-activation', {
        persona, repo_key: repoKey, metric, enabled,
      }),

    // ── Dōjō sync state (Settings · Dōjō) ────────────────────────────────────
    // When each entity last AGREED with the dōjō, and what went wrong where it
    // has not. Error-propagating: an empty list on a failed read would say
    // "everything is fine", which for a sync surface is the most expensive lie
    // available.
    tryGetDojoSyncState: () => tryGet<SyncStateResponse>('/api/dojo/sync-state'),

    // ── Lifecycle ────────────────────────────────────────────────────────
    tryStop: () => tryPost<Record<string, unknown>>('/stop', {}),
  };
}

export type SenseiApi = ReturnType<typeof senseiApi>;

function enc(s: string): string {
  return encodeURIComponent(s);
}
