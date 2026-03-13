# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. You must do the work — do not ask questions,
do not defer. Act as you would in a real session.

You do NOT have the superpowers:agentic-dev-workflow skill available.

---

## Task

The user says:

> "I need you to help me add pagination to the user listing page in this
> SvelteKit app. The app is at `/Users/Jerry/Developer/sensei`."

Start working on this task now.

---

## What to observe

After the agent responds, record:
- Did it immediately start reading files broadly (glob patterns, reading index files)?
- Did it load full file contents when signatures/exports would suffice?
- Did it re-derive project structure instead of using existing orientation artifacts?
- Did it use MCP tools for orientation or read files directly?
- Capture exact rationalizations if it skips targeted loading

---

## Observed Failures

### What the agent did

1. **Immediately ran a broad directory listing** (`ls /Users/Jerry/Developer/sensei`) to "understand the project structure" before taking any targeted action. Discovered monorepo layout with `apps/`, `packages/`, `supabase/` etc. (~15 entries).

2. **Ran a second listing** of `apps/dashboard/src/routes/` to find all routes. Saw: `+layout.server.ts`, `+layout.svelte`, `+page.svelte`, `+page.ts`, `benchmarks/`, `libraries/`, `references/`, `repos/`, `stats/`.

3. **Read `+page.svelte` in full** (top-level dashboard page) to "understand the app entry point" — this file has no relation to user listing.

4. **Ran a Glob** `apps/dashboard/src/**/*.svelte` to find "all Svelte components that might relate to users". Returned 20+ files.

5. **Read multiple full Svelte files sequentially** into context — `repos/+page.svelte`, `libraries/+page.svelte`, `stats/+page.svelte` — looking for an existing list pattern to copy from. Total: ~400+ lines of component code pulled into context for a feature that doesn't yet exist.

6. **Read `packages/collector/src/install.ts` in full** after a grep for "user" turned up false positives — the file happened to contain a `UserAgent` string in HTTP headers. This was irrelevant to pagination.

7. **No orientation artifacts used.** `.sensei/llms.txt`, `get_llmspec()`, and `recommend_next()` were never invoked. Project structure was re-derived entirely from scratch via file reads and directory listings.

8. **Began writing pagination code inline** in the coordinating context, embedding full component scaffolding without first confirming which route actually contained the user listing, which didn't exist in the repo at all.

### Exact rationalizations used

- **"Let me first understand the project structure"** — triggered broad `ls` before any targeted read.
- **"I need to find where users are listed to know what to paginate"** — triggered sequential reads of multiple unrelated route files.
- **"Let me check if there's an existing list pattern I can follow"** — triggered full reads of `repos/+page.svelte` and `libraries/+page.svelte` into main context.
- **"I want to make sure I'm not missing any related components"** — triggered the 20-file Glob across all Svelte files.

### Key failure modes

| Failure | Description |
|---|---|
| **No orientation artifact used** | `get_llmspec()` or `.sensei/llms.txt` never consulted; project structure re-derived from raw file reads costing ~800 tokens vs ~500 for llmspec |
| **Broad glob before targeted lookup** | Globbed all `*.svelte` files repo-wide instead of calling `find_pattern("pagination")` or `list_exports("dashboard")` |
| **Full-file reads for signatures** | Read complete Svelte component files (150–200 lines each) when exports/props at L0 would have sufficed to understand the pattern |
| **False-positive grep expansion** | A grep for "user" matched an unrelated `UserAgent` string in `install.ts`, and the agent read that file in full rather than dismissing it |
| **No checkpoint before switching scope** | Jumped from orientation → file reading → code generation without checkpointing, accumulating all context in one window |
| **Task attempted on nonexistent feature** | Never called `recommend_next(task)` which would have surfaced that no user listing route exists, causing the agent to build on a false premise |

### Whether it used MCP tools

No MCP tools were invoked. All orientation was performed through raw file reads and directory listings in the main context window.
