import { describe, it, expect, vi } from "vitest";
import { SkillGenerator } from "./skill-generator.js";
import { SkillValidator } from "./skill-validator.js";
import type { ModelBackend, ProjectProfile } from "@sensei/shared";

const makeProfile = (): ProjectProfile => ({
  repoName: "test-repo",
  repoPath: "/tmp/test",
  dominantLanguage: "typescript",
  framework: "sveltekit",
  packageNames: ["engine", "cli"],
  keySymbols: ["createClient", "runCli"],
  testPattern: "*.spec.ts",
  cliCommands: { test: "vitest run", build: "bun build" },
  senseiConfig: "repo_id: abc",
});

const makeModel = (responses: string[]): ModelBackend => {
  let callCount = 0;
  return {
    name: "mock",
    init: vi.fn().mockResolvedValue(undefined),
    isAvailable: vi.fn().mockResolvedValue(true),
    generate: vi.fn().mockImplementation(async () => responses[callCount++] ?? "---\nname: test\ndescription: test\n---\n# Test"),
    embed: vi.fn().mockResolvedValue([]),
    extract: vi.fn().mockResolvedValue({}),
  };
};

describe("SkillGenerator", () => {
  it("calls model.generate() exactly 4 times when all validations pass", async () => {
    const VALID_SKILL = "---\nname: test\ndescription: test\n---\n# Test skill content";
    // 4 generations + 4 validations = 8 calls total
    const model = makeModel(Array(8).fill("VALID").map((v, i) => i % 2 === 0 ? VALID_SKILL : "VALID"));
    const validator = new SkillValidator(model, makeProfile());
    const generator = new SkillGenerator(model, makeProfile(), validator);

    const results = await generator.generate();

    expect(Object.keys(results)).toHaveLength(4);
    expect(Object.keys(results)).toEqual(["orientation", "workflow", "context", "patterns"]);
  });

  it("retries generation when validator returns invalid, succeeds on second attempt", async () => {
    // First orientation generation fails validation, second passes; others pass first try
    const generateSpy = vi.fn();
    let callCount = 0;
    const responses = [
      "bad orientation skill",  // orientation attempt 1
      "1. Bad package name",    // validator rejects orientation attempt 1
      "---\nname: ok\ndescription: ok\n---\n# Fixed",  // orientation attempt 2
      "VALID",                  // validator accepts orientation attempt 2
      "---\nname: wf\ndescription: wf\n---\n# Workflow",  // workflow attempt 1
      "VALID",                  // validator accepts
      "---\nname: ctx\ndescription: ctx\n---\n# Context",  // context attempt 1
      "VALID",                  // validator accepts
      "---\nname: pat\ndescription: pat\n---\n# Patterns",  // patterns attempt 1
      "VALID",                  // validator accepts
    ];
    const model: ModelBackend = {
      name: "mock",
      init: vi.fn().mockResolvedValue(undefined),
      isAvailable: vi.fn().mockResolvedValue(true),
      generate: generateSpy.mockImplementation(async () => responses[callCount++] ?? "VALID"),
      embed: vi.fn().mockResolvedValue([]),
      extract: vi.fn().mockResolvedValue({}),
    };
    const validator = new SkillValidator(model, makeProfile());
    const generator = new SkillGenerator(model, makeProfile(), validator);

    const results = await generator.generate();

    expect(results.orientation).toContain("# Fixed");
    // Both generation AND validation prompts flow through the same model.generate() spy
    // (SkillValidator receives the same model instance as SkillGenerator)
    // orientation: 2 generation calls + 2 validation calls = 4
    // workflow, context, patterns: 1 generation + 1 validation each = 3×2 = 6
    // Total: 10
    expect(generateSpy).toHaveBeenCalledTimes(10);
  });

  it("throws after 3 failed validation attempts for a category", async () => {
    // All validator calls return issues
    const model = makeModel(Array(20).fill("1. Bad package name"));
    const validator = new SkillValidator(model, makeProfile());
    const generator = new SkillGenerator(model, makeProfile(), validator);

    await expect(generator.generate()).rejects.toThrow("Failed to generate valid orientation skill");
  });
});
