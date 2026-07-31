# FTR friction analysis — where we miss, what's missing, and how to distill it

> Question: what are the areas of friction where we repeatedly miss the mark and
> lower FTR (first-try rate), what's missing that causes the misses, and can it be
> distilled into constitution guidelines or skills? Grounded in the daemon's
> captured history + the observed patterns of a long multi-feature build session
> (2026-07-29 → 31).

## 1. The data (daemon `sensei` DB)

- **FTR = 56%** (18/32 sessions first-try; 14 corrected). Not terrible, not good.
- **`sensei` (this monorepo) is the friction hotspot: FTR 0.31**, 21 corrections
  over 13 sessions. `strategos` 0.57, `kavach`/`dbd` 0.50; `rokkit`/`torii`/
  `swarco`/`alert-platform` 1.00. Friction concentrates in the big, multi-surface
  repo (Rust daemon + dōjō Worker + DDL + app) — the cross-cutting changes.
- **The distillation loop that should already be doing this is silent.** The
  analyzer is designed to cluster corrections (`inference.corrections`) and derive
  governance rules (`sensei.playbook_rules WHERE source='learned'`). Actual counts:
  **1 correction cluster, 0 learned rules** — despite 21 corrections of signal.
  **The mechanism exists; the derivation isn't firing.** That is itself the
  highest-leverage friction: the product's own "measure → distill → govern" loop is
  not closing on its own maintainers.

## 2. Recurring miss-patterns (observed, with evidence)

Six patterns account for most of the corrections. Each is a *class* of miss, not a
one-off — which is exactly what belongs in governance.

**P1 · Re-deriving a settled decision.** The universal-dereference model was
re-explained by the user across ~3 days; each session I re-derived a wrong
"client-specific dereference" and even scoped a fake decision into a plan doc. Root:
the canonical decision existed as a memory but **was not in the loaded index**, so it
never reached context. Cost: repeated re-litigation, a wrong design in a plan.

**P2 · Trusting a masked signal instead of reading the result.** `cargo … | tail`
and `make … | tail` make the pipe's exit code (tail's `0`) the reported status — a
failing build reported "exit 0" more than once this session; the E0063 compile error
was hidden until I read the output file. Prior sibling: `grep -c FAILED = 0` "passes"
even when nothing compiled. Root: verifying the wrapper, not the outcome.

**P3 · Scoping from the task title, not the resolved-design/schema.** Initial task
descriptions were wrong until I grounded them: "wire the policies tab to
dojo.policies" (the resolved design says it stays a constant explainer), "rule-packs
buildable" (blocked on a non-existent personal namespace), the my-dojos count
(cross-tenant, deferred). Root: assuming scope before reading the spec's *Resolved
design* + the live schema.

**P4 · Blast-radius blindness on a shared type / schema change.** Adding one field
to `RelaySessionUpdate` broke **6 struct literals** (E0063). dbd **doesn't manage
FKs** (silent) and **deploys enum variants alphabetically**; a "destructive" flag
fired on benign `SET DEFAULT` re-applies. Root: not enumerating everything that
touches a shared type/table/enum before changing it.

**P5 · Claiming "done"/"corrected" before checking real state.** The projects screens
shipped honest-empty because the daemon populate-seam wasn't built; the transcript
"data correction" looked complete until a query showed **18 sessions whose rows were
cascade-deleted** (events survive) that aliases alone can't re-attach. Repeatedly
found fabricated fixtures reachable in prod (`knowledgeFor`, `orgProjectsFor`). Root:
declaring success against the code path, not the live data.

**P6 · Deploy / artifact staleness.** A new route method 405'd from a stale
incremental bundle; a new route's first anonymous hit served an edge-cached 404; a
daemon feature stayed invisible until a manual rebuild. Root: shipping the source,
not verifying the running artifact.

## 3. What's missing (root causes, not symptoms)

1. **The friction→constitution loop under-fires.** The single biggest gap: the
   analyzer isn't turning 21 corrections into clusters + candidate rules. If it did,
   P1–P6 would surface as learned rules automatically. (Analyzer follow-up: the
   `derive_signals`/correction-clustering path — see the analyzer memories — produces
   almost nothing here.)
2. **No enforced pre-flight before building.** P3/P4/P5 are all "I skipped a check
   that the codebase could have given me": resolved-design, blast-radius, live-data.
3. **Verification treats wrappers as outcomes.** P2/P6 — no discipline that the thing
   checked is the actual result (exit code of the real command; the deployed artifact;
   the live row).
4. **Memory hygiene.** P1 — canonical/settled decisions must be in the *loaded* index
   (MEMORY.md), not just on disk, or they get re-derived.

## 4. Distillation — constitution guidelines + skills

Two vehicles; most patterns want both a **rule** (the non-negotiable) and a **skill**
(the repeatable procedure that satisfies it).

| Pattern | Constitution guideline (rule) | Skill (workflow) |
|---|---|---|
| P1 | *A settled decision is recorded once, in the loaded index, and cited — never re-derived.* | `recall-canon`: before designing, check the index for a prior decision; if found, cite + apply, don't re-open. |
| P2 | *Verify the outcome, never a masked wrapper.* (mandatory) | `verify-outcome`: never trust a piped exit code; read the real command's status/output; assert the specific effect. |
| P3 | *Ground the spec's resolved-design + live schema before scoping.* | `ground-before-scope`: read the *Resolved design* + query the schema/data; restate the task from that, not the title. |
| P4 | *A shared type/table/enum change requires a blast-radius sweep first.* | `blast-radius`: enumerate every literal/call-site/FK/enum-consumer of the symbol before editing; fix them in the same change. |
| P5 | *"Done" means verified against live data, not the code path.* (mandatory — ties the no-fabrication rule) | `data-reality-check`: query the real state (rows, counts, artifact) that proves the claim before saying done. |
| P6 | *Ship = the running artifact is verified, not just the source committed.* | `verify-deploy`: clean-rebuild on interface changes; smoke the live endpoint (cache-bust); install the daemon as part of completion. |

Notes on placement:
- **P2 and P5 should be *mandatory* rules** — they're the FTR-killers (a false "done"
  and a false "green" both ship defects). The rest are *required/advisory*.
- These map cleanly onto sensei's own primitives: the rules become
  `sensei.playbook_rules` (source could be `learned` once the analyzer derives them,
  or `manual` if we seed them now); the skills become marketplace skills applied per
  workflow (build / deploy / schema-change / data-correction).

## 5. The meta-recommendation

The most durable fix is not to hand-write P1–P6 as rules — it is to **make the
analyzer's correction→cluster→learned-rule derivation actually fire**, so this
distillation is continuous and evidence-weighted (a rule earns its place by measured
correction reduction — the "measure, then keep what helps" mandatory principle).
Seed P2 + P5 as manual mandatory rules now (they're already proven this session);
fix the derive loop so P1/P3/P4/P6 and future patterns surface themselves. Then close
the loop the product exists to close: friction → governance → higher FTR, measured.

**Suggested next steps** (for decision): (a) seed the 2 mandatory rules + 6 skills
above; (b) file/prioritize the analyzer derive-loop gap (1 cluster / 0 learned rules
from 21 corrections) as the root fix; (c) re-measure FTR on `sensei` after a few
sessions under the new rules to confirm they move the number (drop them if they don't).
