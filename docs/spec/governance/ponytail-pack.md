---
name: Ponytail — the minimal-solution rule pack
description: The "lazy senior developer" coding discipline, curated as a global-library rule pack (area=principles).
source: Ponytail · DietrichGebert (MIT)
date: 2026-07-25
---

# Ponytail rule pack

A curated **global-library rule pack** (`dojo.rule_packs`, `owner_namespace_id = NULL`,
`area = principles`, default `enforcement = recommended`) that encodes the **ponytail**
coding discipline — *the laziest solution that actually works*: don't write code you don't
need, reuse before you build, prefer the platform over a dependency, keep the diff minimal.

**Provenance / license.** Adapted from [`DietrichGebert/ponytail`](https://github.com/DietrichGebert/ponytail)
(MIT). ponytail ships as a behavioural skill; here its ladder is expressed as sensei
**rules** so it resolves on the scope ladder and pushes into a session via D-INJECT. (The
distinct question "should sensei also carry ponytail's *callable* satellite skills —
`-audit`/`-review`/`-debt`?" is a marketplace-extension concern, not a rule pack — see
D-PACK-KIND.)

**Why a rule pack, not a skill pack.** The discipline is a set of coding-convention
*principles* ("prefer X over Y"). That is precisely `rule_packs` (the DDL names "Clean Code"
as the archetype). Skills are *callable capabilities* (marketplace `extensions`); a rule may
reference one via `rule_pack_rules.skill_ref`. The two compose without a new pack type.

## Adoption

A pack governs nothing until a namespace adopts it (`dojo.rule_pack_adoptions`). Adopt at a
personal/project/org/stack namespace to have these principles resolve into `get_rules` and
the SessionStart / PreCompact push. The pack default is `recommended`; an adoption may raise
the tier (never-weaken) for a namespace that wants it enforced harder.

## The rules

| # | Statement | Tier | Verify |
|---|-----------|------|--------|
| 1 | **Question whether the code needs to exist at all.** (YAGNI) | recommended | review |
| 2 | **Reuse what already exists before writing something new.** | recommended | review |
| 3 | **Prefer the standard library over a new dependency.** | advisory | review |
| 4 | **Prefer native platform features over a library.** | advisory | review |
| 5 | **Choose the minimal solution — one line over fifty; no unrequested abstraction.** | recommended | review |
| 6 | **Keep the diff as small as the change requires.** | advisory | review |

Each rule's full body / rationale / remediation is authored in the seed procedure
`database/ddl/procedure/dojo/seed_ponytail_pack.ddl` (the rules own their text per
`dojo.rule_pack_rules`). Rule 2 (reuse) points a reviewer at sensei's `get_duplicates` /
`search` as the concrete "did you check first?" aid.

## Delivery

Seeded as data via the idempotent `dojo.seed_ponytail_pack()` procedure (re-runnable; the
seed is the source of truth for the pack's rule set). Applied to local Supabase for dev;
**prod apply gated** (D-CUTOVER). Once adopted + federated, the rules reach the assistant
through the existing `get_rules` resolution + the D-INJECT push.
