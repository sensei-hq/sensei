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
  preferences: PreferencesData;
  assistantFamilies: DaemonAssistantFamily[];
  roots: DaemonWatchRoot[];
  projects: DaemonProject[];
  libraries: { total: number; libs: DaemonLibEntry[] };
  mcps: DaemonMcpEntry[];
  routers: DaemonRouter[];
}
