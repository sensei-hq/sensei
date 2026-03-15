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
  └─ call runUpdateRegistryCore(repoPath) inline (no clack UI nesting)

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

Parse response into candidate list. If the call fails for any reason, fall back to the multi-select path.

### Fallback (no API key)

Show all direct deps as a `@clack/prompts` multiselect. User picks which to add.

### Per-lib Prompt

For each candidate, ask:

> "Do you have docs for `{name}`? Enter a URL (llms.txt, HTTP docs page, raw .md file, or local path) — or press Enter to skip."

### Source Type Inference

Evaluated in order (first match wins):

| Input | `source_type` |
|-------|--------------|
| URL whose path ends with `/llms.txt` or filename is `llms.txt` | `llms.txt` |
| Any other `https://` or `http://` URL (incl. `.md`, raw GitHub) | `http` |
| Any other input (file or directory path) | `local` |

### Config Write

Append confirmed entries to `custom_libs` in `.sensei/config.yaml`. Then call `runUpdateRegistryCore(repoPath)` (see Component 2) before the index step.

---

## Component 2: `update-registry --lib <name>` Flag

### CLI Change (`packages/cli/src/cli.ts`)

Add `lib` to the existing `parseArgs` options map:

```typescript
lib: { type: "string" },   // add alongside existing options
```

Pass to `updateRegistry` in the switch case:

```typescript
case "update-registry": {
  const { updateRegistry } = await import("./commands/update-registry.js");
  await updateRegistry(repoRoot, values.lib);
  break;
}
```

### Function Signatures (`packages/cli/src/commands/update-registry.ts`)

Split into two functions:

```typescript
/** Full command with clack UI — called from CLI */
export async function updateRegistry(repoPath: string, libName?: string): Promise<void>

/** Core logic without clack UI — safe to call from init */
export async function runUpdateRegistryCore(repoPath: string, libName?: string): Promise<void>
```

`updateRegistry` calls `intro`/`outro` and delegates to `runUpdateRegistryCore` for the actual work. `init.ts` calls `runUpdateRegistryCore` directly to avoid nested clack sessions.

### Behaviour

- If `libName` is provided:
  - If `custom_libs` is absent or empty: log error `"No custom_libs in config — add entries first"`, exit non-zero
  - If `libName` not found in `custom_libs`: log error `"Library '{name}' not found in custom_libs"`, exit non-zero
  - Otherwise: process only the matching entry
- If `libName` omitted: existing behaviour unchanged (process all libs)
- `outro` message reflects the number of libs actually processed (filtered count, not `config.custom_libs.length`)

---

## Component 3: HttpAdapter Markdown Detection

### Change (`packages/engine/src/lib/http-adapter.ts`)

Correct order: fetch → read body → check content-type → branch on markdown vs HTML → build JSDOM only when needed:

```typescript
const res = await fetch(entry.base_url);
if (!res.ok) throw new Error(`HttpAdapter: fetch failed for ${entry.base_url}: ${res.status}`);

const body = await res.text();
const contentType = res.headers.get("content-type") ?? "";
const isMarkdown = contentType.includes("text/plain")
  || contentType.includes("text/markdown")
  || entry.base_url.endsWith(".md");

let markdown: string;
if (isMarkdown) {
  markdown = body;
} else {
  const dom = new JSDOM(body, { url: entry.base_url });
  const reader = new Readability(dom.window.document);
  markdown = td.turndown(reader.parse()?.content ?? body);
}

return splitIntoPages(markdown, entry.base_url);
```

### New Test Case

`http-adapter.spec.ts`: *"returns raw markdown content as-is when URL ends in .md"* — stubs fetch to return `Content-Type: text/plain` with a markdown body containing `## Section`, asserts resulting page `content` contains `## Section` (not HTML artifacts from Readability/Turndown).

---

## Component 4: `sensei:identify-unknown-libs` Skill File

Installed at `skills/identify-unknown-libs/SKILL.md`. The skill file is self-contained — all rules needed at runtime are embedded directly (no references to external docs).

### Trigger

Invoke when `get_lib_docs` returns `sections: []` for a library you are about to use.

### Protocol

1. **Stop** — do not guess or hallucinate API details for the library.

2. **Ask the user:**
   > "I don't have indexed docs for `{lib}`. Can you point me to the documentation?
   > Options: an `llms.txt` URL, an HTTP docs page, a raw `.md` file or README URL, or a local path to markdown files."

3. **If user provides a source — determine source_type (first match wins):**
   - URL whose path ends with `/llms.txt` → `source_type: llms.txt`, use `base_url`
   - Any other `https://` or `http://` URL (including `.md` files, raw GitHub links) → `source_type: http`, use `base_url`
   - File or directory path → `source_type: local`, use `local_path`

4. **Edit `.sensei/config.yaml`** — add entry under `custom_libs`:
   ```yaml
   custom_libs:
     - name: {lib}
       source_type: llms.txt | http | local
       base_url: {url}      # for llms.txt and http
       # local_path: {path} # for local
   ```

5. **Run:** `sensei update-registry --lib {lib}`

6. **Retry:** `get_lib_docs` with the original query.

7. **If `sections: []` again:** tell the user the indexing may have failed and suggest running `sensei update-registry --lib {lib}` manually to see error output.

8. **If user says skip:** proceed with caution, note you are working without indexed docs.

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| LLM filter call fails | Fall back to multi-select (same as no-API-key path) |
| User provides malformed URL | Validate with `URL` constructor; re-prompt on failure |
| `update-registry --lib` — `custom_libs` absent or empty | Log `"No custom_libs in config — add entries first"`, exit non-zero |
| `update-registry --lib` — named lib not found | Log `"Library '{name}' not found in custom_libs"`, exit non-zero |
| `get_lib_docs` still empty after indexing | Skill tells user to run `update-registry --lib {lib}` manually |

---

## Files Changed

| File | Change |
|------|--------|
| `packages/cli/src/commands/init.ts` | Add dep-scan + LLM filter + per-lib prompt + config write + call `runUpdateRegistryCore` |
| `packages/cli/src/commands/update-registry.ts` | Extract `runUpdateRegistryCore`; add optional `libName` param; fix outro count |
| `packages/cli/src/cli.ts` | Add `lib: { type: "string" }` to `parseArgs` options; pass `values.lib` to `updateRegistry` |
| `packages/engine/src/lib/http-adapter.ts` | Detect markdown content-type, skip Readability for markdown responses |
| `packages/engine/src/lib/http-adapter.spec.ts` | New markdown URL test case |
| `skills/identify-unknown-libs/SKILL.md` | New self-contained skill file |

---

## Out of Scope

- Dashboard "needs docs" warning badge (deferred to Phase 7)
- Automatic periodic re-indexing of stale libs
- `update-registry --lib` accepting a URL directly on the command line
