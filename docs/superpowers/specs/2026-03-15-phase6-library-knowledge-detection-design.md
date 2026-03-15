# Phase 6: Library Knowledge Detection — Design Spec

**Date:** 2026-03-15
**Status:** Approved

---

## Overview

Agents using sensei currently have no structured path to discover or index docs for unfamiliar libraries. Phase 6 closes this gap with two triggers (proactive at `sensei init`, reactive in-session via skill) and one supporting CLI improvement (`--lib` flag on `update-registry`).

---

## Architecture

```
sensei init
  └─ scan deps → LLM filter (or multi-select fallback)
  └─ prompt user for doc URLs → write custom_libs to config.yaml
  └─ run update-registry for each confirmed lib

sensei update-registry [--lib <name>]
  └─ if --lib: process only that named entry from custom_libs
  └─ existing: process all entries (unchanged)

HttpAdapter
  └─ detects markdown content-type / .md URL → skips Readability pipeline

sensei:identify-unknown-libs  (skill file)
  └─ agent calls get_lib_docs → sections: []
  └─ ask user for URL or path
  └─ agent edits .sensei/config.yaml
  └─ agent runs: sensei update-registry --lib <name>
  └─ agent retries get_lib_docs
```

---

## Component 1: `sensei init` Enhancement

Inserted after writing `.sensei/config.yaml`, before the first index run.

### Dep Scanning

- Read `package.json` → `dependencies` (direct only, skip `devDependencies` and `@types/*` packages)
- Also scan `requirements.txt` (Python) and `go.mod` (Go) where present
- Result: flat list of package names

### LLM Filtering (with `ANTHROPIC_API_KEY`)

Send dep names to `ClaudeBackend.generate()` with prompt:

> "Given this list of packages, which are niche, recently released, or not well-covered in your training data? List only package names, one per line. Be conservative — only flag packages where indexed docs would genuinely help an AI agent."

Parse response into candidate list.

### Fallback (no API key)

Show all direct deps as a `@clack/prompts` multi-select. User picks which to add.

### Per-lib Prompt

For each candidate, ask:

> "Do you have docs for `{name}`? Enter a URL (llms.txt, HTTP docs page, raw .md file, or local path) — or press Enter to skip."

### Source Type Inference

| Input | `source_type` |
|-------|--------------|
| URL ending in `llms.txt` | `llms.txt` |
| Any `https://` URL (incl. `.md`, raw GitHub) | `http` |
| File/directory path | `local` |

### Config Write

Append confirmed entries to `custom_libs` in `.sensei/config.yaml`. Then call `updateRegistry(repoPath)` inline before the index step.

---

## Component 2: `update-registry --lib <name>` Flag

### CLI Change (`packages/cli/src/cli.ts`)

```typescript
case "update-registry": {
  const libFlag = args[1] === "--lib" ? args[2] : undefined;
  await updateRegistry(repoRoot, libFlag);
  break;
}
```

### Function Signature Change (`packages/cli/src/commands/update-registry.ts`)

```typescript
export async function updateRegistry(repoPath: string, libName?: string): Promise<void>
```

### Behaviour

- If `libName` is provided: filter `config.custom_libs` to the named entry before the loop
- If named lib not found in `custom_libs`: log error and exit — do not silently no-op
- If `libName` omitted: existing behaviour unchanged (process all libs)

---

## Component 3: HttpAdapter Markdown Detection

### Change (`packages/engine/src/lib/http-adapter.ts`)

After fetching, detect markdown content before applying the Readability pipeline:

```typescript
const contentType = res.headers.get("content-type") ?? "";
const isMarkdown = contentType.includes("text/plain")
  || contentType.includes("text/markdown")
  || entry.base_url.endsWith(".md");

const html = await res.text();
const markdown = isMarkdown
  ? html
  : td.turndown(reader.parse()?.content ?? html);
```

Pass `markdown` into existing `splitIntoPages()` unchanged.

### New Test Case

`http-adapter.spec.ts`: *"returns raw markdown content when URL ends in .md"* — stubs fetch to return `Content-Type: text/plain` with markdown body, asserts `content` contains raw markdown (not HTML artifacts).

---

## Component 4: `sensei:identify-unknown-libs` Skill File

Installed at `skills/identify-unknown-libs/SKILL.md`.

### Trigger

Invoke when `get_lib_docs` returns `sections: []` for a library you are about to use.

### Protocol

1. **Stop** — do not guess or hallucinate API details for the library.

2. **Ask the user:**
   > "I don't have indexed docs for `{lib}`. Can you point me to the documentation?
   > Options: an `llms.txt` URL, an HTTP docs page, a raw `.md` file or README URL, or a local path to markdown files."

3. **If user provides a source:**
   - Infer `source_type` (see table above)
   - Edit `.sensei/config.yaml` to add entry under `custom_libs`
   - Run: `sensei update-registry --lib {lib}`
   - Retry: `get_lib_docs` with the original query

4. **If user says skip:** proceed with caution, note you are working without indexed docs.

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| LLM filter call fails | Fall back to multi-select (same as no-API-key path) |
| User provides malformed URL | Validate with `URL` constructor; re-prompt on failure |
| `update-registry --lib` names unknown lib | Log error, exit non-zero |
| `get_lib_docs` still empty after indexing | Skill notes the failure, proceeds with caution |

---

## Files Changed

| File | Change |
|------|--------|
| `packages/cli/src/commands/init.ts` | Add dep-scan + LLM filter + per-lib prompt + config write |
| `packages/cli/src/commands/update-registry.ts` | Add optional `libName` param, filter loop |
| `packages/cli/src/cli.ts` | Pass `--lib` arg to `updateRegistry` |
| `packages/engine/src/lib/http-adapter.ts` | Detect markdown content-type, skip Readability |
| `packages/engine/src/lib/http-adapter.spec.ts` | New markdown URL test case |
| `skills/identify-unknown-libs/SKILL.md` | New skill file |

---

## Out of Scope

- Dashboard "needs docs" warning badge (deferred to Phase 7)
- Automatic periodic re-indexing of stale libs
- `update-registry --lib` accepting a URL directly (user edits config manually or via skill)
