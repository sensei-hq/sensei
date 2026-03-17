// packages/engine/src/lib/lib-skill-generator.spec.ts
import { describe, it, expect, vi } from "vitest";
import { LibSkillGenerator } from "./lib-skill-generator.js";
import { SkillValidator } from "../skill-gen/skill-validator.js";
import type { ModelBackend, ProjectProfile, LibEntry, DocPage } from "@sensei/shared";

const VALID_SKILL = "---\nname: myrepo-lib-rokkit\ndescription: Use when using rokkit\n---\n# Rokkit Guide";
const INVALID = "1. Package name wrong";

const makeProfile = (): ProjectProfile => ({
  repoName: "my-repo",
  repoPath: "/tmp/repo",
  dominantLanguage: "typescript",
  framework: "sveltekit",
  packageNames: ["engine"],
  keySymbols: ["createClient"],
  testPattern: "*.spec.ts",
  cliCommands: { test: "bun test" },
  senseiConfig: "repo_id: abc",
});

describe("LibSkillGenerator", () => {
  it("calls generate() once when validator passes on first attempt", async () => {
    const generateSpy = vi.fn()
      .mockResolvedValueOnce(VALID_SKILL)  // generation
      .mockResolvedValueOnce("VALID");     // validation

    const model: ModelBackend = {
      name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
      generate: generateSpy, embed: vi.fn().mockResolvedValue([]), extract: vi.fn().mockResolvedValue({}),
    };

    const validator = new SkillValidator(model, makeProfile());
    const generator = new LibSkillGenerator(model, makeProfile(), validator);
    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" };
    const pages: DocPage[] = [{ title: "Button", summary: "A button", content: "", sourceType: "llms.txt" }];

    const result = await generator.generate(entry, pages);

    expect(result).toBe(VALID_SKILL);
    expect(generateSpy).toHaveBeenCalledTimes(2); // 1 generation + 1 validation
  });

  it("retries on invalid and succeeds on second attempt", async () => {
    const generateSpy = vi.fn()
      .mockResolvedValueOnce("bad skill")  // gen 1
      .mockResolvedValueOnce(INVALID)      // validator rejects
      .mockResolvedValueOnce(VALID_SKILL)  // gen 2
      .mockResolvedValueOnce("VALID");     // validator accepts

    const model: ModelBackend = {
      name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
      generate: generateSpy, embed: vi.fn().mockResolvedValue([]), extract: vi.fn().mockResolvedValue({}),
    };

    const generator = new LibSkillGenerator(model, makeProfile(), new SkillValidator(model, makeProfile()));
    const result = await generator.generate({ name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" }, []);
    expect(result).toBe(VALID_SKILL);
  });

  it("throws after 3 failed attempts", async () => {
    const model: ModelBackend = {
      name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
      generate: vi.fn().mockResolvedValue(INVALID), embed: vi.fn().mockResolvedValue([]), extract: vi.fn().mockResolvedValue({}),
    };
    const generator = new LibSkillGenerator(model, makeProfile(), new SkillValidator(model, makeProfile()));
    await expect(generator.generate({ name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" }, [])).rejects.toThrow("3 attempts");
  });
});
