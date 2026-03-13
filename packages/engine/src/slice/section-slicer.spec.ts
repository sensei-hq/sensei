import { describe, it, expect, vi } from "vitest";
import { SectionSlicer } from "./section-slicer.js";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";
import type { TokenCounter } from "@sensei/shared";

const counter: TokenCounter = { name: "estimate", count: (t) => Math.ceil(t.length / 4) };

function makeDb(sections: Array<{heading: string; level: number; start_line: number; end_line: number; content: string}>) {
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          eq: vi.fn().mockResolvedValue({ data: sections, error: null }),
        })),
      })),
    })),
  } as any;
}

describe("SectionSlicer", () => {
  it("returns one DocSlice per section", async () => {
    const sections = [
      { heading: "Overview", level: 2, start_line: 1, end_line: 5, content: "This is the overview." },
      { heading: "Usage", level: 2, start_line: 6, end_line: 10, content: "Use it like this." },
    ];
    const candidate: ScoredCandidate = { filePath: "docs/guide.md", type: "doc", score: 0.9, strategyScores: {} };
    const slicer = new SectionSlicer(makeDb(sections), "repo-1");
    const slices = await slicer.slice(candidate, counter);

    expect(slices).toHaveLength(2);
    expect(slices[0].kind).toBe("doc");
    expect(slices[0].heading).toBe("Overview");
    expect(slices[0].score).toBe(0.9);
    expect(slices[0].tokens).toBeGreaterThan(0);
  });

  it("returns [] when no sections found", async () => {
    const candidate: ScoredCandidate = { filePath: "docs/empty.md", type: "doc", score: 0.5, strategyScores: {} };
    const slicer = new SectionSlicer(makeDb([]), "repo-1");
    expect(await slicer.slice(candidate, counter)).toHaveLength(0);
  });

  it("returns [] when DB errors", async () => {
    const errorDb = {
      from: vi.fn(() => ({
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            eq: vi.fn().mockResolvedValue({ data: null, error: { message: "fail" } }),
          })),
        })),
      })),
    } as any;
    const candidate: ScoredCandidate = { filePath: "docs/guide.md", type: "doc", score: 0.5, strategyScores: {} };
    expect(await new SectionSlicer(errorDb, "repo-1").slice(candidate, counter)).toHaveLength(0);
  });
});
