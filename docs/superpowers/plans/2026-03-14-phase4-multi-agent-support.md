# Phase 4: Multi-Agent Support — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `sensei setup --agent claude` generates four project-specific skill files using Claude, validates them, writes them to `~/.claude/skills/`, and records metadata in `.sensei/agent-skills.json`; status is visible in a dashboard Agent Skills view.

**Architecture:** `ProjectProfile` is extracted from the Supabase symbol index + `package.json`. `ClaudeBackend` implements `ModelBackend` using `@anthropic-ai/sdk`. `SkillGenerator` produces 4 skill categories, `SkillValidator` reviews each with Claude, retrying up to 2 times on failure. `ClaudeAdapter` writes skill files to `~/.claude/skills/`.

**Tech Stack:** TypeScript, Bun, Vitest, `@anthropic-ai/sdk`, `@supabase/supabase-js`, `@clack/prompts`, SvelteKit

---

## Chunk 1: Foundations

### Task 1: Add types to shared

**Files:**
- Modify: `packages/shared/src/types.ts` (append after line 105)

- [ ] **Step 1: Add the three new types**

Open `packages/shared/src/types.ts` and append at the end:

```typescript
// ─── Agent skill generation types ────────────────────────────────────────────

export interface ProjectProfile {
  repoName: string;
  repoPath: string;
  dominantLanguage: string;             // 'typescript' | 'python' | etc.
  framework: string | null;             // 'sveltekit' | 'react' | 'express' | null
  packageNames: string[];               // monorepo packages e.g. ['engine', 'cli']
  keySymbols: string[];                 // top 20 most-referenced exported symbols
  testPattern: string;                  // e.g. '*.spec.ts'
  cliCommands: Record<string, string>;  // from package.json scripts
  senseiConfig: string;                 // serialised .sensei/config.yaml content
}

export interface AgentSkillFile {
  category: 'orientation' | 'workflow' | 'context' | 'patterns';
  path: string;          // absolute path to written skill file
  generatedAt: string;   // ISO timestamp — new Date().toISOString()
}

export interface AgentSkillsManifest {
  agent: 'claude';
  repoSlug: string;
  skills: AgentSkillFile[];
  updatedAt: string;     // ISO timestamp
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd /Users/Jerry/Developer/sensei && bunx tsc -p packages/shared/tsconfig.json --noEmit
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add packages/shared/src/types.ts
git commit -m "feat(shared): add ProjectProfile, AgentSkillFile, AgentSkillsManifest types"
```

---

### Task 2: ClaudeBackend

**Files:**
- Create: `packages/server/src/model/claude-backend.ts`
- Create: `packages/server/src/model/claude-backend.spec.ts`
- Modify: `packages/server/package.json` (add `@anthropic-ai/sdk`)
- Modify: `packages/server/src/index.ts` (export ClaudeBackend)

- [ ] **Step 1: Add `@anthropic-ai/sdk` to server package**

```bash
cd /Users/Jerry/Developer/sensei && bun add @anthropic-ai/sdk --cwd packages/server
```

Expected: `packages/server/package.json` now lists `"@anthropic-ai/sdk": "^0.78.0"` (or latest) in dependencies.

- [ ] **Step 2: Write the failing tests**

Create `packages/server/src/model/claude-backend.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

describe("ClaudeBackend", () => {
  const originalKey = process.env.ANTHROPIC_API_KEY;

  afterEach(() => {
    if (originalKey === undefined) delete process.env.ANTHROPIC_API_KEY;
    else process.env.ANTHROPIC_API_KEY = originalKey;
  });

  it("init() throws when ANTHROPIC_API_KEY is absent", async () => {
    delete process.env.ANTHROPIC_API_KEY;
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    await expect(backend.init()).rejects.toThrow("ANTHROPIC_API_KEY");
  });

  it("isAvailable() returns false when key absent", async () => {
    delete process.env.ANTHROPIC_API_KEY;
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    expect(await backend.isAvailable()).toBe(false);
  });

  it("isAvailable() returns true when key present", async () => {
    process.env.ANTHROPIC_API_KEY = "test-key";
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    expect(await backend.isAvailable()).toBe(true);
  });

  it("embed() throws NotImplementedError", async () => {
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    await expect(backend.embed("text")).rejects.toThrow("does not support embed");
  });

  it("extract() throws NotImplementedError", async () => {
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    await expect(backend.extract("content", { filePath: "foo.ts" })).rejects.toThrow("does not support extract");
  });

  it("generate() before init() throws", async () => {
    process.env.ANTHROPIC_API_KEY = "test-key";
    const { ClaudeBackend } = await import("./claude-backend.js");
    const backend = new ClaudeBackend();
    // client is null until init() — should throw
    await expect(backend.generate("hello")).rejects.toThrow("not initialized");
  });
});
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/server/src/model/claude-backend.spec.ts
```

Expected: FAIL — "Cannot find module './claude-backend.js'"

- [ ] **Step 4: Implement ClaudeBackend**

Create `packages/server/src/model/claude-backend.ts`:

```typescript
import Anthropic from "@anthropic-ai/sdk";
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "@sensei/shared";

export interface ClaudeBackendOptions {
  model?: string;  // default: 'claude-sonnet-4-6'
}

export class ClaudeBackend implements ModelBackend {
  readonly name = "claude";
  private client: Anthropic | null = null;
  private readonly model: string;

  constructor(opts: ClaudeBackendOptions = {}) {
    this.model = opts.model ?? "claude-sonnet-4-6";
  }

  async init(): Promise<void> {
    const key = process.env.ANTHROPIC_API_KEY;
    if (!key) throw new Error("ANTHROPIC_API_KEY is not set — export it before running sensei setup --agent claude");
    this.client = new Anthropic({ apiKey: key });
  }

  async isAvailable(): Promise<boolean> {
    return Boolean(process.env.ANTHROPIC_API_KEY);
  }

  async generate(prompt: string): Promise<string> {
    if (!this.client) throw new Error("ClaudeBackend not initialized — call init() first");
    const message = await this.client.messages.create({
      model: this.model,
      max_tokens: 4096,
      messages: [{ role: "user", content: prompt }],
    });
    const block = message.content[0];
    if (!block || block.type !== "text") return "";
    return block.text;
  }

  async embed(_text: string): Promise<number[]> {
    throw new Error("ClaudeBackend does not support embed");
  }

  async extract(_content: string, _instructions: ExtractionInstructions): Promise<FileAnalysis> {
    throw new Error("ClaudeBackend does not support extract");
  }
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/server/src/model/claude-backend.spec.ts
```

Expected: 6/6 passing

- [ ] **Step 6: Export from server index**

Add to `packages/server/src/index.ts` after the OllamaBackend export line:

```typescript
export { ClaudeBackend } from "./model/claude-backend.js";
export type { ClaudeBackendOptions } from "./model/claude-backend.js";
```

- [ ] **Step 7: Run full server tests**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/server
```

Expected: all passing

- [ ] **Step 8: Commit**

```bash
git add packages/server/src/model/claude-backend.ts packages/server/src/model/claude-backend.spec.ts packages/server/src/index.ts packages/server/package.json bun.lock
git commit -m "feat(server): add ClaudeBackend implementing ModelBackend via @anthropic-ai/sdk"
```

---

## Chunk 2: Engine Layer

### Task 3: extractProjectProfile

**Files:**
- Create: `packages/engine/src/project-profile.ts`
- Create: `packages/engine/src/project-profile.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/engine/src/project-profile.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { join } from "path";
import { mkdtemp, writeFile, mkdir } from "fs/promises";
import { tmpdir } from "os";
import { extractProjectProfile } from "./project-profile.js";

function makeDb(opts: { repoError?: boolean; symbolRows?: Array<{ name: string; file_path: string }> } = {}) {
  return {
    from: (table: string) => ({
      select: () => ({
        eq: (_col: string, _val: string) => ({
          single: async () => {
            if (table === "repos") {
              if (opts.repoError) return { data: null, error: { message: "DB error" } };
              return { data: { name: "my-repo" }, error: null };
            }
            return { data: null, error: null };
          },
          limit: async () => {
            if (table === "symbols") return { data: opts.symbolRows ?? [], error: null };
            return { data: [], error: null };
          },
        }),
      }),
    }),
  };
}

describe("extractProjectProfile", () => {
  it("returns correct dominantLanguage and keySymbols from fixture symbols", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await mkdir(join(tmpDir, ".sensei"), { recursive: true });
    await writeFile(
      join(tmpDir, "package.json"),
      JSON.stringify({ name: "my-repo", scripts: { test: "vitest run" }, devDependencies: { vitest: "^4.0.0" } }),
      "utf-8"
    );

    const symbolRows = [
      { name: "createClient", file_path: "packages/shared/src/client.ts" },
      { name: "indexRepo", file_path: "packages/engine/src/indexer.ts" },
      { name: "runCli", file_path: "packages/cli/src/cli.ts" },
    ];
    const db = makeDb({ symbolRows });

    const profile = await extractProjectProfile(db as any, "repo-id", tmpDir);

    expect(profile.dominantLanguage).toBe("typescript");
    expect(profile.keySymbols).toContain("createClient");
    expect(profile.keySymbols).toContain("indexRepo");
    expect(profile.testPattern).toBe("*.spec.ts");
  });

  it("throws when package.json is missing", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    const db = makeDb();
    await expect(extractProjectProfile(db as any, "repo-id", tmpDir)).rejects.toThrow("package.json not found");
  });

  it("throws on DB error fetching repo", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await writeFile(join(tmpDir, "package.json"), JSON.stringify({ name: "x" }), "utf-8");
    const db = makeDb({ repoError: true });
    await expect(extractProjectProfile(db as any, "repo-id", tmpDir)).rejects.toThrow("DB error");
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/project-profile.spec.ts
```

Expected: FAIL — "Cannot find module './project-profile.js'"

- [ ] **Step 3: Implement extractProjectProfile**

Create `packages/engine/src/project-profile.ts`:

```typescript
import { readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ProjectProfile } from "@sensei/shared";

const EXT_TO_LANGUAGE: Record<string, string> = {
  ts: "typescript", tsx: "typescript",
  js: "javascript", jsx: "javascript",
  py: "python", go: "go", rs: "rust", rb: "ruby", java: "java",
};

export async function extractProjectProfile(
  db: SupabaseClient,
  repoId: string,
  repoPath: string,
): Promise<ProjectProfile> {
  // 1. Fetch repo name
  const { data: repoRow, error: repoError } = await db
    .from("repos")
    .select("name")
    .eq("id", repoId)
    .single();
  if (repoError || !repoRow) throw new Error(repoError?.message ?? "Repo not found");

  // 2. Fetch symbols for language detection + key symbol names
  const { data: symbolRows, error: symbolError } = await db
    .from("symbols")
    .select("name,file_path")
    .eq("repo_id", repoId)
    .limit(100);
  if (symbolError) throw new Error(symbolError.message);

  const symbols = (symbolRows ?? []) as Array<{ name: string; file_path: string }>;

  // Dominant language from file extension frequency
  const extCounts: Record<string, number> = {};
  for (const sym of symbols) {
    const ext = sym.file_path.split(".").at(-1) ?? "unknown";
    extCounts[ext] = (extCounts[ext] ?? 0) + 1;
  }
  const topExt = Object.entries(extCounts).sort((a, b) => b[1] - a[1])[0]?.[0] ?? "unknown";
  const dominantLanguage = EXT_TO_LANGUAGE[topExt] ?? topExt;

  const keySymbols = symbols.slice(0, 20).map(s => s.name);

  // 3. Read package.json
  const pkgPath = join(repoPath, "package.json");
  if (!existsSync(pkgPath)) throw new Error(`package.json not found at ${pkgPath}`);
  const pkg = JSON.parse(await readFile(pkgPath, "utf-8")) as {
    scripts?: Record<string, string>;
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    workspaces?: string[] | { packages: string[] };
  };

  const cliCommands = pkg.scripts ?? {};
  const allDeps = { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };

  // Framework detection
  let framework: string | null = null;
  if ("@sveltejs/kit" in allDeps) framework = "sveltekit";
  else if ("react" in allDeps) framework = "react";
  else if ("vue" in allDeps) framework = "vue";
  else if ("express" in allDeps) framework = "express";
  else if ("fastify" in allDeps) framework = "fastify";

  // Package names from workspaces
  let packageNames: string[] = [];
  if (pkg.workspaces) {
    const ws = Array.isArray(pkg.workspaces) ? pkg.workspaces : pkg.workspaces.packages;
    packageNames = ws.map(p => p.replace(/^packages\//, "").replace(/\/\*$/, ""));
  }

  // Test pattern
  const testPattern = "vitest" in allDeps ? "*.spec.ts" : "jest" in allDeps ? "*.test.ts" : "*.spec.ts";

  // 4. Read .sensei/config.yaml (best-effort)
  let senseiConfig = "";
  const configPath = join(repoPath, ".sensei", "config.yaml");
  if (existsSync(configPath)) {
    senseiConfig = await readFile(configPath, "utf-8");
  }

  return {
    repoName: repoRow.name as string,
    repoPath,
    dominantLanguage,
    framework,
    packageNames,
    keySymbols,
    testPattern,
    cliCommands,
    senseiConfig,
  };
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/project-profile.spec.ts
```

Expected: 3/3 passing

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/project-profile.ts packages/engine/src/project-profile.spec.ts
git commit -m "feat(engine): add extractProjectProfile from Supabase index + package.json"
```

---

### Task 4: SkillValidator

**Files:**
- Create: `packages/engine/src/skill-gen/skill-validator.ts`
- Create: `packages/engine/src/skill-gen/skill-validator.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/engine/src/skill-gen/skill-validator.spec.ts`:

```typescript
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/skill-gen/skill-validator.spec.ts
```

Expected: FAIL — "Cannot find module './skill-validator.js'"

- [ ] **Step 3: Implement SkillValidator**

Create `packages/engine/src/skill-gen/skill-validator.ts`:

```typescript
import type { ModelBackend, ProjectProfile } from "@sensei/shared";

export class SkillValidator {
  constructor(
    private readonly model: ModelBackend,
    private readonly profile: ProjectProfile,
  ) {}

  async validate(
    category: string,
    skillMarkdown: string,
  ): Promise<{ valid: boolean; issues: string[] }> {
    const prompt = `You are reviewing a generated skill file for accuracy.

Project profile:
- Repo: ${this.profile.repoName}
- Packages: ${this.profile.packageNames.join(", ")}
- Key symbols: ${this.profile.keySymbols.slice(0, 10).join(", ")}
- CLI commands available: ${Object.keys(this.profile.cliCommands).join(", ")}
- Test pattern: ${this.profile.testPattern}

Skill category: ${category}

Generated skill:
---
${skillMarkdown}
---

Check this skill for accuracy. Verify:
1. Package names mentioned match the project profile
2. Commands referenced are real (exist in the profile's CLI commands)
3. No hallucinated file paths
4. YAML frontmatter is present with name and description fields

Reply with exactly one of:
- "VALID" if all checks pass
- A numbered list of issues (e.g. "1. Command 'yarn test' not found") if any checks fail`;

    const response = await this.model.generate(prompt);
    const trimmed = response.trim();

    if (trimmed === "VALID" || trimmed.startsWith("VALID")) {
      return { valid: true, issues: [] };
    }

    const issues = trimmed
      .split("\n")
      .filter(line => /^\d+\./.test(line.trim()))
      .map(line => line.trim());

    return {
      valid: false,
      issues: issues.length > 0 ? issues : [trimmed],
    };
  }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/skill-gen/skill-validator.spec.ts
```

Expected: 3/3 passing

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/skill-gen/skill-validator.ts packages/engine/src/skill-gen/skill-validator.spec.ts
git commit -m "feat(engine): add SkillValidator — Claude-based skill review with numbered issue parsing"
```

---

### Task 5: SkillGenerator

**Files:**
- Create: `packages/engine/src/skill-gen/skill-generator.ts`
- Create: `packages/engine/src/skill-gen/skill-generator.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/engine/src/skill-gen/skill-generator.spec.ts`:

```typescript
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/skill-gen/skill-generator.spec.ts
```

Expected: FAIL — "Cannot find module './skill-generator.js'"

- [ ] **Step 3: Implement SkillGenerator**

Create `packages/engine/src/skill-gen/skill-generator.ts`:

```typescript
import type { ModelBackend, ProjectProfile } from "@sensei/shared";
import type { SkillValidator } from "./skill-validator.js";

type SkillCategory = "orientation" | "workflow" | "context" | "patterns";

function buildPrompt(category: SkillCategory, profile: ProjectProfile): string {
  const slug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");

  if (category === "orientation") {
    return `Generate a skill file in SKILL.md format for the category "orientation".

Project: ${profile.repoName}
Language: ${profile.dominantLanguage}
Framework: ${profile.framework ?? "none"}
Packages: ${profile.packageNames.join(", ")}
Key exported symbols: ${profile.keySymbols.slice(0, 15).join(", ")}

The skill helps an AI agent orient themselves in this codebase. Include:
- What the project does
- Key packages and their responsibilities
- Most important symbols to know
- Where to start when beginning a new task

Output exactly this format and nothing else:

---
name: ${slug}-orientation
description: Use when starting work in the ${profile.repoName} repo to understand its structure and key components.
---

# ${profile.repoName} Orientation

[skill content here — 150-300 words, specific to this project]`;
  }

  if (category === "workflow") {
    const cmdList = Object.entries(profile.cliCommands)
      .slice(0, 10)
      .map(([k, v]) => `  ${k}: ${v}`)
      .join("\n");
    return `Generate a skill file in SKILL.md format for the category "workflow".

Project: ${profile.repoName}
Test pattern: ${profile.testPattern}
CLI commands:
${cmdList}
Sensei config:
${profile.senseiConfig.slice(0, 400)}

The skill teaches an AI agent the correct development workflow. Include:
- How to run tests (exact command)
- How to build/start the project
- Key CLI commands and when to use them
- TDD workflow for this specific project

Output exactly this format and nothing else:

---
name: ${slug}-workflow
description: Use when performing development tasks in ${profile.repoName} to follow the correct workflow.
---

# ${profile.repoName} Workflow

[skill content here — 150-300 words, specific to this project]`;
  }

  if (category === "context") {
    return `Generate a skill file in SKILL.md format for the category "context".

Project: ${profile.repoName}
Packages: ${profile.packageNames.join(", ")}
Key symbols: ${profile.keySymbols.join(", ")}
Sensei MCP tools available: get_session_context, search, load_context, context_pack

The skill teaches when and how to use sensei's context tools. Include:
- When to call get_session_context vs context_pack
- How to search for relevant code
- Token budget guidance
- Which key symbols to load for common task types in this project

Output exactly this format and nothing else:

---
name: ${slug}-context
description: Use when deciding how to load context for a task in ${profile.repoName}.
---

# ${profile.repoName} Context Loading

[skill content here — 150-300 words, specific to this project]`;
  }

  // patterns
  return `Generate a skill file in SKILL.md format for the category "patterns".

Project: ${profile.repoName}
Language: ${profile.dominantLanguage}
Framework: ${profile.framework ?? "none"}
Test pattern: ${profile.testPattern}
Available scripts: ${Object.keys(profile.cliCommands).join(", ")}

The skill documents coding conventions for this project. Include:
- File naming conventions
- Testing conventions (based on ${profile.testPattern})
- Error handling patterns
- Import/export patterns
- Framework-specific conventions for ${profile.framework ?? "this project"}

Output exactly this format and nothing else:

---
name: ${slug}-patterns
description: Use when writing code in ${profile.repoName} to follow established patterns and conventions.
---

# ${profile.repoName} Patterns

[skill content here — 150-300 words, specific to this project]`;
}

export class SkillGenerator {
  constructor(
    private readonly model: ModelBackend,
    private readonly profile: ProjectProfile,
    private readonly validator: SkillValidator,
  ) {}

  async generate(): Promise<Record<SkillCategory, string>> {
    const categories: SkillCategory[] = ["orientation", "workflow", "context", "patterns"];
    const results: Partial<Record<SkillCategory, string>> = {};

    for (const category of categories) {
      let skillMarkdown = await this.model.generate(buildPrompt(category, this.profile));

      for (let attempt = 0; attempt < 3; attempt++) {
        const { valid, issues } = await this.validator.validate(category, skillMarkdown);
        if (valid) break;

        if (attempt === 2) {
          throw new Error(
            `Failed to generate valid ${category} skill after 3 attempts. Issues: ${issues.join("; ")}`,
          );
        }

        // Retry with validator feedback
        const retryPrompt =
          buildPrompt(category, this.profile) +
          `\n\nPrevious attempt was rejected. Issues to fix:\n${issues.join("\n")}\nPlease address these issues in your response.`;
        skillMarkdown = await this.model.generate(retryPrompt);
      }

      results[category] = skillMarkdown;
    }

    return results as Record<SkillCategory, string>;
  }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/skill-gen/skill-generator.spec.ts
```

Expected: 3/3 passing

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/skill-gen/skill-generator.ts packages/engine/src/skill-gen/skill-generator.spec.ts
git commit -m "feat(engine): add SkillGenerator with validate-retry loop (max 3 attempts per category)"
```

---

### Task 6: AgentAdapter interface + ClaudeAdapter

**Files:**
- Create: `packages/engine/src/agent/agent-adapter.ts`
- Create: `packages/engine/src/agent/claude-adapter.ts`
- Create: `packages/engine/src/agent/claude-adapter.spec.ts`

- [ ] **Step 1: Create AgentAdapter interface**

Create `packages/engine/src/agent/agent-adapter.ts`:

```typescript
import type { AgentSkillFile } from "@sensei/shared";

export interface AgentAdapter {
  /** Absolute path to the agent's skills directory */
  readonly skillsDir: string;

  /** Write skill markdown files for each category; returns AgentSkillFile[] with paths + timestamps */
  writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]>;

  /** List skill files already written for this repo slug */
  installedSkills(repoSlug: string): Promise<AgentSkillFile[]>;
}
```

- [ ] **Step 2: Write failing tests for ClaudeAdapter**

Create `packages/engine/src/agent/claude-adapter.spec.ts`:

```typescript
import { describe, it, expect, afterEach } from "vitest";
import { mkdtemp, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { ClaudeAdapter } from "./claude-adapter.js";

describe("ClaudeAdapter", () => {
  let tmpDir: string;

  afterEach(async () => {
    if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
  });

  it("writeSkills creates correct filenames and returns AgentSkillFile[]", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-adapter-test-"));
    const adapter = new ClaudeAdapter(tmpDir); // inject skillsDir for testing

    const skills = {
      orientation: "---\nname: test\ndescription: test\n---\n# Orientation",
      workflow: "---\nname: wf\ndescription: wf\n---\n# Workflow",
      context: "---\nname: ctx\ndescription: ctx\n---\n# Context",
      patterns: "---\nname: pat\ndescription: pat\n---\n# Patterns",
    };

    const result = await adapter.writeSkills(skills, "my-repo");

    expect(result).toHaveLength(4);
    expect(result.map(f => f.category).sort()).toEqual(["context", "orientation", "patterns", "workflow"]);
    expect(result[0].path).toContain("sensei-my-repo-");
    expect(result[0].generatedAt).toBeTruthy();
  });

  it("installedSkills returns only files matching the repo slug prefix", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-adapter-test-"));
    const adapter = new ClaudeAdapter(tmpDir);

    // Write skills for two different repos
    await adapter.writeSkills({ orientation: "# A" }, "repo-a");
    await adapter.writeSkills({ orientation: "# B" }, "repo-b");

    const repoASkills = await adapter.installedSkills("repo-a");
    expect(repoASkills).toHaveLength(1);
    expect(repoASkills[0].category).toBe("orientation");

    const repoBSkills = await adapter.installedSkills("repo-b");
    expect(repoBSkills).toHaveLength(1);
  });

  it("installedSkills returns empty array when directory does not exist", async () => {
    const adapter = new ClaudeAdapter("/nonexistent/path/skills");
    const result = await adapter.installedSkills("any-repo");
    expect(result).toEqual([]);
  });
});
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/agent/claude-adapter.spec.ts
```

Expected: FAIL — "Cannot find module './claude-adapter.js'"

- [ ] **Step 4: Implement ClaudeAdapter**

Create `packages/engine/src/agent/claude-adapter.ts`:

```typescript
import { homedir } from "os";
import { join } from "path";
import { mkdir, writeFile, readdir, stat } from "fs/promises";
import { existsSync } from "fs";
import type { AgentSkillFile } from "@sensei/shared";
import type { AgentAdapter } from "./agent-adapter.js";

export class ClaudeAdapter implements AgentAdapter {
  readonly skillsDir: string;

  constructor(skillsDir?: string) {
    // Use injected path for testing; default to ~/.claude/skills/
    // Note: os.homedir() is used — never expand '~' manually, fs functions don't handle it
    this.skillsDir = skillsDir ?? join(homedir(), ".claude", "skills");
  }

  async writeSkills(skills: Record<string, string>, repoSlug: string): Promise<AgentSkillFile[]> {
    await mkdir(this.skillsDir, { recursive: true });
    const result: AgentSkillFile[] = [];

    for (const [category, markdown] of Object.entries(skills)) {
      const fileName = `sensei-${repoSlug}-${category}.md`;
      const filePath = join(this.skillsDir, fileName);
      await writeFile(filePath, markdown, "utf-8");
      result.push({
        category: category as AgentSkillFile["category"],
        path: filePath,
        generatedAt: new Date().toISOString(),
      });
    }

    return result;
  }

  async installedSkills(repoSlug: string): Promise<AgentSkillFile[]> {
    if (!existsSync(this.skillsDir)) return [];

    const entries = await readdir(this.skillsDir);
    const prefix = `sensei-${repoSlug}-`;
    const result: AgentSkillFile[] = [];

    for (const entry of entries) {
      if (!entry.startsWith(prefix) || !entry.endsWith(".md")) continue;
      const filePath = join(this.skillsDir, entry);
      const stats = await stat(filePath);
      const category = entry.slice(prefix.length, -".md".length) as AgentSkillFile["category"];
      result.push({ category, path: filePath, generatedAt: stats.mtime.toISOString() });
    }

    return result;
  }
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine/src/agent/claude-adapter.spec.ts
```

Expected: 3/3 passing

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/agent/agent-adapter.ts packages/engine/src/agent/claude-adapter.ts packages/engine/src/agent/claude-adapter.spec.ts
git commit -m "feat(engine): add AgentAdapter interface and ClaudeAdapter writing to ~/.claude/skills/"
```

---

### Task 7: Export engine public API

**Files:**
- Modify: `packages/engine/src/index.ts`

- [ ] **Step 1: Add exports**

Append to `packages/engine/src/index.ts`:

```typescript
export * from "./project-profile.js";
export * from "./skill-gen/skill-validator.js";
export * from "./skill-gen/skill-generator.js";
export * from "./agent/agent-adapter.js";
export * from "./agent/claude-adapter.js";
```

- [ ] **Step 2: Verify all engine tests still pass**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/engine
```

Expected: all passing

- [ ] **Step 3: Commit**

```bash
git add packages/engine/src/index.ts
git commit -m "feat(engine): export project-profile, skill-gen, and agent modules"
```

---

## Chunk 3: CLI + MCP

### Task 8: setupAgent CLI command

**Files:**
- Modify: `packages/cli/src/commands/setup.ts`

- [ ] **Step 1: Add setupAgent function**

Open `packages/cli/src/commands/setup.ts` and add after the existing `setupHooks` function:

```typescript
import { writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { spinner } from "@clack/prompts";
import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import { extractProjectProfile, SkillGenerator, SkillValidator, ClaudeAdapter } from "@sensei/engine";
import { ClaudeBackend } from "@sensei/server";
import type { AgentSkillsManifest } from "@sensei/shared";
```

Note: The existing `setup.ts` imports `{ intro, outro, log }` from `@clack/prompts` — `spinner` is NOT currently imported; add it. `writeFile`, `mkdir` are already imported from `"fs/promises"`. `join` is already imported from `"path"`. Add `spinner`, the `@sensei/engine` imports, `ClaudeBackend` from `@sensei/server`, and the `AgentSkillsManifest` type — these are all new.

Add this function at the end of the file:

```typescript
export async function setupAgent(repoPath: string, agent: string): Promise<void> {
  if (agent !== "claude") {
    console.error(`Agent '${agent}' is not yet supported. Supported: claude`);
    process.exit(1);
  }

  intro(`sensei setup --agent ${agent}`);

  // 1. Load config
  const [client, config] = await Promise.all([
    makeSenseiClient(repoPath),
    loadSenseiConfig(repoPath),
  ]);
  if (!client) throw new Error("Supabase client not configured. Run sensei init first.");
  if (!config?.repo_id) throw new Error("Repo not configured. Run sensei init first.");

  // 2. Extract project profile
  const s1 = spinner();
  s1.start("Analysing project...");
  const profile = await extractProjectProfile(client as any, config.repo_id, repoPath);
  s1.stop(`Project analysed: ${profile.dominantLanguage}, ${profile.packageNames.length} packages`);

  // 3. Init Claude backend — fails fast if ANTHROPIC_API_KEY missing
  const backend = new ClaudeBackend();
  await backend.init();

  // 4. Generate + validate all 4 skills
  const validator = new SkillValidator(backend, profile);
  const generator = new SkillGenerator(backend, profile, validator);

  const s2 = spinner();
  s2.start("Generating skills...");
  const skills = await generator.generate();
  s2.stop("Skills generated (4/4)");

  // 5. Write skill files to ~/.claude/skills/
  const adapter = new ClaudeAdapter();
  const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const written = await adapter.writeSkills(skills, repoSlug);

  // 6. Write manifest to .sensei/agent-skills.json
  const manifest: AgentSkillsManifest = {
    agent: "claude",
    repoSlug,
    skills: written,
    updatedAt: new Date().toISOString(),
  };
  const senseiDir = join(repoPath, ".sensei");
  await mkdir(senseiDir, { recursive: true });
  await writeFile(join(senseiDir, "agent-skills.json"), JSON.stringify(manifest, null, 2), "utf-8");

  log.success(`4 skill files written to ${adapter.skillsDir}`);
  written.forEach(f => log.info(`  ${f.path}`));
  outro("Done. Restart Claude Code to pick up the new skills.");
}
```

- [ ] **Step 2: Verify setup.ts compiles**

```bash
cd /Users/Jerry/Developer/sensei && bunx tsc -p packages/cli/tsconfig.json --noEmit 2>&1 | head -30
```

Expected: no errors (or only pre-existing errors unrelated to setup.ts)

- [ ] **Step 3: Commit**

```bash
git add packages/cli/src/commands/setup.ts
git commit -m "feat(cli): add setupAgent — generates and installs Claude skills via ClaudeBackend"
```

---

### Task 9: Wire setup --agent into cli.ts

**Files:**
- Modify: `packages/cli/src/cli.ts`

- [ ] **Step 1: Add `--agent` flag to parseArgs options**

In `cli.ts`, add to the `options` object in `parseArgs`:

```typescript
agent: { type: "string" },
```

- [ ] **Step 2: Add --agent to the setup case**

Find the `case "setup":` block and add handling before the existing `if (values.hooks)` check:

```typescript
case "setup": {
  if (values.agent) {
    const { setupAgent } = await import("./commands/setup.js");
    await setupAgent(repoRoot, values.agent);
    break;
  }
  if (values.hooks) {
    // ... existing hooks handling unchanged ...
  }
  // ... existing mcp handling unchanged ...
}
```

- [ ] **Step 3: Update HELP string**

Add to the `setup:` section in HELP:

```
  --agent <name>           Generate and install project-specific skills (supported: claude)
```

- [ ] **Step 4: Verify cli.ts compiles**

```bash
cd /Users/Jerry/Developer/sensei && bunx tsc -p packages/cli/tsconfig.json --noEmit 2>&1 | head -30
```

Expected: no errors

- [ ] **Step 5: Run all CLI tests**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/cli
```

Expected: all passing

- [ ] **Step 6: Commit**

```bash
git add packages/cli/src/cli.ts
git commit -m "feat(cli): add sensei setup --agent flag to dispatch setupAgent"
```

---

### Task 10: install_skills MCP tool

**Files:**
- Create: `packages/server/src/tools/install-skills.ts`
- Modify: `packages/server/src/mcp-server.ts`

- [ ] **Step 1: Create install-skills.ts**

Create `packages/server/src/tools/install-skills.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import { writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { extractProjectProfile, SkillGenerator, SkillValidator, ClaudeAdapter } from "@sensei/engine";
import { ClaudeBackend } from "../model/claude-backend.js";
import type { AgentSkillsManifest } from "@sensei/shared";

export async function installSkillsTool(
  db: SupabaseClient,
  repoId: string,
  repoPath: string,
): Promise<{ filesWritten: string[]; errors: string[] }> {
  try {
    const backend = new ClaudeBackend();
    await backend.init();

    const profile = await extractProjectProfile(db, repoId, repoPath);
    const validator = new SkillValidator(backend, profile);
    const generator = new SkillGenerator(backend, profile, validator);
    const skills = await generator.generate();

    const adapter = new ClaudeAdapter();
    const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
    const written = await adapter.writeSkills(skills, repoSlug);

    // Write manifest so dashboard reflects updated state
    const manifest: AgentSkillsManifest = {
      agent: "claude",
      repoSlug,
      skills: written,
      updatedAt: new Date().toISOString(),
    };
    const senseiDir = join(repoPath, ".sensei");
    await mkdir(senseiDir, { recursive: true });
    await writeFile(join(senseiDir, "agent-skills.json"), JSON.stringify(manifest, null, 2), "utf-8");

    return { filesWritten: written.map(f => f.path), errors: [] };
  } catch (err) {
    return { filesWritten: [], errors: [err instanceof Error ? err.message : String(err)] };
  }
}
```

- [ ] **Step 2: Register the tool in mcp-server.ts**

In `packages/server/src/mcp-server.ts`, add the import at the top:

```typescript
import { installSkillsTool } from "./tools/install-skills.js";
```

Add the tool registration before the final `return server;` line:

```typescript
server.tool(
  "install_skills",
  "Generate and install project-specific Claude skills derived from the indexed codebase. Requires ANTHROPIC_API_KEY in the environment.",
  {},
  async () => {
    try {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
      const result = await installSkillsTool(client as any, opts.repoId, opts.repoPath);
      beat(client, "install_skills", result.errors.length === 0);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
    }
  }
);
```

- [ ] **Step 3: Verify server compiles**

```bash
cd /Users/Jerry/Developer/sensei && bunx tsc -p packages/server/tsconfig.json --noEmit 2>&1 | head -30
```

Expected: no errors

- [ ] **Step 4: Run all server tests**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run packages/server
```

Expected: all passing

- [ ] **Step 5: Commit**

```bash
git add packages/server/src/tools/install-skills.ts packages/server/src/mcp-server.ts
git commit -m "feat(server): add install_skills MCP tool — generates and writes Claude skill files"
```

---

## Chunk 4: Dashboard

### Task 11: Agent Skills dashboard route

**Files:**
- Create: `apps/dashboard/src/routes/repos/[id]/agents/+page.server.ts`
- Create: `apps/dashboard/src/routes/repos/[id]/agents/+page.svelte`

- [ ] **Step 1: Create +page.server.ts**

Create `apps/dashboard/src/routes/repos/[id]/agents/+page.server.ts`:

```typescript
import type { PageServerLoad, Actions } from './$types';
import { error, fail, redirect } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { readFile, stat } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { AgentSkillsManifest, AgentSkillFile } from '@sensei/shared';

type SkillStatus = 'present' | 'stale' | 'missing';

interface SkillRow extends AgentSkillFile {
  status: SkillStatus;
}

const STALE_DAYS = 7;

function computeStatus(generatedAt: string): SkillStatus {
  const ageMs = Date.now() - new Date(generatedAt).getTime();
  const ageDays = ageMs / (1000 * 60 * 60 * 24);
  return ageDays > STALE_DAYS ? 'stale' : 'present';
}

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  // The repos table stores the filesystem path in `local_path` (see migration 20260313000000)
  const repoPath = (repo as { id: string; name: string; local_path: string }).local_path;
  const manifestPath = join(repoPath, '.sensei', 'agent-skills.json');

  let agent: string | null = null;
  let skills: SkillRow[] = [];

  if (existsSync(manifestPath)) {
    try {
      const raw = await readFile(manifestPath, 'utf-8');
      const manifest = JSON.parse(raw) as AgentSkillsManifest;
      agent = manifest.agent;

      for (const skillFile of manifest.skills) {
        if (!existsSync(skillFile.path)) {
          skills.push({ ...skillFile, status: 'missing' });
          continue;
        }
        // Use file mtime for accurate freshness
        const stats = await stat(skillFile.path);
        const status = computeStatus(stats.mtime.toISOString());
        skills.push({ ...skillFile, status });
      }
    } catch {
      // Malformed manifest — treat as unconfigured
    }
  }

  return {
    repo: repo as { id: string; name: string; local_path: string },
    agent,
    skills,
  };
};

export const actions: Actions = {
  regenerate: async ({ params }) => {
    const db = getDb();
    const { data: repo } = await db
      .from('repos')
      .select('local_path')
      .eq('id', params.id)
      .single();

    if (!repo) return fail(404, { error: 'Repo not found' });

    const repoPath = (repo as { local_path: string }).local_path;

    try {
      const proc = Bun.spawn(
        ['sensei', 'setup', '--agent', 'claude'],
        {
          cwd: repoPath,
          env: { ...process.env },
          stdout: 'pipe',
          stderr: 'pipe',
        }
      );

      // 60-second timeout
      const timeout = setTimeout(() => proc.kill(), 60_000);

      const exitCode = await proc.exited;
      clearTimeout(timeout);

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || 'sensei setup failed' });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei setup' });
    }

    redirect(303, `/repos/${params.id}/agents`);
  },
};
```

- [ ] **Step 2: Create +page.svelte**

Create `apps/dashboard/src/routes/repos/[id]/agents/+page.svelte`:

```svelte
<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const statusColor: Record<string, string> = {
    present: 'status-present',
    stale:   'status-stale',
    missing: 'status-missing',
  };

  const statusLabel: Record<string, string> = {
    present: 'Fresh',
    stale:   'Stale',
    missing: 'Missing',
  };

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString();
  }

  function shortPath(path: string): string {
    const home = path.indexOf('.claude/skills/');
    return home !== -1 ? '~/.claude/skills/' + path.slice(home + '.claude/skills/'.length) : path;
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Agent Skills</h1>

{#if data.agent}
  <p>Configured for: <strong>{data.agent === 'claude' ? 'Claude Code' : data.agent}</strong></p>

  {#if data.skills.length > 0}
    <table>
      <thead>
        <tr>
          <th>Category</th>
          <th>File</th>
          <th>Generated</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {#each data.skills as skill}
          <tr>
            <td>{skill.category}</td>
            <td><code>{shortPath(skill.path)}</code></td>
            <td>{formatDate(skill.generatedAt)}</td>
            <td class={statusColor[skill.status] ?? ''}>{statusLabel[skill.status] ?? skill.status}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p>No skill files found on disk.</p>
  {/if}

  <form method="POST" action="?/regenerate">
    <button type="submit">Regenerate Skills</button>
  </form>
{:else}
  <p>No agent skills configured for this repo.</p>
  <p>Run <code>sensei setup --agent claude</code> in the repo directory to generate skills.</p>
{/if}

<style>
  .status-present { color: green; }
  .status-stale   { color: goldenrod; }
  .status-missing { color: red; }
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; }
</style>
```

- [ ] **Step 3: Verify SvelteKit types are generated**

```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard && bun run dev &
sleep 3
kill %1 2>/dev/null || true
```

This generates `.svelte-kit/types/src/routes/repos/[id]/agents/$types.d.ts`. If the route has TypeScript errors, fix them now.

- [ ] **Step 4: Check for TypeScript errors**

```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard && bunx svelte-check --tsconfig ./tsconfig.json 2>&1 | head -40
```

Expected: 0 errors (warnings OK)

- [ ] **Step 5: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/agents/
git commit -m "feat(dashboard): add Agent Skills route showing skill files status and regenerate action"
```

---

### Task 12: Add Agent Skills link to repo page

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/+page.svelte` (line 57)

- [ ] **Step 1: Add the link**

In `apps/dashboard/src/routes/repos/[id]/+page.svelte`, after the analytics link (line 57):

```svelte
<p><a href="/repos/{data.repo.id}/agents">Agent Skills →</a></p>
```

- [ ] **Step 2: Run full test suite**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run
```

Expected: all passing

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/+page.svelte
git commit -m "feat(dashboard): add Agent Skills link on repo detail page"
```

---

## Final verification

- [ ] **Run entire test suite**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run
```

Expected: all passing, zero errors

- [ ] **Lint check**

```bash
cd /Users/Jerry/Developer/sensei && bunx eslint packages/engine/src/project-profile.ts packages/engine/src/skill-gen/ packages/engine/src/agent/ packages/server/src/model/claude-backend.ts packages/cli/src/commands/setup.ts 2>&1 | head -30
```

Expected: no errors

- [ ] **Done When checklist**
  - [ ] `sensei setup --agent claude` runs → 4 skill files in `~/.claude/skills/` with sensei-specific content
  - [ ] Invalid skills cause a clear error listing the validator's issues
  - [ ] `install_skills` MCP tool returns `filesWritten` array with 4 paths
  - [ ] Dashboard `/repos/[id]/agents` shows skill files with Fresh/Stale/Missing badges
  - [ ] Regenerate button triggers `sensei setup --agent claude` and redirects on success
