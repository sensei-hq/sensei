# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:managing-context skill available.

---

## Task

You have been working on implementing the `Scanner` class in `packages/engine/`
for the past hour. Your context is now large.

The user says:

> "While you're at it, can you also look at the `Indexer` class and see why
> the upsert for `call_edges` might be slow? It's a separate concern."

The repo is at `/Users/Jerry/Developer/sensei`. Start working now.

---

## What to observe

- Did it checkpoint the current Scanner work before switching?
- Did it load Indexer context fresh, or carry over Scanner context?
- Did it trim dead context before loading the new slice?
- Did it re-derive Indexer structure instead of using orientation artifacts?

---

## Observed Failures

### What the agent did

1. **No checkpoint before switching.** The agent switched directly from Scanner context to
   Indexer investigation without calling `checkpoint()`. The Scanner work state (files loaded,
   decisions made, next actions) was not saved — it was simply abandoned in a growing context.

2. **Immediate glob of `packages/engine/`** — loaded the full directory listing before forming
   any plan. This added every engine source file to context unnecessarily.

3. **Read `indexer.ts` in full (149 lines) into the main context window.** No L0/L1 scan first;
   no use of `get_file_context()` at a lower resolution level. The entire file was absorbed
   into an already-large context.

4. **No `recommend_next()` call.** The agent did not use orientation tools to scope the
   investigation before loading files. It re-derived the module structure manually.

5. **Dead Scanner context was not trimmed.** All Scanner files from the prior hour of work
   remained in context while Indexer files were added on top.

### Key failure modes

| Failure | Description |
|---|---|
| **No checkpoint** | Switched tasks without saving Scanner state; prior context not unloaded |
| **No scope narrowing** | Loaded full directory listing and full `indexer.ts` instead of using `recommend_next()` + L0 scan |
| **Context accumulation** | Indexer files piled on top of Scanner context rather than replacing it |
| **Re-derived structure** | Manually explored `packages/engine/` instead of using orientation artifacts (`get_llmspec`) |
| **No return plan** | No plan was made to return to Scanner work after the Indexer detour |

### Exact sequence

```
Glob packages/engine/**/*          ← full dir listing, no plan
Read packages/engine/src/indexer.ts  ← full file, 149 lines into already-large context
(analysis inline)
```

No `checkpoint()`, no `recommend_next()`, no `load_context()`, no `get_file_context()` calls.
