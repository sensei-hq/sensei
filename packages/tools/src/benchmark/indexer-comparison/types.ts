/** A file indexed by either indexer */
export interface IndexedFile {
  path: string;
}

/** A code chunk returned by cocoindex search */
export interface CocoChunk {
  filePath: string;
  language: string;
  content: string;
  startLine: number;
  endLine: number;
  score: number;
}

/** A named symbol from sensei's symbol map */
export interface SenseiSymbol {
  name: string;
  path: string;
  L0: string;   // signature
  L1?: string;  // description (may be absent)
}

/** Test query with expected file paths that should appear in results */
export interface TestQuery {
  query: string;
  expectedFiles: string[];  // partial path matches count
}

export interface ComparisonReport {
  cocoFilesIndexed: number;
  senseiFilesIndexed: number;
  groundTruthExports: number;
  cocoCoverage: number;     // 0–1
  senseiCoverage: number;   // 0–1
  queryResults: QueryComparison[];
  spotCheck: SpotCheckRow[];
}

export interface QueryComparison {
  query: string;
  cocoHit: boolean;   // expected file appeared in top-5 results
  senseiHit: boolean;
}

export interface SpotCheckRow {
  filePath: string;
  cocoContent: string | null;   // first chunk content (truncated)
  senseiDescription: string | null; // L1 description or L0 signature
}
