import { describe, it, expect } from "vitest";
import { Assembler } from "./assembler.js";
import type { Slice, TokenCounter } from "@sensei/shared";

// 1 char = 1 token for predictable tests
const counter: TokenCounter = { name: "estimate", count: (t) => t.length };

function makeSlice(filePath: string, content: string, score: number): Slice {
  return {
    kind: "code",
    filePath,
    startLine: 1,
    endLine: 5,
    content,
    tokens: content.length,
    symbolName: "foo",
    score,
  };
}

describe("Assembler", () => {
  it("never exceeds maxTokens", () => {
    const slices = Array.from({ length: 10 }, (_, i) => makeSlice(`file${i}.ts`, "x".repeat(50), 1.0 - i * 0.1));
    const pack = new Assembler().assemble(slices, { maxTokens: 100, counter, task: "task" });
    expect(pack.totalTokens).toBeLessThanOrEqual(100);
  });

  it("deduplicates slices from sessionContext", () => {
    const slices = [
      makeSlice("src/auth.ts", "auth code", 2.0),   // in sessionContext — skip
      makeSlice("src/utils.ts", "utils code", 1.0), // not in sessionContext — include
    ];
    const pack = new Assembler().assemble(slices, { maxTokens: 8000, counter, task: "task", sessionContext: ["src/auth.ts"] });
    expect(pack.slices).toHaveLength(1);
    expect(pack.slices[0].filePath).toBe("src/utils.ts");
  });

  it("includes slices in score order", () => {
    const slices = [makeSlice("low.ts", "low", 0.3), makeSlice("high.ts", "high", 2.0), makeSlice("mid.ts", "mid", 1.0)];
    const pack = new Assembler().assemble(slices, { maxTokens: 8000, counter, task: "task" });
    expect(pack.slices[0].filePath).toBe("high.ts");
    expect(pack.slices[1].filePath).toBe("mid.ts");
    expect(pack.slices[2].filePath).toBe("low.ts");
  });

  it("sets totalTokens to sum of included slice tokens", () => {
    const slices = [makeSlice("a.ts", "hello", 1.0), makeSlice("b.ts", "world", 0.9)];
    const pack = new Assembler().assemble(slices, { maxTokens: 8000, counter, task: "task" });
    expect(pack.totalTokens).toBe(10);  // "hello".length + "world".length
  });

  it("sets task, modelId, id, createdAt on the pack", () => {
    const pack = new Assembler().assemble([], { maxTokens: 8000, counter, task: "fix bug", modelId: "gpt-4o" });
    expect(pack.task).toBe("fix bug");
    expect(pack.modelId).toBe("gpt-4o");
    expect(pack.id).toBeTruthy();
    expect(pack.createdAt).toBeTruthy();
  });
});
