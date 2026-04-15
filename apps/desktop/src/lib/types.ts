// ─── ACP (AI Coding Platforms) ─────────────────────────────────────────────────

export interface AcpStatus {
  id: string;
  name: string;
  installed: boolean;
  mcp_configured: boolean;
  config_path: string;
}

export interface AcpConfigureResult {
  configured: string[];
  skipped: string[];
  errors: string[];
}

// ─── Installer ────────────────────────────────────────────────────────────────

export interface InstallResult {
  hooks_installed: number;
  skills_installed: number;
  commands_installed: number;
  acps_configured: string[];
  errors: string[];
  marketplace_version: string;
}

export interface UninstallResult {
  acps_removed: string[];
  hooks_removed: boolean;
  skills_removed: number;
  plugin_removed: boolean;
  cache_cleared: boolean;
}

export interface InstalledItem {
  name: string;
  kind: string;
  path: string;
}

export interface MarketplaceCatalogItem {
  name: string;
  kind: string;
  description: string;
  scope: string;
  path: string;
  recommended_for: string[];
  stage: string[];
}

export interface MarketplaceCatalog {
  version: string | null;
  items: MarketplaceCatalogItem[];
}

// ─── Scanned Repo (from Tauri analyze_folder) ─────────────────────────────────

export type RepoStatus = 'active' | 'recent' | 'stale' | 'archived' | 'abandoned' | 'unknown';

export interface ScannedRepo {
  name: string;
  path: string;
  repoId?: string;
  remote?: string | null;
  description?: string | null;
  categories: string[];
  status: RepoStatus;
  last_commit_days?: number | null;
  tech_stack: string[];
  commit_count: number;
  duplicate_of?: string | null;
  variant_group?: string | null;
  client?: string;
}

// ─── Indexing ─────────────────────────────────────────────────────────────────

export type IndexStatus = 'idle' | 'queued' | 'running' | 'done' | 'failed' | 'partial';

export interface IndexProgress {
  currentFile: string;
  filesProcessed: number;
  filesTotal: number;
  filesUnchanged: number;
  filesSkipped: number;
  filesFailed: number;
  startedAt: string;
}

export interface RepoEntry {
  name: string;
  path: string;
  repoId: string;
  indexedAt?: string;
  status: IndexStatus;
  error?: string;
  progress?: IndexProgress;
}

export interface IndexQueueStatus {
  current: { repo_id: string; repo_path: string; status: string } | null;
  queued: Array<{ repo_id: string; repo_path: string; status: string }>;
  recent: Array<{ repo_id: string; repo_path: string; status: string }>;
}

export interface IndexProgressEvent {
  type: 'queued' | 'started' | 'progress' | 'completed' | 'failed';
  repo_id?: string;
  position?: number;
  files_total?: number;
  current_file?: string;
  files_processed?: number;
  files_indexed?: number;
  functions_indexed?: number;
  types_indexed?: number;
  edges_created?: number;
  duration_ms?: number;
  error?: string;
}

export interface DirtyStatus {
  repoId: string;
  codeFiles: number;
  docFiles: number;
  total: number;
}

// ─── Solution Model ──────────────────────────────────────────────────────────

export type RepoRole =
  | 'backend' | 'frontend' | 'mobile' | 'middleware'
  | 'infra' | 'docs' | 'library' | 'shared' | 'unknown';

export type SolutionCategory = 'active' | 'side' | 'idea';

export interface SolutionRepo {
  repoId: string;
  path: string;
  role: RepoRole;
  label?: string;
}

export interface Solution {
  id: string;
  name: string;
  description?: string;
  client?: string;
  category: SolutionCategory;
  repos: SolutionRepo[];
  createdAt: string;
  updatedAt: string;
}

// ─── Server API response shapes ──────────────────────────────────────────────

export interface ServerProject {
  repo_id: string;
  name: string;
  path: string;
  stack: string[];
  libs: string[];
  tags: string[];
  status: string;
  indexed_at?: string;
  last_error?: string;
  // Aliases for existing UI code (populated by API client)
  repoId: string;
  indexedAt?: string;
  lastError?: string;
  partiallyIndexed?: boolean;
}

export interface ProjectSummary {
  repoId: string;
  name: string;
  path: string;
  stack: string[];
  libs: string[];
  tags: string[];
  status: string;
  indexedAt?: string;
  functions: number;
  types: number;
  packages: number;
  modules: number;
  edges: number;
  solutions: Array<{ solutionId: string; solutionName: string; role: string }>;
}

export interface GraphNode {
  id: string;
  name: string;
  kind: string;
  file: string;
  line: number;
  complexity?: number;
  repoId?: string;
  role?: string;
}

export interface GraphEdge {
  source: string;
  target: string;
  type: string;
  repoId?: string;
}

export interface GraphData {
  summary: { totalSymbols: number; totalEdges: number; communities: number };
  communities: Array<{
    id: string; label: string; project: string; color: string;
    symbolCount: number; godNodes: string[];
  }>;
  godNodes: Array<{
    name: string; project: string; degree: number; community: string; file: string;
  }>;
  rationale: Array<{
    file: string; tag: string; project: string; text: string;
  }>;
}

export interface SolutionGraphResponse {
  solutionId: string;
  name: string;
  nodes: number;
  edges: number;
  repos: number;
  graph: { nodes: GraphNode[]; edges: GraphEdge[] };
}

export interface SolutionAnalysis {
  solution_id: string;
  links: Array<{
    from_repo: string; to_repo: string; link_type: string;
    details: string[]; strength: number;
  }>;
  inferred_roles: InferredRole[];
  shared_libs: Array<{ name: string; repos: string[] }>;
}

export interface InferredRole {
  repo_id: string;
  role: string;
  confidence: number;
  reasons: string[];
}

// ─── Graph Queries ───────────────────────────────────────────────────────────

export interface FunctionDetail {
  id: string;
  name: string;
  file: string;
  line: number;
  signature?: string;
  docstring?: string;
  complexity: number;
  tags?: string;
}

export interface TypeDetail {
  id: string;
  name: string;
  file: string;
  line: number;
  kind: string;
}

export interface CommunityInfo {
  id: number;
  size: number;
  sample_members: string[];
}

export interface DocDrift {
  doc_id: string;
  doc_path: string;
  edge_type: string;
  changed_target: string;
}

// ─── Libraries ───────────────────────────────────────────────────────────────

export interface LibEntry {
  name: string;
  repos: string[];
  repoCount: number;
}

export interface LibDoc {
  id: string;
  title: string;
  url: string;
  summary: string;
  content?: string;
  source_type: string;
  component: string;
  indexed_at: string;
}

export interface DepVersion {
  lib_name: string;
  version: string;
  raw_version: string;
  source: string;
  dev: boolean;
}

// ─── Sessions ────────────────────────────────────────────────────────────────

export interface SessionData {
  stats: Record<string, unknown> | null;
  sessions: Array<{
    id: string; task: string; project: string; startedAt: string;
    completedAt?: string; outcome?: string; summary?: string;
    cost?: number; tokensIn?: number; tokensOut?: number; ftr?: number | null;
  }>;
  toolUsage: unknown[];
  benchmarkPairs: unknown[];
}

export interface IndexError {
  repo_id: string;
  file_path: string;
  error: string;
  adapter?: string;
  timestamp: string;
}
