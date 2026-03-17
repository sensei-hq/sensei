# sensei init Skill Selection Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current all-or-nothing skill install in `sensei init` with a catalog-driven multiselect that shows recommended skills pre-checked. Add `--use-recommended` flag for non-interactive installs.

**Architecture:** New `skill-catalog.ts` module holds the static catalog with `recommended: boolean` per entry. `install-skills.ts` gains two new functions: `promptAndInstallSkillsFromCatalog()` (interactive multiselect) and `installSkillsFromCatalog()` (non-interactive). `init.ts` and `cli.ts` are updated to wire in the new flag.

**Tech Stack:** TypeScript, @clack/prompts multiselect, Node.js `fs/promises`, Vitest

---

## Chunk 1: Catalog + Updated Install Functions

### Task 1: Create `packages/cli/src/lib/skill-catalog.ts`

**Files:**
- Create: `packages/cli/src/lib/skill-catalog.ts`

- [ ] **Step 1: Write the catalog file**

```typescript
// packages/cli/src/lib/skill-catalog.ts

export interface SkillCatalogEntry {
  name: string;        // matches skills/<name>/SKILL.md
  label: string;       // display name in multiselect prompt
  description: string; // one-line hint shown in prompt
  recommended: boolean; // pre-checked in multiselect
}

export const SKILL_CATALOG: SkillCatalogEntry[] = [
  {
    name: "zero-errors-policy",
    label: "Zero Errors Policy",
    description: "Enforces zero lint and test errors before marking any task complete",
    recommended: true,
  },
  {
    name: "managing-project-sessions",
    label: "Project Sessions",
    description: "Session continuity — loads context and open items at session start",
    recommended: true,
  },
  {
    name: "pattern-based-development",
    label: "Pattern-Based Development",
    description: "Checks PATTERNS.md for applicable recipes before writing new code",
    recommended: true,
  },
  {
    name: "detecting-doc-drift",
    label: "Doc Drift Detection",
    description: "Flags when design docs fall out of sync with code",
    recommended: true,
  },
  {
    name: "identifying-patterns",
    label: "Pattern Identification",
    description: "Discovers and documents recurring structural patterns in the codebase",
    recommended: true,
  },
  {
    name: "decomposing-broad-tasks",
    label: "Task Decomposition",
    description: "Breaks broad requests into focused, independently testable tasks",
    recommended: false,
  },
  {
    name: "managing-context",
    label: "Context Management",
    description: "Trims and focuses context as sessions grow",
    recommended: false,
  },
  {
    name: "running-agentic-sessions",
    label: "Agentic Sessions",
    description: "Orients agents efficiently using index tools instead of broad file reads",
    recommended: false,
  },
  {
    name: "compressing-content",
    label: "Content Compression",
    description: "Reduces token usage when passing code to LLMs",
    recommended: false,
  },
  {
    name: "indexing-codebase",
    label: "Codebase Indexing",
    description: "Builds and navigates the sensei index for a new codebase",
    recommended: false,
  },
];
```

- [ ] **Step 2: Verify the file was created**

```bash
cat packages/cli/src/lib/skill-catalog.ts
```
Expected: file prints without error.

- [ ] **Step 3: Commit**

```bash
git add packages/cli/src/lib/skill-catalog.ts
git commit -m "feat(cli): add skill catalog with recommended defaults"
```

---

### Task 2: Add catalog-driven functions to `install-skills.ts` with tests

**Files:**
- Modify: `packages/cli/src/commands/install-skills.ts`
- Create: `packages/cli/src/commands/install-skills.spec.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/cli/src/commands/install-skills.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@clack/prompts", () => ({
  select: vi.fn(),
  multiselect: vi.fn(),
  isCancel: vi.fn(() => false),
  spinner: vi.fn(() => ({ start: vi.fn(), stop: vi.fn() })),
  log: { warn: vi.fn() },
}));

vi.mock("fs/promises", () => ({
  // readdir is not called by the catalog functions — they iterate SKILL_CATALOG directly
  access: vi.fn().mockResolvedValue(undefined),
  copyFile: vi.fn().mockResolvedValue(undefined),
  mkdir: vi.fn().mockResolvedValue(undefined),
}));

import { installSkillsFromCatalog, promptAndInstallSkillsFromCatalog } from "./install-skills.js";
import { multiselect, isCancel } from "@clack/prompts";
import { copyFile } from "fs/promises";

const mockMultiselect = multiselect as ReturnType<typeof vi.fn>;
const mockIsCancel = isCancel as unknown as ReturnType<typeof vi.fn>;
const mockCopyFile = copyFile as ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.clearAllMocks();
});

describe("installSkillsFromCatalog", () => {
  it("installs only recommended skills in 'recommended' mode", async () => {
    await installSkillsFromCatalog("/repo", "recommended");
    const copiedNames = (mockCopyFile as ReturnType<typeof vi.fn>).mock.calls.map(
      ([src]: [string]) => src.split("/").slice(-2, -1)[0]
    );
    expect(copiedNames).toContain("zero-errors-policy");
    expect(copiedNames).toContain("managing-project-sessions");
    expect(copiedNames).toContain("pattern-based-development");
    // Non-recommended not included
    expect(copiedNames).not.toContain("decomposing-broad-tasks");
  });

  it("installs all catalog skills in 'all' mode", async () => {
    await installSkillsFromCatalog("/repo", "all");
    const copiedNames = (mockCopyFile as ReturnType<typeof vi.fn>).mock.calls.map(
      ([src]: [string]) => src.split("/").slice(-2, -1)[0]
    );
    expect(copiedNames).toContain("zero-errors-policy");
    expect(copiedNames).toContain("decomposing-broad-tasks");
  });
});

describe("promptAndInstallSkillsFromCatalog", () => {
  it("uses catalog recommended entries as initial values in multiselect", async () => {
    mockMultiselect.mockResolvedValue(["zero-errors-policy", "managing-project-sessions"]);
    await promptAndInstallSkillsFromCatalog("/repo");
    const call = mockMultiselect.mock.calls[0][0];
    expect(call.initialValues).toContain("zero-errors-policy");
    expect(call.initialValues).not.toContain("decomposing-broad-tasks");
  });

  it("installs only the user-selected skills", async () => {
    mockMultiselect.mockResolvedValue(["zero-errors-policy"]);
    await promptAndInstallSkillsFromCatalog("/repo");
    const copiedNames = (mockCopyFile as ReturnType<typeof vi.fn>).mock.calls.map(
      ([src]: [string]) => src.split("/").slice(-2, -1)[0]
    );
    expect(copiedNames).toEqual(["zero-errors-policy"]);
  });

  it("does nothing when user cancels", async () => {
    mockIsCancel.mockReturnValue(true);
    await promptAndInstallSkillsFromCatalog("/repo");
    expect(mockCopyFile).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/cli && bunx vitest run src/commands/install-skills.spec.ts
```
Expected: FAIL — `installSkillsFromCatalog` and `promptAndInstallSkillsFromCatalog` not exported

- [ ] **Step 3: Add the two new functions to `install-skills.ts`**

Add these imports at the top of `install-skills.ts` (after existing imports):
```typescript
import { SKILL_CATALOG } from "../lib/skill-catalog.js";
```

Add these two functions at the end of the file (after `installSkills`):

```typescript
/** Interactive catalog-driven install — recommended skills pre-checked. */
export async function promptAndInstallSkillsFromCatalog(repoPath: string): Promise<void> {
  const skillsDir = await findSkillsDir();
  if (!skillsDir) {
    log.warn("Could not locate sensei skills directory — skipping skill installation.");
    return;
  }

  const selected = await multiselect({
    message: "Which sensei skills would you like to install? (space to toggle, enter to confirm)",
    options: SKILL_CATALOG.map(entry => ({
      value: entry.name,
      label: entry.label,
      hint: entry.description,
    })),
    initialValues: SKILL_CATALOG.filter(e => e.recommended).map(e => e.name),
    required: false,
  });
  if (isCancel(selected) || (selected as string[]).length === 0) return;

  const targetDir = join(repoPath, ".claude", "skills");
  const s = spinner();
  s.start("Installing skills...");

  let installedCount = 0;
  await mkdir(targetDir, { recursive: true });
  for (const skillName of selected as string[]) {
    const skillMd = join(skillsDir, skillName, "SKILL.md");
    try {
      await access(skillMd);
      await copyFile(skillMd, join(targetDir, `${skillName}.md`));
      installedCount++;
    } catch {
      // skill not present in this build — skip
    }
  }
  s.stop(`Installed ${installedCount} skill${installedCount !== 1 ? "s" : ""} → ${targetDir}`);
}

/** Non-interactive: install skills from the catalog filtered by mode. */
export async function installSkillsFromCatalog(
  repoPath: string,
  mode: "recommended" | "all",
): Promise<void> {
  const skillsDir = await findSkillsDir();
  if (!skillsDir) {
    log.warn("Could not locate sensei skills directory.");
    return;
  }

  const entries = mode === "recommended"
    ? SKILL_CATALOG.filter(e => e.recommended)
    : SKILL_CATALOG;

  const targetDir = join(repoPath, ".claude", "skills");
  const s = spinner();
  s.start(`Installing ${mode} skills...`);

  let installedCount = 0;
  await mkdir(targetDir, { recursive: true });
  for (const entry of entries) {
    const skillMd = join(skillsDir, entry.name, "SKILL.md");
    try {
      await access(skillMd);
      await copyFile(skillMd, join(targetDir, `${entry.name}.md`));
      installedCount++;
    } catch {
      // skill not present in this build — skip
    }
  }
  s.stop(`Installed ${installedCount} skill${installedCount !== 1 ? "s" : ""} → ${targetDir}`);
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd packages/cli && bunx vitest run src/commands/install-skills.spec.ts
```
Expected: all tests PASS

- [ ] **Step 5: TypeScript check**

```bash
bunx tsc --noEmit -p packages/cli/tsconfig.json
```
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add packages/cli/src/commands/install-skills.ts packages/cli/src/commands/install-skills.spec.ts
git commit -m "feat(cli): add promptAndInstallSkillsFromCatalog and installSkillsFromCatalog"
```

---

## Chunk 2: Wire Into `init.ts` and `cli.ts`

### Task 3: Update `init.ts` — add `useRecommended` option, replace skill install call

**Files:**
- Modify: `packages/cli/src/commands/init.ts`
- Modify: `packages/cli/src/commands/init.spec.ts`

- [ ] **Step 1: Write the failing tests**

Open `packages/cli/src/commands/init.spec.ts`. Make these three changes:

**Change 1:** Add a `vi.mock("./install-skills.js", ...)` call alongside the other `vi.mock()` calls at the top of the file — it must appear before the `import { init }` line so Vitest's hoisting works correctly:

```typescript
vi.mock("./install-skills.js", () => ({
  installSkills: vi.fn().mockResolvedValue(undefined),
  promptAndInstallSkills: vi.fn().mockResolvedValue(undefined),
  promptAndInstallSkillsFromCatalog: vi.fn().mockResolvedValue(undefined),
  installSkillsFromCatalog: vi.fn().mockResolvedValue(undefined),
}));
```

**Change 2:** Add this import at file scope (alongside the other imports, after the `vi.mock` blocks):

```typescript
import { promptAndInstallSkillsFromCatalog, installSkillsFromCatalog } from "./install-skills.js";
```

**Change 3:** Add a nested `describe` block at the end of the existing outer `describe` block. These tests run under the existing `beforeEach` setup (which already configures `mockText`, Supabase client, `mockIndexRepo`, etc.) so no additional mock state is needed:

```typescript
  describe("skill installation routing", () => {
    it("calls promptAndInstallSkillsFromCatalog by default (no --use-recommended)", async () => {
      await init("/repo", {});
      expect(promptAndInstallSkillsFromCatalog).toHaveBeenCalledWith("/repo");
      expect(installSkillsFromCatalog).not.toHaveBeenCalled();
    });

    it("calls installSkillsFromCatalog('recommended') when useRecommended=true", async () => {
      await init("/repo", { useRecommended: true });
      expect(installSkillsFromCatalog).toHaveBeenCalledWith("/repo", "recommended");
      expect(promptAndInstallSkillsFromCatalog).not.toHaveBeenCalled();
    });
  });
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd packages/cli && bunx vitest run src/commands/init.spec.ts
```
Expected: new tests FAIL

- [ ] **Step 3: Update `InitOptions` and the skill install section in `init.ts`**

Add `useRecommended` to `InitOptions`:
```typescript
export interface InitOptions {
  global?: boolean;
  supabaseUrl?: string;
  serviceKey?: string;
  useRecommended?: boolean;  // Add this
}
```

Replace the existing `installSkills` import at line 9 of `init.ts` with the expanded static import:
```typescript
// Before (line 9):
import { installSkills } from "./install-skills.js";

// After:
import { installSkills, installSkillsFromCatalog, promptAndInstallSkillsFromCatalog } from "./install-skills.js";
```

> This replaces the dynamic `await import("./install-skills.js")` in the else branch below.

Replace the skill install section (step 9 in `init.ts`, lines 294–302):
```typescript
// Before:
  // 9. Install skills
  //    --global → install to ~/.claude/skills/; default → install to .claude/skills/
  if (opts.global) {
    await installSkills(cwd, "global");
  } else {
    // Interactive prompt for scope
    const { promptAndInstallSkills } = await import("./install-skills.js");
    await promptAndInstallSkills(cwd);
  }

// After:
  // 9. Install skills
  //    --global → install to ~/.claude/skills/; default → install to .claude/skills/
  if (opts.global) {
    await installSkills(cwd, "global");
  } else if (opts.useRecommended) {
    await installSkillsFromCatalog(cwd, "recommended");
  } else {
    await promptAndInstallSkillsFromCatalog(cwd);
  }
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd packages/cli && bunx vitest run src/commands/init.spec.ts
```
Expected: all tests PASS

- [ ] **Step 5: TypeScript check**

```bash
bunx tsc --noEmit -p packages/cli/tsconfig.json
```
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add packages/cli/src/commands/init.ts packages/cli/src/commands/init.spec.ts
git commit -m "feat(cli): wire catalog skill selection into sensei init"
```

---

### Task 4: Add `--use-recommended` flag to `cli.ts`

**Files:**
- Modify: `packages/cli/src/cli.ts`

- [ ] **Step 1: Add the flag to `parseArgs` options**

In `cli.ts`, inside the `options` object of `parseArgs`, add after `"service-key"`:
```typescript
"use-recommended": { type: "boolean", default: false },
```

- [ ] **Step 2: Pass the flag to `init()`**

Find the `case "init"` block (around line 159). Change the `init()` call to pass `useRecommended`:
```typescript
// Before:
      await init(repoRoot, {
        global: values.global,
        supabaseUrl: values["supabase-url"] ?? process.env.SUPABASE_URL,
        serviceKey: values["service-key"] ?? process.env.SUPABASE_SERVICE_KEY,
      });

// After:
      await init(repoRoot, {
        global: values.global,
        supabaseUrl: values["supabase-url"] ?? process.env.SUPABASE_URL,
        serviceKey: values["service-key"] ?? process.env.SUPABASE_SERVICE_KEY,
        useRecommended: values["use-recommended"],
      });
```

- [ ] **Step 3: Add help text**

In the HELP string, find the `init:` section (around line 88). Add the new flag:
```
init:
  --global                 Install skills and hooks globally (~/.claude/) instead of repo-local
  --supabase-url <url>     Supabase URL (default: $SUPABASE_URL or prompts; fallback: http://localhost:54321)
  --service-key <key>      Supabase service role key (default: $SUPABASE_SERVICE_KEY or prompts)
  --use-recommended        Install recommended skills without prompting
```

- [ ] **Step 4: TypeScript check**

```bash
bunx tsc --noEmit -p packages/cli/tsconfig.json
```
Expected: zero errors

- [ ] **Step 5: Commit**

```bash
git add packages/cli/src/cli.ts
git commit -m "feat(cli): add --use-recommended flag to sensei init"
```

---

## Final Verification

- [ ] **Run full CLI test suite**

```bash
bun run --filter '@sensei/cli' test
```
Expected: zero failures

- [ ] **Run full test suite**

```bash
bun run --filter '*' test
```
Expected: zero failures

- [ ] **TypeScript check**

```bash
bunx tsc --noEmit
```
Expected: zero errors

- [ ] **Verify catalog exports via Vitest**

```bash
cd packages/cli && bunx vitest run src/lib/skill-catalog.spec.ts --passWithNoTests
```

Or verify manually by checking the exported array has exactly 5 recommended entries:

```bash
cd packages/cli && bunx tsc --noEmit && grep -c "recommended: true" src/lib/skill-catalog.ts
```
Expected: prints `5`
