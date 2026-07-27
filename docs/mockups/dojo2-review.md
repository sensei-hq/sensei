# Dōjō2 review — the live design/build worklist

Review of the built dōjō2 shell against the mocks (`Sensei/lib/dojo2/dojo2-app.jsx`) and
the stated intent: **one inbox of in-flight sessions → click → the session's plan +
progress**, kept flat and simple. Durable design rules are separate —
[`../design/mockup-brief.md`](../design/mockup-brief.md).

Each item names its data shape (real relay data), what to surface, and whether the fix is a
**mock** change or a **build** change.

Real shapes (from `dojo/src/lib/relay-data.ts`, backed by `dojo.relay_sessions` /
`relay_segments` / `relay_inbox`):

- `RelayRun` = `{ run_id, title, goal, status, progress_done, progress_total, current_phase, current_feature, last_event_at, heartbeat_at, paused_until, pause_reason }`
- `RelaySegment` = `{ id, parent_id, seq, title, summary, detail, agent, model, spec_ref, state, is_gate, gate_severity }`
- `RelayGate` = `{ id, run_id, run_title, segment_id, kind(approval|decision|chat|nudge|stall), payload(stripped prompt + form), created_at }`

## Status

The three mock-change asks have landed **in the mocks**. The implementation has **not** caught
up on the inbox model — it still ships the pre-inbox nav (`Live runs · Approve · Decide · Chat`)
and the plan/inbox visualizer components are unbuilt. Full mock↔impl comparison and the build
worklist: [`../analysis/2026-07-27-mock-vs-impl-gap-analysis.md`](../analysis/2026-07-27-mock-vs-impl-gap-analysis.md).

| # | Item | Kind | State |
|---|------|------|-------|
| 1 | Collapse runs / approve / decide / chat + Needs-you → one Inbox | mock | **done** |
| 2 | Flatten the over-nested org nav | mock | **done** |
| 6 | Landing = the Inbox | mock | **done** |
| 3 | Terminal run shows stale "Running 1/3" | build | fixed (`5f0c0e1c`), needs daemon rebuild |
| 4 | Fixture screens show invented data | build | open |
| 5 | Legacy `(console)` duplicates dōjō2 | build | gated (D-CUTOVER) |

## What landed (mock)

**One Inbox (§1).** `NAV_YOU` is now `Inbox · Projects` (Work) · `Constitution · Rule packs`
(Govern) · `My dōjōs · Contributions` (Dōjōs). The `Relay` group and the standalone
"Needs you" band are gone — approve / decide / chat are actions answered inside a session
detail (`/you/runs/[run_id]`), not surfaces of their own. The inbox row carries a
"needs you" badge instead.

Inbox row shape (one per in-flight session):
```
InboxRow = {
  run_id, title,
  status,                       // running | paused | stalled | blocked | done | failed
  progress: { done, total },    // from segment states
  needs_you: number,            // pending gates for this run (RelayGate where run_id ==)
  attention: 'gate' | 'stalled' | 'blocked' | null,  // why it surfaces first
  last_event_at, heartbeat_stale: boolean            // heartbeat_at older than ~5 min
}
```
List surfaces: title · status chip · progress (`1/3`) · needs-you badge · relative
last-activity. Sort: needs-you + stalled/blocked first, then running, then terminal.
Detail (click a row): the plan outline (phase → task `RelaySegment`s with
state/agent/model/spec_ref), progress, and the pending gates answered in place.

**Flat org nav (§2).** The org zone collapsed from 14 top-level sections to three everyone
sees — `Home · Constitution · Projects` — plus one destination per zone of responsibility,
role-gated, each expanding to tabs in-page: **Governance** (Triage · Approvals · Knowledge,
maintainer) · **Clients** (Engagements · Incidents · Client-audit, lead) · **Admin**
(Members · Roles · Scopes · Identity · Audit · Health · Billing, admin). One level of
nesting instead of fourteen rail items.

**Landing = Inbox (§6).** Personal landing is the Inbox (option a — one fewer surface); org
landing is `home`.

## Outstanding — build

**§3 · Stale terminal status.** A completed run showed "Running 1/3" — the daemon marked it
done but stopped federating once it left the active set. Fixed: `report_run_outcome`
enqueues a final publish (`5f0c0e1c`); needs a daemon rebuild to go live. No mock change —
the inbox `status` already carries the right value once federated.

**§4 · Fixture screens show invented data.** These render mock arrays to a real user:

- `/you/projects` (+ `[id]`) — mock repos (`lumen-auth`, `ledger-core`), no backend. Shape:
  `Project = { id, name, repo, dojo?, last_activity, open_runs }`, federate the daemon's
  `list_projects`. For now: a real empty state until a `/v1` projects route exists.
- `/you/contributions` — hardcoded `approved 2 · pending 1 · helped 612`, no backend. Shape:
  a promotions ledger `{ rule, scope, status, adopted_by_count }`. For now: empty state; drop
  the `612`.

Empty states now; the `/v1` read-routes are a Tier-3 follow-up. The mocks are fine — the
wiring is missing.

**§5 · Two live IAs.** `(console)/console/*` (18 routes) still ships, orphaned, duplicating
every org-admin surface and the run detail. D-CUTOVER cleanup: redirect `/console/*` → the
dōjō2 equivalents, then delete. Gated — not this pass.

## Outstanding — design

The three planes are one loop: **Dōjō defines → Sensei applies → the impact shows in Sensei
→ contributes back up → Relay supervises throughout.** A change on one plane must show its
counterpart on the others.

**Sensei — the impact surface (app plane).** Where adopted governance and learnings land and
show impact — the plane most likely to be under-designed. Design the surfaces that close the
loop back from Dōjō: *what governs this project* (the resolved constitution in-app), *did it
help* (rule → measured effect: FTR / churn / correction signals), *traceability*
(rule/decision → the code or PR it shaped), and the *contribute-up* touchpoint (a learning
formed here → shared to the Dōjō, with the anonymize/preview flow). Reference `lib/observatory/`
+ `lib/project/`; keep on the design system (`assistant-card.jsx`) and the Rokkit migration.

**Relay — execute & supervise (revive it).** Relay was implemented (phone UI +
segment-publish + hook-gate; the daemon holds a live line to the Dōjō over Supabase realtime)
but a UX change broke the integration. Reviving it also gives the live phase/checkpoint +
nudge supervision. Design against the rebuilt relay kit (`RunCard` / `GateCard` /
`DecisionCard` / `ChatThread` / `NeedsYouBand`): the live run view (phases done/doing/next +
activity + a "needs you" band), approve a gated command, decide (options + free reply), chat
to steer, nudge — identical on phone and console, ranked by what's blocked on you. Confirm the
data path end-to-end (daemon → Worker `/v1/t/{tk}/relay/*` → phone/console); the break is
likely where the dojo2 relay UI meets the daemon publish/gate wiring.

**Governance — rule packs + the ladder (current focus).** Each pack carries an **area**
(principles · architecture · security · compliance · tech-stack · design · process), a
**scope** on the ladder (`organization · team · project · stack · personal` — "organization"
replaces company/client, which is the viewer's per-membership relationship), an
**enforcement** level (advisory / recommended / required / mandatory — drives precedence and
whether a rule is always-injected vs on-demand), a real **source** (Robert C. Martin, OWASP,
PCI SSC, Rokkit for Zen-Sumi, Gang of Four…), and **rules[]** each `{ text, detail?, hard?,
checker?, skill? }`. A pack row is an at-a-glance summary (kanji · name · area chip · scope
chip · source · "N rules ▾") that expands to the rules. The constitution/ladder preview shows
the resolved ladder for a project, conflicts settled, mandatory locks, and per-rule provenance
back to its authoring scope. Enforcement is the discriminator for the instruction-delivery
model ([`../design/instruction-delivery-model.md`](../design/instruction-delivery-model.md)).

*Open bug:* the **adopted pill** doesn't match the mockup in dark mode — needs a real
`success-edge` border token (dōjō lacks the `-edge` tokens today) plus the mockup's
`check-circle` and success tone/soft/edge chip. Small, independent.

**Re-added org consoles.** Eight consoles brought into the dojo2 IA as tabs (Triage,
Approvals, Knowledge, Engagements, Incidents, Client-audit, Identity, Health) still need the
dojo2-kit re-skin. Routes + backend already exist; the job is the re-skin and role-grouped nav
destination.

**Website.** The old review flagged copy accuracy (privacy over-claims; export / assistants /
instruments claims), a roadmap + waitlist beat, and responsive. Verify current state before
actioning — much may be shipped. Lower priority.

## Coverage directives (from the 2026-07-24 coverage audit)

Full audit: [`../analysis/2026-07-24-coverage-audit.md`](../analysis/2026-07-24-coverage-audit.md).
Triggered by the logout 404 — a designed-but-uncovered critical path.

- **Cover the five missing critical paths** as first-class Zen-Sumi surfaces, not framework
  fallbacks: 404 / not-found · `+error.svelte` boundary (failed load → calm page + retry) ·
  permission-denied (direct URL to a role-gated section) · session-expired / re-auth ·
  rate-limit (429). None exist today.
- **`EmptyState` is the law for empties.** The shared `kit/EmptyState.svelte` (空 · "Still
  listening.") is adopted by 7+ screens — always compose it, never hand-roll an empty branch.
- **Consolidate duplicated shells into named kit primitives** (data-driven props, no inline
  `padding` / `letter-spacing` literals): `Card` (~23 copies), `ListItem` (8+), `FieldLabel`
  (10+), `LabelWithIcon` (6+); fold `Eyebrow` into `SectionHead`'s eyebrow slot. Batch these
  and browser-verify by computed style — big-touch, don't sweep blind.
- **Shared logic → a util, not a copy.** `getInitials` (`kit/initials.ts`) is the template: a
  second copy of any computation is a refactor signal.
