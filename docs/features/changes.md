---
name: Feature changes
type: reference
---

# Feature changes

Notable changes to how features are shaped — the *why* behind a restructure, so
the feature docs themselves can stay current-facing (they describe what we want,
not where things came from).

## Setup simplified to a single gate (2026-07)

**What changed.** Setup was originally a multi-gate wizard — a long linear
sequence that also carried assistants, profile, projects, libraries,
instruments, routers, and model assignments before the user reached the app.
That is gone. The entry gate is now three steps: health gate → folder scan →
dōjō auto-discover. Everything else moved to a separate, free-navigation
[Configuration](02-config.md) surface, reachable anytime and never blocking.

**Why.** A journey-map analysis found the multi-gate wizard put heavy friction
between the user and value — they had to answer many questions before seeing
anything of their own. The principle became **value before setup**: the first
thing the user sees is their own projects, not a wizard. Tuning stays available
but is never a gate.

**Follow-up.** The code still has remnants named `setup` (the config routes live
under `app/src/routes/(config)/setup/`). Renaming/cleaning those up is tracked in
the [backlog](../backlog.md) (Cleanup / tech-debt).
</content>
