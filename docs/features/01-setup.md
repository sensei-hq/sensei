---
name: Setup
type: feature
kind: functional
---

# Setup

A user's entry into sensei should be frictionless. The first launch does the
least it can to get someone working: it checks the machine has what it needs,
scans the folders the user's code lives in, and — when those repos point at an
organization — offers any shared dōjōs it recognizes. Everything else is
configuration that can be reached anytime after (see [Configuration](02-config.md)).

By default everything stays local. Nothing is shared unless the user opts in.

The entry gate is deliberately three steps:

1. **Health gate** — verify (and install) the prerequisites.
2. **Folder scan** — point sensei at where the code lives.
3. **Dōjō auto-discover** — recognize org-owned repos and surface matching dōjōs.

## Flows

1. **First launch.** Health gate → auto-fix anything missing (or one manual step
   if even Homebrew is absent) → Folder scan → Dōjō auto-discover → into the
   observatory. A short Welcome opens it; Enter closes it.
2. **Every later launch.** The health gate runs again; when everything is green
   it passes straight through to the app.

## Mockups

- [Health — a probe on dependencies, and resolution](../mockups/Sensei/lib/setup/bootstrap-splash.jsx)
- [Setup wizard — folder + scan stages](../mockups/Sensei/lib/setup/setup-wizard.jsx)
- Dōjō auto-discover — not yet mocked

## What's involved

> What the user sees and does. `- [x]` done · `- [~]` partial · `- [ ]` not
> started. The mechanics behind each step live in the
> [setup & config design module](../design/setup-and-config.md).

### Health gate

On launch sensei checks the machine has what it needs and sets up anything
missing, so the user doesn't have to.

- [x] A health check that shows what's ready and what's still missing
- [x] Installs the local pieces sensei needs — the database, the local model runtime (Ollama), and sensei's own components (cli, MCP server, background service)
- [x] Does the install for the user — no commands to run in the normal case
- [x] One clear manual step only if the package manager itself is missing
- [x] Works offline or behind a proxy
- [x] A re-check button; when everything is ready it opens the app

### Folder scan

The user points sensei at the folders their work lives in, and sensei organizes
what it finds. Someone might keep personal projects under `~/personal` and work
projects under `~/work`, or a mix under `~/developer` — they hand those folders
to sensei to scan.

- [x] Choose the folders your work lives in
- [x] sensei scans them and organizes what it finds into projects
- [x] Reads each codebase's structure and tech stack as it scans

**Why it matters.** The scan is the foundation for everything after. It lets
sensei judge each codebase later — the stage it's at (greenfield or brownfield),
code quality, test coverage, duplication, complexity, dependency depth — and,
from the tech stack, recommend corrections: a dōjō's quality principles, or the
bare minimum every project should have (linters, formatters, coverage in CI,
standard commands). A new team member or first-timer ends up with measurement
and basic hygiene locked in from the start. (The analysis and recommendations
themselves are separate features — see Observe and [Configuration](02-config.md).)

### Dōjō auto-discover

- [ ] After the scan, recognize repos that belong to an organization
- [ ] Match the org against known dōjōs
- [ ] Surface a "we found some dōjōs" prompt the user can log in to

_Joining a dōjō by invitation/URL and how membership is validated are
[Configuration](02-config.md), not part of the entry gate._

## Status

| Feature | Status | Notes |
|---|---|---|
| Health gate (install + dependencies) | Done | bootstrap crate + splash; six foundations, auto-fix + manual + offline paths |
| Folder scan | Done | roots (drag/browse/paste, recursive) → repos → code graph → watch; live SSE progress |
| Dōjō auto-discover | Not started | inspect remotes → tell personal vs org → match dōjōs → surface prompt; not built |
| Welcome / Enter bookends | Done | framing screens around the gate |
</content>
