---
name: Governance & grounding — feature
updated: 2026-07-20
cluster: 3 · synthetic analysis (foundation)
status: rules ✅ · project memory + anchoring ✅ · tooling awareness 🟡 · right-sized retrieval ✅ · post-compaction stickiness 🟡 (the frontier)
---

# Governance & grounding

## What is it?

The layer that keeps you and the assistant working from *this project's* actual
principles, memory, and tools — right-sized to the task and persistent across a
long session — instead of re-deriving decisions, re-searching for tooling, or
drifting once the context has been compacted.

## What problems does it solve?

- The assistant burns cycles hunting for which commands/tools this project uses
  (test · build · lint · the MCP tools) instead of already knowing them.
- It sends a large context for a narrow task — slow, costly, noisy.
- Guiding principles and past decisions get forgotten — worst of all **after
  context compaction** on a long / multi-day session.
- Grounding isn't project-specific — generic advice instead of *this* repo's
  rules and memory.

## How do you use it?

Mostly you don't — it's ambient. sensei pushes the project's rules, the relevant
memory, and the known tool/command set into the assistant's path at the start of
a session and at the right moments. You author and promote the rules (the
guiding principles); you can see what's grounding the current session.

## What happens behind the scenes? (how it unfolds)

- **Guiding principles** — rules resolved live (mandatory + project + scoped), so
  the assistant always has the current constraints, not a stale copy.
- **Project-specific memory** — decisions, gotchas, preferences, anchored to the
  doc spine; retrieved *scoped to the task*, not dumped whole.
- **Known tooling** — the project's commands + available tools come from the
  manifest, so the assistant doesn't search for them.
- **Right-sized context** — a narrow task pulls narrow context (slot / feature
  scoped), not the whole project.
- **Re-grounding** — principles + scoped memory + tool set are re-asserted at
  session boundaries and after compaction, so grounding survives long runs.

## How does it stay sticky (survive compaction)? — the hard part

Grounding can't be a one-time prompt that a summary throws away. sensei
re-injects the rules, the task-scoped memory, and the tool set at session start
and after a compaction event (via the session hook), so the assistant is
*re-grounded* rather than left to drift. **Today this is partial:** session-start
guidance and memory-anchoring exist; robust, automatic post-compaction
re-injection is the frontier and the thing most worth getting right.

## How is context kept right-sized?

Memory is anchored to spine slots and retrieved by the task's slot / feature, so
a bug fix in one module doesn't drag in the whole project's memory. Governance
rules are resolved for the scope in play, not blanket-applied.

## Who does what?

- **You / lead** — author and promote the guiding principles (rules); decide
  what's mandatory vs advisory.
- **sensei** — grounds automatically: pushes rules + scoped memory + tools;
  re-grounds after compaction.
- **The assistant** — consumes the grounding; needs less searching and less
  context to stay on target.

## Where does it fit?

It's the substrate under everything else: the front door recommends a *playbook*,
but governance + grounding is what keeps the assistant honest to the project's
principles and memory *while* it works — and what makes any guidance stick.

## Related features

Memory (cluster 4), project **tooling awareness** (manifest / commands), and
**right-sized context assembly** are their own feature docs; this one is the
*governance + grounding* thread that ties them together around "guiding
principles that stick."

## Where are the screens?

The rules / governance view + a "what's grounding this session" surface —
**pending / partial**. `mockup-ref` to follow once the designer's screens land.
