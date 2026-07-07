# The gate playbook — how the spec docs get executed

The five-section spec is useless without a discipline for applying it.
This directory names the **gates** — the concrete review + verification
steps that stand between "I read the spec" and "I shipped." Every gate
maps to a subagent that runs in an isolated context, so a bad
implementation can't smuggle itself past a self-review.

## The four gates

    ┌──────────────┐   spec-doc-reviewer      ┌──────────────┐
    │ Drafted spec │ ────────────────────────▶│  Reviewed    │
    └──────────────┘                          └──────┬───────┘
                                                     │
                                                     ▼
                                           implementation
                                                     │
                                                     ▼
    ┌──────────────┐   done-gate-verifier     ┌──────────────┐
    │ Implementation│ ───────────────────────▶│  Verified    │
    └──────┬───────┘   wrong-gate-hunter      └──────┬───────┘
           │                                        │
           ▼                                        ▼
      sensei-persona-reviewer                sensei-acceptance-tester
      (does it serve each persona?)          (end-to-end user journey)

The four spec-authored gates are:

1. **spec-doc-reviewer** — gate BEFORE coding. Confirms the spec is
   usable; punch-lists what's vague, missing, or contradicts the
   mockup.
2. **done-gate-verifier** — gate AFTER coding. Executes each item in
   the spec's "Done gate" section against the running daemon + app;
   returns evidence per gate.
3. **wrong-gate-hunter** — gate AFTER coding, run in parallel with the
   verifier. Actively probes for each item in the spec's "Wrong gate"
   section; done-verifier says "good state present", hunter says "bad
   state absent" — they are not inverses.
4. **sensei-persona-reviewer** (existing) — gate AFTER coding, once the
   two mechanical gates pass. Loads the personas from
   `docs/mockups/Sensei/…` and reviews the work through each one's
   lens.

There is a fifth, informal gate — **sensei-acceptance-tester** — for
end-to-end user-journey verification when a segment (not just a
screen) crosses the finish line.

## When to reach for which — the rulebook

| Situation | Gate to run | Then |
|---|---|---|
| Just drafted a new spec | **spec-doc-reviewer** | Fix any FAIL items before coding. |
| About to implement against an existing spec | **spec-doc-reviewer** | Even for a "draft" status doc — the DB shape may have moved. |
| Just finished implementing a spec | **done-gate-verifier** + **wrong-gate-hunter** in parallel | If both PASS: run **sensei-persona-reviewer**. If either FAIL: return to code. |
| A whole segment (e.g. Bootstrap, Observatory, Project window) is done | **sensei-acceptance-tester** | The segment-level end-to-end. |
| Suspect a regression on a shipped spec | **wrong-gate-hunter** | Fastest way to name the specific defect. |
| Uncertain whether a design choice actually helps users | **sensei-persona-reviewer** with the specific persona | Independent, unbiased read. |

## The unhappy paths

- **spec-doc-reviewer returns `not-ready`.** Do not implement. Fix the
  spec. Re-run the reviewer.
- **spec-doc-reviewer returns `needs-fixes`.** Small edits; run once
  more before coding.
- **done-gate-verifier returns `partially-verified: daemon not
  reachable`.** Start the daemon (`make crates-debug &&
  make install-service`) or point at the release binary. Do not
  declare done from a self-review.
- **wrong-gate-hunter returns `one-or-more-tripping`.** Read the
  root-cause guess. If the guess is wrong, respond in a
  `wrong-gate-hunter` follow-up call with an updated probe — do not
  spiral into ad-hoc debugging without narrowing.
- **sensei-persona-reviewer says the persona is not served.** This
  is the "the screen renders but the user gets nothing" verdict.
  Reconsider the spec, not the code. The mockup may have moved on
  from what the spec captures.

## Existing agents worth reaching for

These live in `marketplace/plugins/sensei/agents/` and are already in
the runtime:

- **sensei-analyst** — before drafting a spec that needs requirements
  clarity. Use when the mockup itself is ambiguous.
- **sensei-developer** — before coding a spec that touches unclear
  file placement or a duplication risk. Consumes the spec + the
  existing codebase.
- **sensei-ux-designer** — when a spec's visual/interaction claim
  disagrees with the mockup. Independent UX read.
- **sensei-persona-reviewer** — post-implementation persona check,
  already integrated into the gate playbook above.
- **sensei-acceptance-tester** — segment-level end-to-end.
- **sensei-devops-sre** — for pipeline specs that involve deployment,
  migrations, or reliability-critical changes (e.g. `pipeline/capture`).
- **sensei-security-reviewer** — for anything that crosses a trust
  boundary (Dōjō upstream/downstream, auth, client-work
  dereferencing).
- **sensei-performance-engineer** — for pipeline specs with volume
  concerns (`pipeline/analyzer`, `pipeline/capture`).

## Autonomous execution recipe

When running the spec queue autonomously (e.g. during vacation), the
loop per doc is:

```
for doc in $(next-N-todo docs/llm-spec/):
    invoke  spec-doc-reviewer(doc)     -> gate 1
    IF gate1 == not-ready: park + notify + continue
    implement doc
    invoke  done-gate-verifier(doc)    -> gate 2  ┐
    invoke  wrong-gate-hunter(doc)     -> gate 3  ┤ parallel
    IF gate2 or gate3 fails: park + notify + continue
    invoke  sensei-persona-reviewer(doc) -> gate 4
    commit + push
```

Each parked doc gets a `park/{doc}.md` note with the gate output so
the human can pick up where the autonomous run stopped.

## Adding a new gate

If you find a class of defect that neither gate catches, define a new
`.claude/agents/{name}.md` following the same three-part shape:

1. **Purpose** — what defect class it catches, in one paragraph.
2. **Procedure** — the exact steps + which tool calls to make.
3. **Report format** — the verdict, evidence shape, and rules.

Then add the gate to the table in this README. **Do not run gates
that aren't documented here.** The playbook is the schedule.
