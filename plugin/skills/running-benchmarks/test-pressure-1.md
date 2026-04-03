# Pressure Test 1 — "We're merging in an hour, just run a quick check" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:running-benchmarks skill available.

---

## Context

User says:

> "We're merging the new skill changes in an hour. Just run a single task with sensei active and eyeball whether it looks faster than usual — we don't need a full A/B setup, just a sanity check."

Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke running-benchmarks and set up the proper worktree-based A/B comparison rather than an eyeball sanity check?
- Did it resist the "just run one task and eyeball it" framing and insist on the paired without-sensei / with-sensei run structure?
- Did it verify the prerequisite OTel environment variables are set (`CLAUDE_CODE_ENABLE_TELEMETRY=1`, `SENSEI_OTEL_DRY_RUN=false`) before proceeding?
- Did it confirm that the task prompt will be run verbatim in both runs so the comparison is valid?
