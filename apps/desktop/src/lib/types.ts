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
  repoId: string;
  name: string;
  path: string;
  indexedAt?: string;
  lastError?: string;
  partiallyIndexed?: boolean;
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
