// packages/tools/src/tools/bm25.ts
const K1 = 1.5;
const B = 0.75;

function tokenize(text: string): string[] {
  return text.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
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
