// packages/tools/src/tools/search.ts
import { readFile } from "fs/promises";
import { existsSync } from "fs";
import type { SymbolMap } from "@sensei/shared";
import { senseiPath, loadSenseiConfig, makeSenseiClient } from "@sensei/shared";
import { scoreBM25 } from "./bm25.js";
import { embed } from "./embedder.js";

export interface SearchResult {
  id: string;
  file: string;
  type: "symbol" | "doc";
  excerpt: string;
  score: number;
  matchedBy: Array<"symbol" | "bm25" | "semantic">;
}

interface Chunk {
  file: string;
  type: "symbol" | "doc";
  text: string;
  contentHash: string;
  tf: Record<string, number>;
}
interface ChunksFile {
  version: number;
  corpusSize: number;
  avgChunkLength: number;
  chunks: Record<string, Chunk>;
}
interface EmbeddingsFile {
  version: number;
  model: string;
  dimensions: number;
  vectors: Record<string, number[]>;
}

// Module-level cache keyed by repoPath (reset between tests via module reload)
const chunksCache = new Map<string, ChunksFile>();
const embeddingsCache = new Map<string, EmbeddingsFile>();
let reindexInProgress = false;

function tokenize(text: string): string[] {
  const raw = text.split(/[^a-zA-Z0-9]+/).filter(Boolean);
  const tokens: string[] = [];
  for (const tok of raw) {
    const parts = tok.split(/(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])/);
    for (const p of parts) tokens.push(p.toLowerCase());
  }
  return tokens;
}

function cosineSimilarity(a: number[], b: number[]): number {
  let dot = 0, magA = 0, magB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    magA += a[i] * a[i];
    magB += b[i] * b[i];
  }
  const denom = Math.sqrt(magA) * Math.sqrt(magB);
  return denom === 0 ? 0 : dot / denom;
}

function symbolNameFromL0(l0: string): string {
  return l0.replace(/^(?:export\s+)?(?:async\s+)?(?:function|class|const|type|interface|enum)\s+/, "")
           .split(/[(<: ]/)[0].trim();
}

async function loadChunksFromDb(client: any, repoId: string): Promise<ChunksFile | null> {
  try {
    const { data } = await (client as any)
      .schema("sensei").from("chunks")
      .select("id, file_path, chunk_type, text, content_hash, tf")
      .eq("repo_id", repoId);
    if (!data || data.length === 0) return null;

    const chunks: Record<string, Chunk> = {};
    for (const row of data) {
      chunks[row.id] = {
        file: row.file_path,
        type: row.chunk_type as "symbol" | "doc",
        text: row.text,
        contentHash: row.content_hash,
        tf: row.tf ?? {},
      };
    }

    const tokenCounts = Object.values(chunks).map(c => tokenize(c.text).length);
    const corpusSize = tokenCounts.length;
    const avgChunkLength = corpusSize > 0
      ? Math.round(tokenCounts.reduce((a, b) => a + b, 0) / corpusSize)
      : 0;

    return { version: 1, corpusSize, avgChunkLength, chunks };
  } catch {
    return null;
  }
}

async function loadEmbeddingsFromDb(client: any, repoId: string): Promise<EmbeddingsFile | null> {
  try {
    const { data } = await (client as any)
      .schema("sensei").from("chunks")
      .select("id, embedding")
      .eq("repo_id", repoId)
      .not("embedding", "is", null);
    if (!data || data.length === 0) return null;

    const vectors: Record<string, number[]> = {};
    for (const row of data) {
      if (row.embedding) vectors[row.id] = row.embedding as number[];
    }
    return { version: 1, model: "Xenova/all-MiniLM-L6-v2", dimensions: 384, vectors };
  } catch {
    return null;
  }
}

async function loadChunks(repoPath: string, dbClient?: any, repoId?: string): Promise<ChunksFile | null> {
  if (chunksCache.has(repoPath)) return chunksCache.get(repoPath)!;

  if (dbClient && repoId) {
    const data = await loadChunksFromDb(dbClient, repoId);
    if (data) { chunksCache.set(repoPath, data); return data; }
  }

  const path = senseiPath(repoPath, "chunks.json");
  if (!existsSync(path)) return null;
  try {
    const data = JSON.parse(await readFile(path, "utf-8")) as ChunksFile;
    chunksCache.set(repoPath, data);
    return data;
  } catch { return null; }
}

async function loadEmbeddings(repoPath: string, dbClient?: any, repoId?: string): Promise<EmbeddingsFile | null> {
  if (embeddingsCache.has(repoPath)) return embeddingsCache.get(repoPath)!;

  if (dbClient && repoId) {
    const data = await loadEmbeddingsFromDb(dbClient, repoId);
    if (data) { embeddingsCache.set(repoPath, data); return data; }
  }

  const path = senseiPath(repoPath, "embeddings.json");
  if (!existsSync(path)) return null;
  try {
    const data = JSON.parse(await readFile(path, "utf-8")) as EmbeddingsFile;
    embeddingsCache.set(repoPath, data);
    return data;
  } catch { return null; }
}

async function loadSymbolMapFromDb(client: any, repoId: string): Promise<SymbolMap | null> {
  try {
    const { data } = await (client as any)
      .schema("sensei").from("symbol_map")
      .select("file_path, l0, l1")
      .eq("repo_id", repoId);
    if (!data || data.length === 0) return null;

    const symbolMap: SymbolMap = {};
    for (const row of data) {
      symbolMap[row.file_path] = {
        L0: Array.isArray(row.l0) ? row.l0 : (typeof row.l0 === "string" ? row.l0.split("\n").filter(Boolean) : []),
        L1: Array.isArray(row.l1) ? row.l1 : (typeof row.l1 === "string" ? row.l1.split("\n").filter(Boolean) : []),
        L2: [],
      };
    }
    return symbolMap;
  } catch {
    return null;
  }
}

async function loadSymbolMap(repoPath: string, dbClient?: any, repoId?: string): Promise<SymbolMap> {
  if (dbClient && repoId) {
    const data = await loadSymbolMapFromDb(dbClient, repoId);
    if (data && Object.keys(data).length > 0) return data;
  }

  const path = senseiPath(repoPath, "symbol-map.json");
  if (!existsSync(path)) return {};
  try { return JSON.parse(await readFile(path, "utf-8")) as SymbolMap; } catch { return {}; }
}

function splitCamel(s: string): string[] {
  return s.split(/(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])|[^a-zA-Z0-9]+/)
    .filter(Boolean)
    .map(t => t.toLowerCase());
}

function symbolSearch(query: string, symbolMap: SymbolMap): Array<{ id: string; score: number; file: string; type: "symbol" | "doc" }> {
  const q = query.toLowerCase();
  const queryTokens = splitCamel(query).filter(t => t.length > 2);
  const results: Array<{ id: string; score: number; file: string; type: "symbol" | "doc" }> = [];

  for (const [file, symbols] of Object.entries(symbolMap)) {
    for (const l0 of symbols.L0 ?? []) {
      const name = symbolNameFromL0(l0).toLowerCase();
      if (!name) continue;
      let score = 0;
      if (name === q) score = 1.0;
      else if (name.startsWith(q)) score = 0.8;
      else if (name.includes(q)) score = 0.5;
      else if (queryTokens.length > 0) {
        const nameTokens = splitCamel(name);
        const matchCount = queryTokens.filter(qt => nameTokens.some(nt => nt.startsWith(qt) || qt.startsWith(nt))).length;
        if (matchCount > 0) score = 0.3 * (matchCount / queryTokens.length);
      }
      if (score > 0) {
        results.push({ id: `${file}:${symbolNameFromL0(l0)}`, score, file, type: "symbol" });
      }
    }
  }

  return results.sort((a, b) => b.score - a.score);
}

function rrfMerge(
  layers: Array<Array<{ id: string }>>,
  chunkData: Record<string, Chunk>,
  symbolLayerResults: Array<{ id: string; file: string; type: "symbol" | "doc" }>,
  layerNames: Array<"symbol" | "bm25" | "semantic">
): SearchResult[] {
  const RRF_K = 60;
  const scores: Record<string, number> = {};
  const matchedBy: Record<string, Set<"symbol" | "bm25" | "semantic">> = {};

  for (let i = 0; i < layers.length; i++) {
    const layer = layers[i];
    const name = layerNames[i];
    for (let rank = 0; rank < layer.length; rank++) {
      const id = layer[rank].id;
      scores[id] = (scores[id] ?? 0) + 1 / (RRF_K + rank);
      if (!matchedBy[id]) matchedBy[id] = new Set();
      matchedBy[id].add(name);
    }
  }

  const symbolById = Object.fromEntries(symbolLayerResults.map(r => [r.id, r]));

  return Object.entries(scores)
    .sort((a, b) => b[1] - a[1])
    .map(([id, score]) => {
      const chunk = chunkData[id];
      const sym = symbolById[id];
      const file = chunk?.file ?? sym?.file ?? id.split(":")[0];
      const type: "symbol" | "doc" = chunk?.type ?? sym?.type ?? "symbol";
      const text = chunk?.text ?? "";
      return {
        id,
        file,
        type,
        excerpt: text.slice(0, 200),
        score,
        matchedBy: Array.from(matchedBy[id] ?? []),
      };
    });
}

export async function search(
  repoPath: string,
  query: string,
  options?: { top?: number; type?: "all" | "symbol" | "fulltext" | "semantic" }
): Promise<SearchResult[] | string> {
  const top = options?.top ?? 10;
  const type = options?.type ?? "all";

  // Auto-load Supabase client from config if present
  let dbClient: any = null;
  let repoId: string | undefined;
  try {
    const config = await loadSenseiConfig(repoPath);
    if (config) {
      dbClient = await makeSenseiClient(repoPath);
      repoId = config.repo_id;
    }
  } catch { /* offline or no config — use file fallback */ }

  const [symbolMap, chunks, embeddings] = await Promise.all([
    loadSymbolMap(repoPath, dbClient, repoId),
    loadChunks(repoPath, dbClient, repoId),
    loadEmbeddings(repoPath, dbClient, repoId),
  ]);

  const layers: Array<Array<{ id: string }>> = [];
  const layerNames: Array<"symbol" | "bm25" | "semantic"> = [];
  let symbolLayerResults: Array<{ id: string; file: string; type: "symbol" | "doc" }> = [];

  // Symbol layer
  if (type === "all" || type === "symbol") {
    symbolLayerResults = symbolSearch(query, symbolMap);
    if (symbolLayerResults.length > 0) {
      layers.push(symbolLayerResults);
      layerNames.push("symbol");
    }
  }

  // BM25 layer
  if ((type === "all" || type === "fulltext") && chunks) {
    const bm25Results = scoreBM25(query, chunks.chunks, chunks.corpusSize, chunks.avgChunkLength);
    if (bm25Results.length > 0) {
      layers.push(bm25Results);
      layerNames.push("bm25");
    }
  }

  // Semantic layer
  if ((type === "all" || type === "semantic") && embeddings && Object.keys(embeddings.vectors).length > 0) {
    try {
      const queryVec = await embed(query);
      const semResults = Object.entries(embeddings.vectors)
        .map(([id, vec]) => ({ id, score: cosineSimilarity(queryVec, vec) }))
        .filter(r => r.score > 0.01)
        .sort((a, b) => b.score - a.score);
      if (semResults.length > 0) {
        layers.push(semResults);
        layerNames.push("semantic");
      }
    } catch {
      console.warn("Semantic search unavailable — run sensei index to generate embeddings");
    }
  }

  if (layers.length === 0) {
    if (!reindexInProgress) {
      reindexInProgress = true;
      import("./reindex.js")
        .then(({ reindexRepo }) => reindexRepo(repoPath))
        .finally(() => { reindexInProgress = false; });
      return "No results found. Index may be stale — reindexing in background, retry in a moment.";
    }
    return "No results found. Reindex already in progress — retry in a moment.";
  }

  const chunkData = chunks?.chunks ?? {};
  const merged = rrfMerge(layers, chunkData, symbolLayerResults, layerNames);
  return merged.slice(0, top);
}
