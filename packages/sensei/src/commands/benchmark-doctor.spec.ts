import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";

const TMP = "/tmp/sensei-benchmark-test";

// Mock callClaude to avoid real API calls
vi.mock("../claude.js", () => ({
  callClaude: vi.fn().mockResolvedValue({
    text: `## File: 01-core.md\n# Core\n## Features\n### Feature A\nTODO\n## Status\n| Feature | Status |\n|---|---|\n| A | 🔲 Planned |`,
    usage: { tokensIn: 100, tokensOut: 200 },
  }),
}));

import {
  buildTargetedIndexPrompt,
  buildRawContentPrompt,
  buildFullRepoIndexPrompt,
  parseOutputFolder,
  structuralScore,
} from "./benchmark-doctor.js";

beforeEach(() => {
  mkdirSync(join(TMP, "requirements"), { recursive: true });
  mkdirSync(join(TMP, "examples"), { recursive: true });
  mkdirSync(join(TMP, ".index"), { recursive: true });
  writeFileSync(join(TMP, "requirements/01-core.md"), "# Core\nThe core module handles auth.\n");
  writeFileSync(join(TMP, "examples/01-example.md"), "# Example\n## Features\n### Login\nTODO\n## Status\n| Feature | Status |\n|---|---|\n");
  writeFileSync(join(TMP, ".index/symbol-map.json"), JSON.stringify({
    "docs/requirements/01-core.md": { L0: ["# Core"], L1: ["handles auth"], L2: [] },
  }));
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("buildTargetedIndexPrompt", () => {
  it("includes only input-dir entries from symbol-map", () => {
    const prompt = buildTargetedIndexPrompt({
      inputDir: join(TMP, "requirements"),
      repoPath: TMP,
      templateContent: "# Template",
      outputName: "features",
    });
    expect(prompt).toContain("01-core.md");
    expect(prompt).toContain("# Template");
    expect(prompt).not.toContain("src/");
  });
});

describe("buildRawContentPrompt", () => {
  it("includes full file content from input dir and examples", () => {
    const prompt = buildRawContentPrompt({
      inputDir: join(TMP, "requirements"),
      examplesDir: join(TMP, "examples"),
      outputName: "features",
    });
    expect(prompt).toContain("handles auth");
    expect(prompt).toContain("01-example.md");
  });
});

describe("buildFullRepoIndexPrompt", () => {
  it("includes entire symbol-map and template", () => {
    const prompt = buildFullRepoIndexPrompt({
      repoPath: TMP,
      templateContent: "# Template",
      outputName: "features",
    });
    expect(prompt).toContain("symbol-map");
    expect(prompt).toContain("# Template");
  });
});

describe("parseOutputFolder", () => {
  it("splits Claude response on ## File: markers", () => {
    const raw = `## File: README.md\n# Features\nOverview\n\n## File: 01-core.md\n# Core\n## Features\n`;
    const files = parseOutputFolder(raw);
    expect(files["README.md"]).toContain("# Features");
    expect(files["01-core.md"]).toContain("## Features");
  });

  it("handles response without file markers as README", () => {
    const raw = `# Features\nSome content`;
    const files = parseOutputFolder(raw);
    expect(files["README.md"]).toContain("# Features");
  });
});

describe("structuralScore", () => {
  it("scores high for perfect output matching template sections", () => {
    const template = "## Features\n## Status\n";
    const output = {
      "01.md": "## Features\n### A\nTODO\n## Status\n| F | S |\n|---|---|\n| A | 🔲 Planned |",
      "README.md": "# Overview",
    };
    const original = { "01-core.md": "# Core\nauth module" };
    const score = structuralScore({ template, output, original });
    expect(score).toBeGreaterThanOrEqual(8);
  });

  it("penalises missing README", () => {
    const template = "## Features\n## Status\n";
    const output = { "01.md": "## Features\n## Status\n" };
    const original = { "01-core.md": "auth" };
    const score = structuralScore({ template, output, original });
    expect(score).toBeLessThanOrEqual(7);
  });

  it("penalises TODO inflation", () => {
    const template = "## Features\n## Status\n";
    const output = { "01.md": "TODO TODO TODO TODO TODO TODO TODO\n## Features\n## Status\n", "README.md": "# R" };
    const original = { "01-core.md": "auth" }; // 0 TODOs originally
    const score = structuralScore({ template, output, original });
    expect(score).toBeLessThan(10);
  });
});
