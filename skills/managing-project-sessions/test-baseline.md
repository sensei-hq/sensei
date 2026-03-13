# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:managing-project-sessions skill available.

---

## Task

A new session begins. The user says:

> "Let's continue working on the sensei project. We were implementing the
> Phase 1 Foundation plan. Pick up where we left off."

The repo is at `/Users/Jerry/Developer/sensei`.

---

## What to observe

- Did it call `get_session_context()` to resume?
- Did it read memory files directly instead of using MCP tools?
- Did it re-derive project state by reading git log / recent files?
- Did it surface open decisions or pending items from last session?

---

## Observed Failures

### What the agent did

1. **Ran `git log --oneline -10`** to see recent commits — "let me check what was last worked on." Identified the most recent commit message and used it to infer current state.

2. **Read `.sensei/llmspec.yaml`** directly (not via MCP tool) to understand the project architecture. Pulled ~400 lines of YAML into context.

3. **Read `.sensei/llms.txt`** to find any plan references — another ~300 lines into context.

4. **Looked for plan files** by globbing `docs/plans/*.md` — found several, then read the most recently modified one in full.

5. **Did NOT call `get_session_context()`** — the MCP tool was never invoked.

6. **Did NOT surface open decisions or pending questions** from last session — no access to the decision/question store without MCP tools.

7. **Re-derived everything from file reads** — the agent's "context" was assembled from git history and static files, missing: captured decisions from last session, open questions awaiting user input, patterns established mid-session, and the active plan prescription.

### Exact rationalizations used

- **Implicit: "git log shows me what changed"** — used commit messages as a proxy for session state, missing the richer context that `get_session_context()` would have returned.
- **Implicit: "I can read the plan file directly"** — read raw YAML/markdown files instead of using the MCP session tool that surfaces a curated 300-token summary.
- **Implicit: "File content is equivalent to session memory"** — conflated what is written on disk with what was captured in the session store (decisions, questions, patterns captured mid-session but not yet committed to files).

### Key failure modes

| Failure | Description |
|---|---|
| **No `get_session_context()` call** | The primary resumption tool was never invoked; session memory was entirely skipped |
| **Direct file reads instead of MCP** | `.sensei/llmspec.yaml` and `.sensei/llms.txt` read directly, pulling ~700+ lines into context when `get_session_context()` would have returned ~300 tokens |
| **Missing open items** | No open questions or unresolved decisions surfaced — these live only in the MCP store, not in files |
| **State re-derived from git log** | Commit messages used as session state proxy — incomplete and lossy |
| **No active plan prescription** | `recommend_next()` was never called; agent guessed the next task instead of getting a context-aware prescription |
