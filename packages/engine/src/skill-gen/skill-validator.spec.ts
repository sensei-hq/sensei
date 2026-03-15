import { describe, it, expect, vi } from "vitest";
import { SkillValidator } from "./skill-validator.js";
import type { ModelBackend, ProjectProfile } from "@sensei/shared";

const makeProfile = (): ProjectProfile => ({
  repoName: "test-repo",
  repoPath: "/tmp/test",
  dominantLanguage: "typescript",
  framework: null,
  packageNames: ["engine", "cli"],
  keySymbols: ["createClient"],
  testPattern: "*.spec.ts",
  cliCommands: { test: "vitest run" },
  senseiConfig: "",
});

const makeModel = (response: string): ModelBackend => ({
  name: "mock",
  init: vi.fn().mockResolvedValue(undefined),
  isAvailable: vi.fn().mockResolvedValue(true),
  generate: vi.fn().mockResolvedValue(response),
  embed: vi.fn().mockResolvedValue([]),
  extract: vi.fn().mockResolvedValue({}),
});

describe("SkillValidator", () => {
  it("returns valid:true when model returns VALID", async () => {
    const validator = new SkillValidator(makeModel("VALID"), makeProfile());
    const result = await validator.validate("orientation", "---\nname: test\ndescription: test\n---\n# Test");
    expect(result.valid).toBe(true);
    expect(result.issues).toHaveLength(0);
  });

  it("returns valid:false with parsed issues when model returns numbered list", async () => {
    const response = "1. Package name 'wrong-pkg' not in profile\n2. Command 'yarn test' not found";
    const validator = new SkillValidator(makeModel(response), makeProfile());
    const result = await validator.validate("orientation", "---\nname: test\n---");
    expect(result.valid).toBe(false);
    expect(result.issues).toHaveLength(2);
    expect(result.issues[0]).toContain("Package name");
  });

  it("returns valid:false with raw text when model returns non-list", async () => {
    const validator = new SkillValidator(makeModel("Something is wrong"), makeProfile());
    const result = await validator.validate("orientation", "---\nname: test\n---");
    expect(result.valid).toBe(false);
    expect(result.issues).toEqual(["Something is wrong"]);
  });
});
