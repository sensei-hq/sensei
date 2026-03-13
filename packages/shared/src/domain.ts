// ─── Phase 1 domain types ─────────────────────────────────────────────────────

export interface Repo {
  id: string;
  name: string;
  local_path: string;
  remote_url: string | null;
  stack: string[];
  entry_points: EntryPoint[];
  last_indexed_at: string | null;
  created_at: string;
}

export interface EntryPoint {
  path: string;
  role: string;
}

export type SymbolKind =
  | "function" | "class" | "type" | "interface"
  | "enum" | "const" | "method" | "component" | "hook" | "unknown";

export interface RepoSymbol {
  id: string;
  repo_id: string;
  file_path: string;
  name: string;
  kind: SymbolKind;
  signature: string | null;
  docstring: string | null;
  line_start: number;
  line_end: number;
  is_exported: boolean;
  updated_at: string;
}

export interface CallEdge {
  id: string;
  repo_id: string;
  caller_id: string;
  callee_name: string;
  callee_file: string | null;
}

export interface Import {
  id: string;
  repo_id: string;
  source_file: string;
  target_path: string;
  names: string[];
}

export interface FileEntry {
  path: string;       // repo-relative path
  absPath: string;    // absolute path on disk
  mtime: number;      // milliseconds epoch
  hash: string;       // sha256 hex of file contents
  size: number;       // bytes
}

export interface ScanResult {
  repoId: string;
  files: FileEntry[];
  changed: string[];   // repo-relative paths changed since last scan
  deleted: string[];   // repo-relative paths removed since last scan
}

export interface ParsedSymbol {
  name: string;
  kind: SymbolKind;
  signature: string | null;
  docstring: string | null;
  lineStart: number;
  lineEnd: number;
  isExported: boolean;
}

export interface ParsedEdge {
  callerName: string;
  calleeName: string;
  calleeFile: string | null;
}

export interface ParsedImport {
  targetPath: string;
  names: string[];
}

export interface ParsedFile {
  filePath: string;    // repo-relative
  language: string;
  symbols: ParsedSymbol[];
  edges: ParsedEdge[];
  imports: ParsedImport[];
}

export interface IndexResult {
  repoId: string;
  symbolsUpserted: number;
  edgesUpserted: number;
  importsUpserted: number;
  filesIndexed: number;
  filesDeleted: number;
  durationMs: number;
  errors: string[];
}

// ─── Phase 2 context pack types ───────────────────────────────────────────────

export interface CodeSlice {
  kind: "code";
  filePath: string;
  startLine: number;
  endLine: number;
  content: string;
  tokens: number;
  symbolName: string;
  score: number;
}

export interface DocSlice {
  kind: "doc";
  filePath: string;
  heading: string;
  startLine: number;
  endLine: number;
  content: string;
  tokens: number;
  score: number;
}

export type Slice = CodeSlice | DocSlice;

export interface ContextPack {
  id: string;
  task: string;
  slices: Slice[];
  totalTokens: number;
  modelId?: string;
  createdAt: string;
}
