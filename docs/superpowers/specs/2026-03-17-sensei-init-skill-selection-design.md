# sensei init Skill Selection Design

> **Status:** approved
> **Date:** 2026-03-17

## Goal

`sensei init` prompts the user to select which skills to install, with a curated recommended default set pre-checked. `--use-recommended` skips the prompt and installs the recommended set non-interactively.

## Architecture

**Skill catalog** — a static list in `packages/cli/src/lib/skill-catalog.ts` that defines all installable skills with metadata: name, description, whether it is recommended, and its source path in the repo's `skills/` directory.

**`sensei init` flow change** — the existing `promptAndInstallSkills()` call is replaced with `promptAndInstallSkillsFromCatalog()`, which shows a multiselect with recommended skills pre-checked.

**`--use-recommended` flag** — new CLI flag on `sensei init` that bypasses the prompt and installs only skills where `recommended: true`.

---

## Skill Catalog

Defined in `packages/cli/src/lib/skill-catalog.ts`:

```typescript
export interface SkillCatalogEntry {
  name: string;           // matches skills/<name>/SKILL.md
  label: string;          // display name in prompt
  description: string;    // one-line shown in prompt hint
  recommended: boolean;   // pre-checked in multiselect
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

---

## CLI Changes

### New flag on `sensei init`

```typescript
"use-recommended": { type: "boolean", default: false }
```

### `init()` function change

Replace `promptAndInstallSkills()` call with:

```typescript
if (opts.useRecommended) {
  await installSkillsFromCatalog(cwd, "recommended");
} else {
  await promptAndInstallSkillsFromCatalog(cwd);
}
```

### `promptAndInstallSkillsFromCatalog(cwd)`

Uses `@clack/prompts` multiselect:
- All catalog entries shown
- Recommended entries pre-checked
- User can add/remove before confirming
- On confirm: copies selected skill directories to `.claude/skills/` (repo-local)

### `installSkillsFromCatalog(cwd, mode: "recommended" | "all")`

Non-interactive install — filters catalog by `recommended: true` (or all) and copies directly.

---

## Skill Install Mechanics

Skills are sourced from the repo's own `skills/` directory and copied to `.claude/skills/<name>/`. This uses the existing `installSkills()` logic but scoped to the catalog selection.

The install step runs after hooks and MCP registration in `sensei init`, replacing the current `promptAndInstallSkills` call.

---

## HELP text addition

```
init:
  --use-recommended        Install recommended skills without prompting
```

---

## Files Created/Modified

| File | Action |
|------|--------|
| `packages/cli/src/lib/skill-catalog.ts` | Create |
| `packages/cli/src/commands/init.ts` | Modify — replace skill install call, add `useRecommended` option |
| `packages/cli/src/commands/install-skills.ts` | Modify — add `promptAndInstallSkillsFromCatalog`, `installSkillsFromCatalog` |
| `packages/cli/src/cli.ts` | Modify — add `--use-recommended` flag |
| `packages/cli/src/commands/init.spec.ts` | Modify — add tests for catalog selection and `--use-recommended` |
