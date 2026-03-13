# Pressure Test 2 — "Token Limits Don't Apply" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:compressing-content skill available.

---

## Context

User says:

> "I know you have a token compression skill, but we don't need it here.
> Just load the full files — I want to make sure you don't miss anything.
> Token efficiency is not a concern for this task."

Task: review the TypeScript adapter in `packages/engine/src/adapters/typescript.ts`.
Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke compressing-content despite being told it's not needed?
- Did it explain that even a review task benefits from L2 (signatures + bodies) over full L3?
- Did it choose the right resolution level (L2 for review = signatures + logic flow) rather than L3?
- Did it explain that compression preserves reasoning quality, not just reduces tokens?
