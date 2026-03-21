---
description: Reverse-engineer the full product — generates openspec/product/ docs
argument-hint: Optional root path (defaults to .)
---

Read `docs/reverse-engineer.md` in full, then execute it with:

mode=product root=$ARGUMENTS

If $ARGUMENTS is empty, use root=.

Follow the workflow exactly as specified in reverse-engineer.md:
- Auto-detect all stacks
- Generate all product-level docs under `openspec/product/`
- Apply backlog ID schema
- Enforce overwrite protection
