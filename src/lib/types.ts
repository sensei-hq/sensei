// ─── Stage pattern ─────────────────────────────────────────────────────────────

import type { EventManager } from './events.js';
import type { Component } from 'svelte';

export interface Stage {
  id: string;
  title: string;
  icon: string;
  description: string;
  watermark?: boolean;
  component: Component;
  canAdvance: () => boolean;
  load?: () => Promise<void>;
  source?: EventManager<any>;
}

export interface StateEvent<T extends { id: string }> {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;
  data: T | T[];
}

// ─── Scan ──────────────────────────────────────────────────────────────────────

export type FolderStatus = 'discovered' | 'queued' | 'indexing' | 'indexed' | 'failed';
export type ProjectStatus = 'scanning' | 'indexing' | 'active' | 'failed';
export type ActivityLevel = 'discover' | 'queue' | 'process' | 'info' | 'success' | 'error';

export interface ScanProjectFolder {
  id: string;
  name: string;
  path: string;
  stack: string[];
  filesTotal: number;
  filesCompleted: number;
  status: FolderStatus;
}

/** Folder-level SSE event from daemon — includes projectId for routing. */
export interface ScanFolderEvent extends ScanProjectFolder {
  projectId: string;
}

export interface ScanProject {
  id: string;
  name: string;
  status: ProjectStatus;
  folders: ScanProjectFolder[];
  autoDetected: boolean;
  confidence: 'high' | 'medium' | 'low';
}

export interface ActivityEvent {
  id: string;
  level: ActivityLevel;
  message: string;
  elapsed: number;
  timestamp: number;
}

// ─── Assistants (AI coding tools) ──────────────────────────────────────────────

export interface AssistantStatus {
  id: string;
  name: string;
  family: string;
  installed: boolean;
  mcp_configured: boolean;
  config_path: string;
}

export interface AssistantFamily {
  family: string;
  name: string;
  members: AssistantStatus[];
  installed: boolean;
  config_path: string;
}

export interface AssistantConfigureResult {
  configured: string[];
  skipped: string[];
  errors: string[];
}

// ─── Installer ────────────────────────────────────────────────────────────────

export interface InstallResult {
  hooks_installed: number;
  skills_installed: number;
  commands_installed: number;
  stale_commands_removed: number;
  stale_skills_removed: number;
  assistants_configured: string[];
  errors: string[];
  marketplace_version: string;
}

export interface RemoveResult {
  assistants_removed: string[];
  plugin_removed: boolean;
  commands_removed: number;
  skills_removed: number;
  agents_removed: number;
  hooks_removed: boolean;
  cache_cleared: boolean;
  projects_cleaned: string[];
  errors: string[];
}

export interface AssistantRemoveResult {
  assistants_removed: string[];
  errors: string[];
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

// ─── Project Model (group of 1+ repos) ──────────────────────────────────────

export type RepoRole =
  | 'backend' | 'frontend' | 'mobile' | 'middleware'
  | 'infra' | 'docs' | 'library' | 'shared' | 'unknown';

export type ProjectCategory = 'active' | 'side' | 'idea';

export interface ProjectRepo {
  repoId: string;
  path: string;
  role: RepoRole;
  label?: string;
}

export interface Project {
  id: string;
  name: string;
  description?: string;
  client?: string;
  category: ProjectCategory;
  repos: ProjectRepo[];
  createdAt: string;
  updatedAt: string;
}

/** @deprecated Use Project instead */
export type Solution = Project;
/** @deprecated Use ProjectRepo instead */
export type SolutionRepo = ProjectRepo;
/** @deprecated Use ProjectCategory instead */
export type SolutionCategory = ProjectCategory;

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
  doc_type?: string;
  level?: string;
  parent_id?: string;
  tags?: string;
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

export interface ProjectGraphResponse {
  projectId: string;
  name: string;
  nodes: number;
  edges: number;
  repos: number;
  graph: { nodes: GraphNode[]; edges: GraphEdge[] };
}

export interface ProjectAnalysis {
  project_id: string;
  links: Array<{
    from_repo: string; to_repo: string; link_type: string;
    details: string[]; strength: number;
  }>;
  inferred_roles: InferredRole[];
  shared_libs: Array<{ name: string; repos: string[] }>;
}

/** @deprecated Use ProjectGraphResponse instead */
export type SolutionGraphResponse = ProjectGraphResponse;
/** @deprecated Use ProjectAnalysis instead */
export type SolutionAnalysis = ProjectAnalysis;

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

// ── Diagnostic Log Types ──────────────────────────────────────────────────

export interface SystemInfo {
    os:        string;
    arch:      string;
    ram_gb:    number;
    cpu_cores: number;
}

export interface LogEntry {
    id:     string;
    ts:     string;
    level:  'info' | 'warn' | 'error';
    layer:  'ui' | 'api' | 'sidecar' | 'data_load';
    step:   string;
    msg:    string;
    data?:  Record<string, unknown>;
    err?:   string;
    stack?: string;
}

export interface BootstrapTrace {
    id:            string;
    ts:            string;
    action_type:   'check' | 'resolve' | 'instruct';
    step:          string;
    desc:          string;
    cmd:           string;
    exit:          number | null;
    out:           string;
    err:           string;
    ms:            number;
    ok:            boolean;
    fix_attempted: boolean;
    fix_approach:  string | null;
    fix_ok:        boolean | null;
}

export interface LogSession {
    id:          string;
    module:      string;
    started_at:  string;
    app_version: string;
    system_info: SystemInfo;
    outcome:     'success' | 'partial' | 'failed';
    duration_ms: number;
    traces:      (BootstrapTrace | LogEntry)[];
}
