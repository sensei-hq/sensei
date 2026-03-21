---
description: Deep-dive a feature — generates openspec/specs/<capability>/ docs
argument-hint: Capability name (e.g. auth, payments)
---

Read `docs/reverse-engineer.md` in full, then execute it with:

mode=feature capability=$ARGUMENTS

If $ARGUMENTS is empty, run interactive feature selection (requires `openspec/product/features.md` to exist — run `/product` first if it doesn't).

Follow the workflow exactly as specified in reverse-engineer.md:
- Generate proposal.md, spec.md, design.md, api.md, flow-diagram.md, nfr.md
- Apply backlog ID schema under BL-<CAPABILITY>-*
- Enforce overwrite protection
