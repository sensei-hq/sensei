import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { ASTSlicer } from "./ast-slicer.js";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";
import type { TokenCounter } from "@sensei/shared";

const counter: TokenCounter = { name: "estimate", count: (t) => Math.ceil(t.length / 4) };

function makeDb(symbols: Array<{name: string; line_start: number; line_end: number}>) {
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          eq: vi.fn().mockResolvedValue({ data: symbols, error: null }),
        })),
      })),
    })),
  } as any;
}

describe("ASTSlicer", () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), "ast-slicer-"));
    await mkdir(join(dir, "src"), { recursive: true });
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("returns one CodeSlice per symbol with correct content", async () => {
    const lines = ["line 1", "export function foo() {", "  return 1;", "}", "export function bar() {", "  return 2;", "}"];
    await writeFile(join(dir, "src/a.ts"), lines.join("\n"));

    const db = makeDb([
      { name: "foo", line_start: 2, line_end: 4 },
      { name: "bar", line_start: 5, line_end: 7 },
    ]);

    const candidate: ScoredCandidate = { filePath: "src/a.ts", type: "code", score: 1.5, strategyScores: {} };
    const slicer = new ASTSlicer(db, dir, "repo-1");
    const slices = await slicer.slice(candidate, counter);

    expect(slices).toHaveLength(2);
    expect(slices[0].kind).toBe("code");
    expect(slices[0].symbolName).toBe("foo");
    expect(slices[0].startLine).toBe(2);
    expect(slices[0].endLine).toBe(4);
    expect(slices[0].score).toBe(1.5);
    expect(slices[0].content).toContain("export function foo()");
    expect(slices[0].tokens).toBeGreaterThan(0);
  });

  it("returns [] when file does not exist", async () => {
    const db = makeDb([{ name: "foo", line_start: 1, line_end: 3 }]);
    const candidate: ScoredCandidate = { filePath: "src/nonexistent.ts", type: "code", score: 1.0, strategyScores: {} };
    const slicer = new ASTSlicer(db, dir, "repo-1");
    expect(await slicer.slice(candidate, counter)).toHaveLength(0);
  });

  it("returns [] when no symbols found", async () => {
    await writeFile(join(dir, "src/empty.ts"), "// no symbols");
    const db = makeDb([]);
    const candidate: ScoredCandidate = { filePath: "src/empty.ts", type: "code", score: 1.0, strategyScores: {} };
    const slicer = new ASTSlicer(db, dir, "repo-1");
    expect(await slicer.slice(candidate, counter)).toHaveLength(0);
  });
});
