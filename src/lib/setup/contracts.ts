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

/** An AI coding assistant family. */
export interface DaemonAssistantFamily {
  id: string;
  name: string;
  installed: boolean;
  selected: boolean;
  config_path: string | null;
  version: string | null;
  install_path: string | null;
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

/** A detected library. */
export interface DaemonLibEntry {
  id: string;
  name: string;
  version: string;
  lang: string;
  usage: number;
  source: string;
  docs: 'indexed' | 'partial' | 'schema' | 'none';
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
}
