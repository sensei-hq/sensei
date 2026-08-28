---
description: Stage 2 of 4 — turn an understood problem into a spec and a plan, with every claim about existing code verified before build can start
argument-hint: A spec path, an issue number, or the slice name from /sensei:analysis
---

# /sensei:design — stage 2 of 4

`analysis → **design** → build → complete`

Produce the spec and the plan. Then prove the spec is not built on something that is not true.

This stage owns the failure that costs the most: **the spec asserts something about the existing
codebase, it is wrong, and nobody finds out until implementation touches reality.** Two real
examples, both from one session, both in specs written by someone who knew the codebase well:

> *"Sign-in state lives only in the Keychain and nothing can list it."* — `sensei.personas` already
> listed it. The claim was the sole justification for a new table, which was designed, specced, and
> nearly built.

> *"`unpushed_metric_rows` is the one production push path."* — it had no production caller at all.
> The claim set the scope of an entire slice.

Neither is a reasoning error. Both are perfectly good prose. **A faceted spec review would have
approved both**, because reviewing prose about code is not the same as checking code. That is why
this stage has a verification step and not just a review step.

---

## Step 0 — Resume the thread

Run `/sensei:session` and read the checkpoint. Do not ask the user to re-explain what
`/sensei:analysis` already established; if the analysis output is not in context, read it from
`docs/` rather than reconstructing it from the argument.

If there is no analysis for this work, say so and run `/sensei:analysis` first. Designing from a
one-line ask is how an assumed scope becomes a spec.

## Step 1 — Write the spec

Use `/sensei:spec` (and `/sensei:blueprint` or `/sensei:mockup` where the slice is a screen). The
existing commands do the work; this stage sequences them and adds the gate.

Whatever else it contains, the spec must have:

- **Resolved design** — what we are building, decided, not a menu of options.
- **Done gate** — checkable statements. "Works correctly" is not one.
- **Wrong gate** — the specific ways this could pass its done gate and still be wrong.
- **Claims** — see below. This is the part that is new and the part that is load-bearing.

## Step 2 — Write the claims ledger

Go back through the spec and pull out **every assertion about what already exists**. For each,
write the check that would DISPROVE it and the result that would count as disproof.

```markdown
## Claims (verified 2026-08-28)

| claim | check | expect | actual | verdict |
|---|---|---|---|---|
| nothing can enumerate signed-in personas | `rg -c 'verified_at' crates/senseid/src/db/pg_store/personas.rs` | 0 | 3 | FALSE |
| `repositories.dojo_id` has no Rust reader | `rg -c 'dojo_id' -g '*.rs' crates/` | 0 | 0 | CONFIRMED |
```

Signals that a sentence is a claim: **nothing, only, never, always, the one, no X exists,
unused, cannot be answered today, 0 rows.** Absolutes are the highest-yield — a claim that
"nothing can list it" is one command from being disproved, and it is usually holding up a whole
section.

Statements about the FUTURE ("we will add") and preference ("it is cleaner to") are intent, not
claims. Do not ledger them; you will drown the table and the real claims will hide.

**A spec with no claims section is not finished.** Nearly every spec asserts something about the
current codebase — if you found none, you did not look.

## Step 3 — Verify, independently

Launch `sensei-claims-verifier` on the spec.

It re-derives each claim **without** reading your evidence first — deliberately, because checking
an author's citation in the author's frame reproduces the author's conclusion. Its verdicts are
not advisory:

- **Any `FALSE` → the spec is wrong. Stop and fix it.** Not a footnote, not a follow-up. Find
  everything that depended on the false claim and re-decide it. In the persona example, `FALSE`
  deleted an entire proposed table and replaced it with a one-line query.
- **`MISLEADING` → rewrite the sentence.** Technically-true-but-reader-will-conclude-wrong is how
  "no production caller" gets read as "no caller".
- **`UNCHECKABLE` → make it checkable or delete it.** A claim nobody can test is a claim nobody
  can maintain.

Record the corrected verdicts back into the ledger. A `FALSE` that was found and fixed is worth
keeping in the table with its history — it tells the next reader that this ground was contested.

## Step 4 — Faceted review

Now review the design itself, which is a different question from whether its facts hold. Launch in
parallel, one message:

- `sensei-plan-depth-reviewer` — is every piece deep enough to build unattended? No TBDs, observable
  criteria, pre-answered ambiguities.
- `spec-doc-reviewer` — are the required sections real rather than ceremonial? Do the done gates
  agree with the mockup and the DDL?
- `sensei-data-correctness-reviewer` — re-derive any computed value, key, or grouping the spec
  defines. A wrong key in a spec becomes a wrong key in three files.
- `sensei-security-reviewer` — only when the slice touches auth, identity, secrets, or a
  trust boundary. Say so if you skip it.

Same independence rule as `/sensei:review`: they must not see each other's findings, and a reviewer
that errored is `NOT RUN`, never a pass.

**Verify each finding yourself before acting on it.** Adversarial reviewers produce
plausible-but-wrong findings, and a spec is cheap to change in the wrong direction.

## Step 5 — Fix, then plan

Fix everything the verification and review turned up — the spec is prose, so the fix is a rewrite,
not a test. Then run `/sensei:plan` to register the plan graph.

Plan against the CORRECTED spec. Planning against the draft is how a false claim survives into a
task list, where it stops looking like a claim and starts looking like work.

## Step 6 — Checkpoint and hand off

Run `/sensei:checkpoint` with: the slice, the spec path, which claims were found false and what
changed as a result, and the exact next command. Mirror it into `docs/CHECKPOINT.md`.

Then report:

1. The spec path, and its Resolved design in two lines.
2. **The claims ledger** — how many checked, how many `FALSE`, and what each false one cost.
3. Review findings fixed; anything dropped in verification, with why.
4. Reviewers `NOT RUN`, named.
5. The registered plan, and the first task.

## The gate

`/sensei:build` should not start while any of these is true. Say which one blocks you.

- A claim is `FALSE` and the spec still depends on it.
- A claim is `UNCHECKABLE` and it is load-bearing.
- The done gate contains a statement nobody can check.
- A reviewer is `NOT RUN` and its mandate is plainly in scope.

## Wrong gate

Ways this stage passes and is still wrong:

- **The ledger only contains claims that were easy to check.** The load-bearing premise is
  usually the vague one. If nothing in the table is an absolute, you ledgered the footnotes.
- **The verifier confirmed everything.** Possible, but check that it re-derived rather than read
  your citations — an agreeable verifier is worse than none, because it launders the assumption.
- **Claims were verified weeks ago.** The repo moved. Re-run the ledger at build start; it is cheap.
- **The review passed because the reviewers were asked about the spec, not the code.** Prose review
  cannot catch a false premise. That is what step 3 is for, and it is not optional.
