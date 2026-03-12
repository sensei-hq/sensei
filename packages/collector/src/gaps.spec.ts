import { describe, it, expect } from "vitest";
import { detectGapPatterns, BASH_TO_TOOL_PATTERNS } from "./gaps.js";

describe("detectGapPatterns", () => {
  it("maps grep to search_index", () => {
    const commands = [
      "grep -r 'auth' src/",
      "grep -r 'token' src/",
      "grep -r 'auth' src/",
    ];
    const gaps = detectGapPatterns(commands);
    const grepGap = gaps.find(g => g.pattern.includes("grep"));
    expect(grepGap).toBeDefined();
    expect(grepGap!.count).toBe(3);
    expect(grepGap!.suggested_tool).toContain("search_index");
  });

  it("maps find to Glob", () => {
    const commands = ["find . -name '*.ts'", "find . -name '*.json'"];
    const gaps = detectGapPatterns(commands);
    const findGap = gaps.find(g => g.pattern.includes("find"));
    expect(findGap).toBeDefined();
    expect(findGap!.suggested_tool).toContain("Glob");
  });

  it("maps cat to Read", () => {
    const commands = ["cat src/index.ts", "cat package.json"];
    const gaps = detectGapPatterns(commands);
    const catGap = gaps.find(g => g.pattern.includes("cat"));
    expect(catGap).toBeDefined();
    expect(catGap!.suggested_tool).toContain("Read");
  });

  it("returns results sorted by count descending", () => {
    const commands = [
      "grep -r 'x' src/",
      "grep -r 'y' src/",
      "grep -r 'z' src/",
      "cat file.ts",
    ];
    const gaps = detectGapPatterns(commands);
    for (let i = 1; i < gaps.length; i++) {
      expect(gaps[i - 1].count).toBeGreaterThanOrEqual(gaps[i].count);
    }
  });

  it("ignores commands that have no sensei equivalent", () => {
    const commands = ["npm install", "git status", "echo hello"];
    const gaps = detectGapPatterns(commands);
    // None of these map to a sensei tool
    expect(gaps).toHaveLength(0);
  });

  it("returns empty array for empty input", () => {
    expect(detectGapPatterns([])).toHaveLength(0);
  });

  it("maps rg to search_index", () => {
    const commands = ["rg 'auth' src/", "rg 'token'"];
    const gaps = detectGapPatterns(commands);
    const rgGap = gaps.find(g => g.suggested_tool.includes("search_index"));
    expect(rgGap).toBeDefined();
    expect(rgGap!.count).toBe(2);
  });

  it("maps sed to Edit", () => {
    const commands = ["sed -i 's/foo/bar/' file.ts"];
    const gaps = detectGapPatterns(commands);
    const sedGap = gaps.find(g => g.suggested_tool.includes("Edit"));
    expect(sedGap).toBeDefined();
  });

  it("maps curl to WebFetch", () => {
    const commands = ["curl https://example.com/api"];
    const gaps = detectGapPatterns(commands);
    const curlGap = gaps.find(g => g.suggested_tool.includes("WebFetch"));
    expect(curlGap).toBeDefined();
  });

  it("exports BASH_TO_TOOL_PATTERNS with at least 4 entries", () => {
    expect(Object.keys(BASH_TO_TOOL_PATTERNS).length).toBeGreaterThanOrEqual(4);
  });
});
