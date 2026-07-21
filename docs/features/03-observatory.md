---
name: Observatory
type: feature
kind: functional
---

# Observatory

The observatory is where the user lives day to day. Setup gets sensei watching;
the observatory is both halves of that watching — the **behind-the-scenes
observation** of how the user and their assistant work, and the **daily UX** that
shows it back. Its job is simple: surface the one thing that needs a decision,
let the user act on it, and keep them in control of what — if anything — leaves
the machine.

Every area of the observatory follows the same shape: it **observes** the work,
**forms a finding**, and **offers one action** — Apply, Review, or Dismiss. The
rail is grouped so the day has a natural order:

- **Anchors** — where every day starts (Intake · Today · Projects).
- **Needs you** — anything with a pending decision. The daily payoff.
- **Review & diagnostics** — reached periodically, not every day.
- **Settings** — visited only when something needs changing (see [Configuration](02-config.md)).

A **Focus / All** toggle collapses the rail to just the anchors and "Needs you"
— what needs a decision, nothing else. Dōjō is always opt-in and always
previewed: nothing is shared without a confirmation.

## Flows

1. **The daily loop.** Open to Today → see the one thing that needs you → act
   (Apply / Review / Dismiss) → drill into a project when you want more.
2. **Focus mode.** Collapse to anchors + "Needs you" and clear the queue.
3. **Give back (opt-in).** A finding is reviewed, then shared upstream to a dōjō;
   approved knowledge comes back as an Upgrade — every step previewed.

## Mockups

- [Sensei Journey Map](../mockups/Sensei/Sensei%20Journey%20Map.html) — the reference journey (segments · the daily loop · module lifecycles)
- [Observatory shell — the rail + section switch](../mockups/Sensei/lib/observatory/observatory.jsx)
- [Today](../mockups/Sensei/lib/observatory/observatory-today.jsx) · [Intake](../mockups/Sensei/lib/observatory/intake.jsx)
- [Insights](../mockups/Sensei/lib/observatory/learnings-v2.jsx) · [Memories](../mockups/Sensei/lib/observatory/learnings-anatomy-v2.jsx)
- [Impact](../mockups/Sensei/lib/observatory/impact.jsx) · [Traceability](../mockups/Sensei/lib/observatory/traceability.jsx)
- [Sessions](../mockups/Sensei/lib/observatory/sessions-zen.jsx) · [Libraries](../mockups/Sensei/lib/observatory/libraries.jsx) · [Instruments](../mockups/Sensei/lib/observatory/instruments-simple.jsx)
- [Dōjō touchpoints — connections · sharing · upgrades](../mockups/Sensei/lib/dojo/dojo-inapp.jsx) · [Sharing settings](../mockups/Sensei/lib/observatory/collective-settings.jsx)

## What's involved

> Each section, and what it's for. `- [x]` done · `- [~]` partial · `- [ ]` not
> started. The section objective is the sentence; sub-notes are what the user does.

### Anchors — where every day starts

- [x] **Intake** (門) — the front door: describe a chunk of work and sensei recommends a way of working (a playbook), then confirms before anything runs. A chunk belongs to a project, so intake's home is the [project window](04-project.md) → see its Working style section; deep design: [playbook module](../design/playbook.md).
- [x] **Today** (家) — the day's one thing: what needs a decision now, surfaced first, so the user doesn't have to go hunting.
- [x] **Projects** (場) — every project sensei found, with health at a glance; open one to work inside it (the project window is its own deeper surface).

### Needs you — the daily payoff

- [x] **Insights** (今) — turn the analysis into action: mentor-voice findings, triaged, each with Apply / Review / Dismiss.
- [x] **Memories** (覚) — see and curate what sensei has learned — facts, decisions, patterns — including how a memory is promoted.
- [x] **Impact** (果) — measure whether guidance worked: verdicts on applied changes, with regression alerts.
- [~] **Traceability** (巻) — keep docs and code aligned: requirement / doc ↔ code linkage, what's covered, what drifted.
- [~] **Upgrades** (贈) — receive vetted improvements: approved knowledge distributed back from a dōjō, accepted in-app.

### Review & diagnostics — reached periodically

- [x] **Sessions** (録) — review how work actually went: the digest of assistant sessions — prompts, tool calls, outcomes.
- [x] **Libraries** (庫) — ground the assistant in the libraries it uses: docs + code indexed and wrapped with sensei's tools.
- [~] **Atlas** (図) — see the shape of the codebase: the code + architecture graph — structure, calls, communities.
- [~] **Instruments** (具) — see and try the MCP tools sensei can reach for: a playground, replay of real use, and health.
- [x] **Logs** (診) — see what sensei is doing: background activity and diagnostics.
- [~] **Dōjō** (結) — join the shared plane: connect to an employer / client / community dōjō (addition).
- [~] **Sharing** (群) — give back to the collective: contribute learnings upstream, always previewed.
- [~] **Share review** (送) — control what leaves the machine: review exactly what's queued to share before it goes.

### Cross-cutting

- [x] **Focus / All** — Focus collapses the rail to anchors + "Needs you".
- [ ] **Dōjō discovery** — surface dōjōs from scanned repos' orgs (mockups cover discovery + addition; addition is partial, discovery is future — see [Setup](01-setup.md)).

## Behind the scenes — what sensei observes

sensei's premise is that it's an **observer**: it watches every interaction
between the user and their assistant, and what the assistant does to the code.
That observation is what lets it answer the questions that matter:

- is the instruction effective?
- is the user struggling, or getting what they wanted?
- can the assistant find its way around the code?
- is the code any good? — modular, using known patterns (OO / functional /
  patterns.dev), free of duplication, using the right stack-specific MCP, well
  tested (TDD, coverage), following standards (linter, formatter), built for the
  known personas, and in alignment with the docs?

### Capture — the raw signal

- [x] Session capture — prompts, tool calls, edits, outcomes (Claude first; Zed too)
- [x] Transcripts + events ingest
- [x] MCP — configure and watch
- [~] More assistants beyond Claude and Zed
- [~] Project-level observation — scaffold a project on stack best-practices, stack-specific tooling (MCP), and project knowledge (graph · context · backlog · progress · what's next)

### From capture to what you see

Capture is the raw signal. Three stages turn it into the findings the sections
above surface (per-capability statuses live in the [capability map](README.md)):

- **The map** — the code + activity graph, kept live by the watcher, plus semantic search and context assembly. → powers **Atlas** and **Libraries**.
- **Synthetic analysis** — the analyzer runs passes over each project: duplicates, communities, document drift, architecture, traceability, and metrics (FTR · churn · correction-prone). → powers **Insights**, **Traceability**, and **Impact**.
- **Knowledge** — what the analysis keeps: memories (with promotion), patterns and anti-patterns, and conventions. → powers **Memories**.

## Status

| Section | Status | Notes |
|---|---|---|
| Intake (front door) | Done | recommend-and-confirm a playbook; e2e green |
| Today | Done | the day's one thing |
| Projects | Done | index + health; deeper project window is a separate track |
| Insights | Done | mentor-voice findings, triaged, Apply/Review/Dismiss |
| Memories | Done | facts/decisions/patterns + promotion |
| Impact | Done | verdicts on applied changes + regression alerts |
| Traceability | Partial | requirement/doc ↔ code linkage in progress |
| Upgrades | Partial | dōjō distribution back, opt-in accept |
| Sessions | Done | assistant-session digest |
| Libraries | Done | docs + code indexed and wrapped |
| Atlas | Partial | code + architecture graph |
| Instruments | Partial | playground / replay / health in flight |
| Logs | Done | activity + diagnostics |
| Dōjō connections (addition) | Partial | connect screens shipped; membership validation not built |
| Sharing / Share review | Partial | contribute + preview; opt-in |
| Focus / All | Done | rail mode toggle |
| Dōjō discovery | Not started | screens mocked; build is future |
| Capture (sessions · transcripts · events · MCP) | Done | Claude + Zed; prompts · tool calls · edits · outcomes |
| Project-level observation | Partial | scaffold done; stack tooling + project knowledge partial |
| Map · analysis · knowledge pipeline | Done | powers Atlas · Insights · Traceability · Impact · Memories (statuses in the capability map) |
</content>
