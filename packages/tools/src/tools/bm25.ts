// packages/tools/src/tools/bm25.ts
const K1 = 1.5;
const B = 0.75;

function tokenize(text: string): string[] {
  // Split on non-alphanumeric boundaries, then further split camelCase tokens
  const raw = text.split(/[^a-zA-Z0-9]+/).filter(Boolean);
  const tokens: string[] = [];
  for (const tok of raw) {
    // Split camelCase / PascalCase: e.g. "reindexRepo" → ["reindex", "Repo"]
    const parts = tok.split(/(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])/);
    for (const p of parts) tokens.push(p.toLowerCase());
  }
  return tokens;
}

export function scoreBM25(
  query: string,
  chunks: Record<string, { tf: Record<string, number> }>,
  corpusSize: number,
  avgChunkLength: number
): Array<{ id: string; score: number }> {
  const queryTerms = tokenize(query);
  if (queryTerms.length === 0) return [];

  // Compute df per query term
  const df: Record<string, number> = {};
  for (const term of queryTerms) {
    df[term] = 0;
    for (const chunk of Object.values(chunks)) {
      if ((chunk.tf[term] ?? 0) > 0) df[term]++;
    }
  }

  const results: Array<{ id: string; score: number }> = [];

  for (const [id, chunk] of Object.entries(chunks)) {
    const chunkLen = Object.values(chunk.tf).reduce((a, b) => a + b, 0);
    let score = 0;

    for (const term of queryTerms) {
      const termFreq = chunk.tf[term] ?? 0;
      if (termFreq === 0) continue;

      const n = corpusSize;
      const d = df[term] ?? 0;
      const idf = Math.log((n - d + 0.5) / (d + 0.5) + 1);
      const norm = K1 * (1 - B + B * chunkLen / (avgChunkLength || 1));
      score += idf * (termFreq * (K1 + 1)) / (termFreq + norm);
    }

    if (score > 0) results.push({ id, score });
  }

  return results.sort((a, b) => b.score - a.score);
}
