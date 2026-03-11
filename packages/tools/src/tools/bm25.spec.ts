// packages/tools/src/tools/bm25.spec.ts
import { describe, it, expect } from "vitest";
import { scoreBM25 } from "./bm25.js";

const authChunk = { tf: { login: 2, email: 1, authenticate: 1, user: 2 } };
const homeChunk = { tf: { home: 3, page: 2, render: 1 } };
const corpus = { "src/auth.ts:login": authChunk, "src/home.ts:render": homeChunk };

describe("scoreBM25", () => {
  it("returns auth chunk first for 'login' query", () => {
    const results = scoreBM25("login", corpus, 2, 5);
    expect(results[0].id).toBe("src/auth.ts:login");
    expect(results[0].score).toBeGreaterThan(0);
  });

  it("returns only positive-score results", () => {
    const results = scoreBM25("unrelated", corpus, 2, 5);
    expect(results.every(r => r.score > 0)).toBe(true);
    expect(results).toHaveLength(0);
  });

  it("term in all chunks scores near zero for IDF", () => {
    // With N=20 chunks all containing 'common': df=20
    // IDF = log((20-20+0.5)/(20+0.5)+1) = log(0.0244+1) ≈ 0.0241
    // score ≈ 0.0241 * (1 * 2.5) / (1 + 1.5) ≈ 0.024 → well below 0.1
    const ubiquitous = Object.fromEntries(
      Array.from({ length: 20 }, (_, i) => [`chunk-${i}`, { tf: { common: 1 } }])
    );
    const results = scoreBM25("common", ubiquitous, 20, 1);
    results.forEach(r => expect(r.score).toBeLessThan(0.1));
  });

  it("longer documents are penalised (b=0.75 effect)", () => {
    // Same term frequency, one chunk is longer
    const short = { tf: { auth: 1, x: 1 } };            // length 2
    const long  = { tf: { auth: 1, a: 1, b: 1, c: 1, d: 1, e: 1 } }; // length 6
    const mixed = { "short": short, "long": long };
    const results = scoreBM25("auth", mixed, 2, 4);
    const shortScore = results.find(r => r.id === "short")?.score ?? 0;
    const longScore  = results.find(r => r.id === "long")?.score ?? 0;
    expect(shortScore).toBeGreaterThan(longScore);
  });

  it("returns results sorted descending by score", () => {
    const results = scoreBM25("authenticate user", corpus, 2, 5);
    for (let i = 1; i < results.length; i++) {
      expect(results[i - 1].score).toBeGreaterThanOrEqual(results[i].score);
    }
  });
});
