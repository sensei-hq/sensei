import { readFile } from "fs/promises";
import { existsSync } from "fs";
import { join } from "path";
import { createHash } from "crypto";
import type { SymbolMap } from "@sensei/shared";
import { embed, ensureReady } from "./embedder.js";

interface Chunk {
  file: string;
  type: "symbol" | "doc";
  text: string;
  contentHash: string;
  tf: Record<string, number>;
}

interface ChunksFile {
  version: 1;
  corpusSize: number;
  avgChunkLength: number;
  chunks: Record<string, Chunk>;
}


function tokenize(text: string): string[] {
  // Split on non-alphanumeric boundaries, then further split camelCase tokens
  const raw = text.split(/[^a-zA-Z0-9]+/).filter(Boolean);
  const tokens: string[] = [];
  for (const tok of raw) {
    // Split camelCase / PascalCase: e.g. "reindexRepo" → ["reindex", "repo"]
    const parts = tok.split(/(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])/);
    for (const p of parts) tokens.push(p.toLowerCase());
  }
  return tokens;
}

function computeTf(text: string): Record<string, number> {
  const tf: Record<string, number> = {};
  for (const term of tokenize(text)) {
    tf[term] = (tf[term] ?? 0) + 1;
  }
  return tf;
}

function contentHash(text: string): string {
  return createHash("sha256").update(text).digest("hex").slice(0, 16);
}

function symbolNameFromL0(l0: string): string {
  // Extract name: everything before first `(`, `<`, ` `, or `:`
  return l0.replace(/^(?:export\s+)?(?:async\s+)?(?:function|class|const|type|interface|enum)\s+/, "")
           .split(/[(<: ]/)[0]
           .trim();
}

function chunkTextForSymbol(l0: string, l1: string): string {
  // l1 typically looks like "// description\n// signature" or just "// signature"
  const lines = l1.split("\n").map(l => l.replace(/^\/\/\s?/, "").trim()).filter(Boolean);
  const hasDescription = lines.length >= 2 || (lines.length === 1 && lines[0] !== l0.trim());
  const text = hasDescription ? `${l0}\n${lines.join(" ")}` : l0;
  return text.slice(0, 300);
}

function sectionId(file: string, heading: string): string {
  const slug = heading.replace(/^#{2,3}\s+/, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
  return `${file}#${slug}`;
}

export async function extractChunks(
  symbolMap: SymbolMap,
  docFiles: string[],
  repoPath: string
): Promise<Record<string, { file: string; type: "symbol" | "doc"; text: string }>> {
  const result: Record<string, { file: string; type: "symbol" | "doc"; text: string }> = {};

  // Code chunks — one per symbol from symbol-map
  for (const [file, symbols] of Object.entries(symbolMap)) {
    const l0s = symbols.L0 ?? [];
    const l1s = symbols.L1 ?? [];
    for (let i = 0; i < l0s.length; i++) {
      const l0 = l0s[i];
      const l1 = l1s[i] ?? `// ${l0}`;
      const name = symbolNameFromL0(l0);
      if (!name) continue;
      const id = `${file}:${name}`;
      const text = chunkTextForSymbol(l0, l1);
      result[id] = { file, type: "symbol", text };
    }
  }

  // Doc chunks — one per H2/H3 section
  for (const docFile of docFiles) {
    if (!docFile.endsWith(".md") && !docFile.endsWith(".mdx")) continue;
    const fullPath = join(repoPath, docFile);
    if (!existsSync(fullPath)) continue;
    try {
      const content = await readFile(fullPath, "utf-8");
      const lines = content.split("\n");
      let currentHeading: string | null = null;
      let bodyLines: string[] = [];

      const flush = () => {
        if (!currentHeading) return;
        const headingText = currentHeading.replace(/^#{2,3}\s+/, "");
        const bodyText = bodyLines
          .join(" ")
          .replace(/[#*`_\[\]]/g, "")
          .replace(/\s+/g, " ")
          .trim()
          .slice(0, 400);
        const text = `${headingText}\n${bodyText}`.trim();
        if (text) {
          const id = sectionId(docFile, currentHeading);
          result[id] = { file: docFile, type: "doc", text };
        }
      };

      for (const line of lines) {
        if (/^#{2,3} /.test(line)) {
          flush();
          currentHeading = line;
          bodyLines = [];
        } else if (currentHeading) {
          bodyLines.push(line);
        }
      }
      flush();
    } catch { /* skip unreadable */ }
  }

  return result;
}

export async function buildChunksAndEmbeddings(
  repoPath: string,
  symbolMap: SymbolMap,
  docFiles: string[],
  options?: { force?: boolean; client?: any; repoId?: string }
): Promise<void> {
  const client = options?.client ?? null;
  const repoId = options?.repoId ?? null;
  const useDb = !!(client && repoId);

  // Chunks and embeddings are DB-only — no local file storage
  if (!useDb) return;

  // Load existing data for incremental embedding (skip unchanged chunks)
  let existingChunks: Record<string, { contentHash: string; tf: Record<string, number> }> = {};
  let existingVectors: Record<string, number[]> = {};

  if (!options?.force) {
    try {
      const { data } = await (client as any)
        .schema("sensei").from("chunks")
        .select("id, content_hash, tf, embedding")
        .eq("repo_id", repoId);
      if (data) {
        for (const row of data) {
          existingChunks[row.id] = { contentHash: row.content_hash, tf: row.tf ?? {} };
          if (row.embedding) {
            existingVectors[row.id] = typeof row.embedding === 'string'
              ? JSON.parse(row.embedding)
              : row.embedding as number[];
          }
        }
      }
    } catch { /* start fresh */ }
  }

  // Ensure model is ready before the embedding loop — downloads if not cached
  let embeddingAvailable = true;
  try {
    await ensureReady();
  } catch {
    embeddingAvailable = false;
    console.warn("Semantic search unavailable — run sensei index to generate embeddings");
  }

  const rawChunks = await extractChunks(symbolMap, docFiles, repoPath);
  const newChunks: ChunksFile["chunks"] = {};
  const newVectors: Record<string, number[]> = {};

  for (const [id, raw] of Object.entries(rawChunks)) {
    const hash = contentHash(raw.text);
    const tf = computeTf(raw.text);
    newChunks[id] = { file: raw.file, type: raw.type, text: raw.text, contentHash: hash, tf };

    // Re-use existing vector if text unchanged
    if (existingChunks[id]?.contentHash === hash && existingVectors[id]) {
      newVectors[id] = existingVectors[id];
    } else if (embeddingAvailable) {
      try {
        newVectors[id] = await embed(raw.text);
      } catch {
        // Embedding failed for this chunk — omit vector, continue
      }
    }
  }

  const tokenCounts = Object.values(newChunks).map(c => tokenize(c.text).length);
  const corpusSize = tokenCounts.length;
  const avgChunkLength = corpusSize > 0
    ? Math.round(tokenCounts.reduce((a, b) => a + b, 0) / corpusSize)
    : 0;

  {
    // Upsert all chunks to DB in batches
    const BATCH = 200;
    const allNewIds = Object.keys(newChunks);
    const rows = allNewIds.map(id => {
      const chunk = newChunks[id];
      return {
        repo_id: repoId,
        id,
        file_path: chunk.file,
        chunk_type: chunk.type,
        text: chunk.text,
        content_hash: chunk.contentHash,
        tf: chunk.tf,
        embedding: newVectors[id] ? `[${newVectors[id].join(",")}]` : null,
        updated_at: new Date().toISOString(),
      };
    });

    for (let i = 0; i < rows.length; i += BATCH) {
      await (client as any).schema("sensei").from("chunks")
        .upsert(rows.slice(i, i + BATCH), { onConflict: "repo_id,id" });
    }

    // Delete stale chunks (IDs no longer in the index)
    if (allNewIds.length > 0) {
      const { data: oldRows } = await (client as any)
        .schema("sensei").from("chunks")
        .select("id")
        .eq("repo_id", repoId);
      const toDelete = (oldRows ?? [])
        .map((r: { id: string }) => r.id)
        .filter((id: string) => !new Set(allNewIds).has(id));
      if (toDelete.length > 0) {
        await (client as any).schema("sensei").from("chunks")
          .delete()
          .eq("repo_id", repoId)
          .in("id", toDelete);
      }
    }
  }
}
