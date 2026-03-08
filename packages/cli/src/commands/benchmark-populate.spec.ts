import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { buildPopulatePrompt, parseYamlOutput, parsePopulateScore } from "./benchmark-populate.js";

const TMP = "/tmp/sensei-populate-test";

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
  writeFileSync(join(TMP, "README.md"), "# Sensei\n\nA CLI toolchain for AI skills.\n\nMore details here.\n");
  writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({
    "packages/cli/src/cli.ts": { L0: ["export async function main"], L1: ["CLI entry point"], L2: [] },
    "packages/cli/src/cli.spec.ts": { L0: ["describe"], L1: ["tests"], L2: [] },
    "docs/design/01-overview.md": { L0: ["# Overview", "## Architecture"], L1: [], L2: [] },
    "docs/plans/2026-plan.md": { L0: ["# Plan"], L1: [], L2: [] },
  }));
  writeFileSync(join(TMP, ".sensei/llmspec.yaml"), [
    "project: sensei",
    "description: TODO: one-sentence summary",
    "concepts: []",
    "patterns: []",
    "docs:",
    "  - path: docs/design/01-overview.md",
    "    covers: []",
    "  - path: docs/plans/2026-plan.md",
    "    covers: []",
  ].join("\n"));
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("buildPopulatePrompt", () => {
  it("includes README content", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("A CLI toolchain for AI skills");
  });

  it("includes source files but excludes .spec. files", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("packages/cli/src/cli.ts");
    expect(prompt).not.toContain("packages/cli/src/cli.spec.ts");
  });

  it("includes non-plan doc paths", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("docs/design/01-overview.md");
  });

  it("excludes docs/plans/ from doc headings section", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).not.toContain("docs/plans/2026-plan.md");
  });

  it("includes current llmspec skeleton", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("TODO: one-sentence summary");
  });

  it("does NOT include skill section when skillContent is null", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).not.toContain("populate-llmspec skill");
  });

  it("prepends skill content when skillContent is provided", () => {
    const prompt = buildPopulatePrompt(TMP, "## populate-llmspec skill protocol\nStep 1: do this");
    expect(prompt).toContain("populate-llmspec skill protocol");
    // Skill section should appear before the repo context
    const skillIdx = prompt.indexOf("populate-llmspec skill protocol");
    const readmeIdx = prompt.indexOf("A CLI toolchain");
    expect(skillIdx).toBeLessThan(readmeIdx);
  });

  it("does not leave orphan covers lines when docs/plans entry is stripped", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    const skeletonStart = prompt.indexOf("## Current llmspec.yaml skeleton");
    const instructionsStart = prompt.indexOf("## Instructions");
    // Extract only the YAML content between the skeleton header and the instructions
    const skeletonSection = prompt.slice(skeletonStart, instructionsStart);
    // Only one covers: line should remain (for docs/design entry)
    const coverCount = (skeletonSection.match(/covers:/g) ?? []).length;
    expect(coverCount).toBe(1);
  });
});

describe("parseYamlOutput", () => {
  it("returns raw YAML when no code fences", () => {
    const raw = "project: foo\ndescription: bar";
    expect(parseYamlOutput(raw)).toBe(raw);
  });

  it("strips ```yaml ... ``` code fences", () => {
    const raw = "```yaml\nproject: foo\ndescription: bar\n```";
    expect(parseYamlOutput(raw)).toBe("project: foo\ndescription: bar");
  });

  it("strips ``` ... ``` code fences without language tag", () => {
    const raw = "```\nproject: foo\n```";
    expect(parseYamlOutput(raw)).toBe("project: foo");
  });

  it("strips leading/trailing prose if yaml block is present", () => {
    const raw = "Here is the YAML:\n```yaml\nproject: foo\n```\nDone.";
    expect(parseYamlOutput(raw)).toBe("project: foo");
  });
});

describe("parsePopulateScore", () => {
  it("extracts score from score-coverage.ts output", () => {
    const output = "\nllmspec coverage score: 87/100\n  description:   ✓ (10pts)\n";
    expect(parsePopulateScore(output)).toBe(87);
  });

  it("returns 0 when pattern not found", () => {
    expect(parsePopulateScore("no score here")).toBe(0);
  });

  it("returns 100 for perfect score", () => {
    const output = "llmspec coverage score: 100/100";
    expect(parsePopulateScore(output)).toBe(100);
  });
});
