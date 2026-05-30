/**
 * Daemon response contracts — source of truth for API shapes.
 * Every interface maps to what the daemon returns over HTTP.
 */

/** A watch root directory (folders_to_watch row). */
export interface DaemonWatchRoot {
  id: string;
  path: string;
  name: string;
  status: 'scanning' | 'watching' | 'paused';
  excluded: string[];
  repos_found: number;
  scanned: boolean;
  modified_at: string;
}

/** One detected variant within an assistant family (e.g. "Claude Code", "Claude Desktop"). */
export interface AssistantVariant {
  id: string;
  name: string;
  installed: boolean;
  /** True when sensei is integrated with this variant — mirrors AssistantStatus.configured. */
  configured: boolean;
}

/** An AI coding assistant product family (e.g. "Claude", "Cursor"). */
export interface DaemonAssistantFamily {
  id: string;        // family key: "claude", "cursor", "zed", etc.
  name: string;      // display name: "Claude", "Cursor", etc.
  selected: boolean;
  variants: AssistantVariant[];
}

/** A project with its folders. */
export interface DaemonProject {
  id: string;
  name: string;
  description: string | null;
  client: string | null;
  goal: string | null;
  stack: { languages: string[]; frameworks: string[]; runtimes: string[]; services: string[] };
  icon: { kind: string; value: string };
  folders: DaemonProjectFolder[];
}

export interface DaemonProjectFolder {
  id: string;
  name: string;
  path: string;
  kind: string;
  role: string | null;
}

/** A detected library — what `GET /api/libs` actually returns now. The
 *  daemon joins `libraries` ⨝ `referenced_libraries` ⨝ `folders` so every
 *  entry carries its ecosystem (npm/pypi/cargo/go/docs), latest known
 *  version, optional description, and the folders that reference it.
 *  `enabled` is the user's intent for whether sensei should wrap this
 *  library, persisted as a set in the `setup.libraries` config key. */
export interface DaemonLibEntry {
  id: string;        // = library uuid (stable across refreshes)
  name: string;
  ecosystem: string; // "npm" | "pypi" | "cargo" | "go" | "docs"
  version: string | null;
  description: string | null;
  pageCount: number;
  repos: string[];
  repoCount: number;
  enabled: boolean;
}

/** An MCP server entry from the registry. */
export interface DaemonMcpEntry {
  id: string;
  name: string;
  publisher: string;
  kind: string;
  summary: string;
  tools: number;
  verified: boolean;
  installed: boolean;
  recommended: boolean;
  selected: boolean;
  project_count: number;
}

/** A gateway router — endpoint that may front multiple providers. */
export interface DaemonRouter {
  id: string;
  name: string;
  providers: string[];
  capabilities: string[];
  needs_key: boolean;
  configured: boolean;
}

/** Scan baseline — derived from loaded data at hydration time. */
export interface ScanBaseline {
  rootCount: number;
  repoCount: number;
  fileCount: number;
  scannedRootIds: string[];
}

/** Preferences stored as a single JSON object in daemon config. */
export interface PreferencesData {
  displayName: string;
  contributeLearnings: boolean;
  reviewBeforeShare: boolean;
  shareSchedule: string;
  downloadCollective: string;
  correctionAggressiveness: string;
  digestCadence: string;
  nudgeOnRegression: boolean;
  anonymizedTelemetry: boolean;
  showWelcome: boolean;
}

/** Bundle returned by layout load — assembled from parallel daemon fetches. */
export interface WizardLoadData {
  completion: Record<string, 'pending' | 'done'>;
  /** Daemon's `config['setup_complete']` — true when the user finished the
   *  wizard end-to-end. Sourced from the same getConfig() call that
   *  populates `completion`; we surface it separately so wizardState can
   *  reconcile its `setupComplete` $state (and the localStorage cache)
   *  against daemon truth on every load. */
  setupComplete: boolean;
  preferences: PreferencesData;
  assistantFamilies: DaemonAssistantFamily[];
  roots: DaemonWatchRoot[];
  projects: DaemonProject[];
  libraries: { total: number; libs: DaemonLibEntry[] };
  mcps: DaemonMcpEntry[];
  routers: DaemonRouter[];
}

// ── Knowledge plane (Phase 0) ─────────────────────────────────────────

export type MemoryStatus =
    | 'proposed' | 'active' | 'reinforced' | 'challenged'
    | 'battle_tested' | 'archived' | 'rejected';

export type MemoryScope = 'global' | 'project' | 'stack' | 'task_type' | 'module';

export type OutcomeKind = 'applied' | 'consulted' | 'violated' | 'ignored';

export interface Memory {
    id:               string;
    project_id:       string | null;
    scope:            MemoryScope;
    scope_filter:     string | null;
    type:             string;
    title:            string;
    content:          string;
    impact:           string | null;
    strength:         number;
    status:           MemoryStatus;
    applied_count:    number;
    violated_count:   number;
    last_relevant_at: string | null;
    tags:             string[];
    triage_signal:    string | null;
    modified_at:      string;
}

export interface MemoryEvidence {
    session_id:  string | null;
    note:        string | null;
    recorded_at: string;
}

export interface MemoryExample {
    node_id:  string | null;
    is_good:  boolean;
    note:     string | null;
}

export interface MemoryOutcomeRecord {
    outcome:     OutcomeKind;
    session_id:  string | null;
    context:     string | null;
    recorded_at: string;
}

export interface MemoryDetail {
    memory:    Memory;
    evidence:  MemoryEvidence[];
    examples:  MemoryExample[];
    outcomes:  MemoryOutcomeRecord[];
}

export interface MemoryListResponse { memories: Memory[]; }

export interface ContextResponse {
    version:     string;
    memories:    Memory[];
    cache_until: string;
}

export interface ProposalCreateBody {
    project_id?:   string;
    scope:         MemoryScope;
    scope_filter?: string;
    type:          string;
    title:         string;
    content:       string;
    impact?:       string;
    tags?:         string[];
    triage_signal: string;
}

export interface MemoryCreateBody {
    project_id?:   string;
    scope:         MemoryScope;
    scope_filter?: string;
    type:          string;
    title:         string;
    content:       string;
    impact?:       string;
    tags?:         string[];
}

export interface OutcomeBody {
    memory_id:   string;
    outcome:     OutcomeKind;
    session_id?: string;
    context?:    string;
}

export interface OutcomesBatchResponse {
    recorded: number;
    skipped:  { memory_id: string; reason: string }[];
}
