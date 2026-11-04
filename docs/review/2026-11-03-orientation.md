# Orientation: where we are, what gets us to 80% this week (2026-11-03)

Written by the reviewer/coordinator for every stream: Muse (sync WP1–9), the diagrams session,
Claude Cowork (Notice, the dōjō sync screen, billing), and the tracker-integration helper.
It is built from four read-only audits (PRD, roadmap vs code, navigation/screens, journeys). Status
claims were checked in code where marked. Reviews of each handoff go next to this file as
`docs/review/YYYY-MM-DD-<slice>.md`.

## 1. The product in one line, and the test every screen must pass

Sensei is a **guide and mentor**. It observes code and AI sessions and turns what it sees into
**concrete next steps toward higher quality**. With workspaces, teams, tickets and sprints in place,
metrics are no longer only charts: they become **targets that gate work**.

**The screen test.** For every panel, ask what decision it drives and what the next step is. If the
answer is "none, it's informational", the panel is incomplete. That holds however good it looks.

## 2. Where we are: readiness by area

| Area | Ready | Built | Biggest gaps |
|---|---|---|---|
| Onboarding / health | ~75% | bootstrap, 5-step setup, Health + Remedy, Developer ID | first run never mentions the dōjō; #282 Keychain on rebuilt binaries; #300/#301 sign-in |
| Diagrams (local) | ~85% | S0–S4, S3 ER, S7 focus (committed) | S4 2c pooling double count; S6 history (proposal); the dōjō side (WP6/7) 0% |
| Metrics engine | ~75% | daily→quarterly views, perf work, My metrics, team views | **ActionItems has no action** (0 buttons/links); #156 architecture metrics missing |
| Metrics as **gates** | **~0%** | nothing (verified): no thresholds, no gating field, no ratchet | the M-7 gating field, a target store, evaluation, a failure flow |
| Sync (WP1–9) | ~40% | WP1–3, WP4 staged | WP4 F4, WP5, WP8 (D46), **WP9A set-based registration**, #314 production verify |
| Governance | ~50% | scope resolution, rule packs + adoption, resolved rules pulled | no violation/override store (G-GOV-AUDIT); no "what adopting changed here"; team scope #318 |
| Learnings | ~60% | memories, Learnings screen, share-review, k≥3 promotion, playbook rules | no `report_learning`/`report_memory_use`; no failure → lesson loop; too many competing verbs |
| Trackers / sprints | ~35% | Jira (daemon), boards per level (D40), sprints, Tickets screen | GitHub Issues / ADO adapters; ownership conflict (below); epics unread |
| Teams / viewers | ~70% | roles, teams, invites, viewers, lead view | **no viewer shell**; leads have no menu of their own; A4 writable workspaces |
| Billing / plans | ~15% | billing_accounts, seats, ScrBilling | no provider, no subscription table; D46 is WP8 |

Overall this is about 60%, which matches your estimate.

## 3. What "80%" should mean this week

Not more screens. Get **one spine to work end to end**:

> A developer installs Sensei and gets actionable quality advice → connects to a dōjō from first run →
> their repositories sync, **verified in production** → an org lead sets **targets** on a workspace →
> the sprint shows which targets pass or fail → a failing target tells the developer exactly what to fix
> (in the desktop app, next to the code) → the fix, or a repeated miss, becomes a **learning** that can
> become a rule.

That spine touches every core area: sync, governance, metrics, learnings and trackers. It also closes the
journeys' biggest dead ends. Everything not on the spine waits (WP6/7, S6, billing payments, Wave B/D).

## 4. Order of work (forward-only dependencies)

**Day 1 (today/tomorrow): decisions and unblocking**
1. You: answer the 5 decisions in §7. Gates cannot be specified without #1 and #2 there.
2. Muse: land WP4 (F4 `having count(*) >= 2`, unstage `packages/codemap/focus.*`). Then **WP8** (it
   changes `all_my_repositories`, which WP4 reads, so run WP4's suite again after it). Then **WP9 Part A**.
3. Reviewer (me): write the **targets & gates spec** `docs/spec/dojo/targets-and-gates.md` from
   T3:122–139 plus the mockup's "Set targets" stage, with claims verified (sensei-claims-verifier).
4. Cowork: mockups for **Set targets** (workspace tab) and **a failing target** (sprint scorecard row →
   desktop notice → the fix). Use the existing Notice and keep it consistent with the Notice streamlining.

**Days 2–3: build the spine**
5. Claude Code: the **M-7 gating eligibility** field on the metric catalogue, then a target store with the
   ladder from §7 #2, `block | warn | off`, and a ratchet. Then evaluation per sprint over the tickets the
   tracker has linked. Red-first SQL and Rust tests, the same access shape as WP4.
6. Diagrams session: **#156 architecture metrics** (cycles, zone of pain, coupling) as metric rows. The
   analysis already exists in `crate::analysis` for the lenses, so this is reuse, not new work. It unblocks
   4 of T3's 7 gate targets. Fix S4 2c first.
7. Claude Code: **metrics that act**. ActionItems get real actions (open the file or lens, create a memory,
   propose a rule). Every metric card names its next step.
8. Muse: **WP5** workspace metrics screen. It must show targets beside the series once 5 lands; build the
   series part now.

**Days 4–5: close the loop, verify, document**
9. Claude Code: the **failure → learning** loop: `report_learning` + `report_memory_use` MCP tools; a
   repeated gate miss proposes a memory or rule (through the existing `playbook_rules` path, no new
   store).
10. Muse: **#314 production verification** of sync (needs WP9A). Done means checked against live rows.
11. Reviewer: the **PRD consistency sweep** (§6) and the menu changes (§5) once approved.
12. Release (patch/minor) from main, after the release checklist.

**Not this week:** WP6/WP7 (dōjō diagrams), S6 history, payments, Wave B (dōjō→sensei pull), Wave D (DORA).

## 5. Proposed menu order (the journeys imply it; today's rail fights it)

The journeys run first run → daily solo → **connect** → team (personal) → team (org). Today the desktop
rail puts Dōjō/Sharing **last, under Review, after Logs**, and splits the dōjō over three screens
(`/dojo/connections`, `/dojo/sharing`, `/settings/dojo`).

**Desktop main rail (proposed)**
1. Today
2. Workspaces
3. **Act**: Insights · Upgrades · Traceability · Memories · Impact (today's "Needs you", renamed to a verb)
4. **Dōjō**: one entry (connection status, Sharing, Share review). It merges the three screens; the
   connection settings link from it
5. **Review**: Sessions · Instruments · Consolidation · Libraries · Sources · Logs
6. Settings

Also: a final, skippable **"Join your dōjō"** step in setup. The journeys put it right after first value.

**Workspace window (proposed):** Overview (recommendations first) · Intake · **Metrics & targets** ·
Tickets · Memories · Impact · Traceability · Patterns · Diagrams ▸ · then reference: Sessions · Libraries ·
Instruments · Integrations · About.

**Dōjō org (proposed):** Home (a "needs you" queue, not a summary) · Workspaces (Targets · Sprints ·
Metrics) · Teams · **Govern** (Constitution moves here, beside Triage · Approvals · Knowledge) · Admin.
Plus two role views that do not exist today: a **lead** view (my teams → per-person, per-workspace) and a
**viewer shell** (only the workspaces granted → metrics and reports, nothing else). Today a viewer lands
in the personal menu with no org view at all.

## 6. Docs and PRD: what to fix (sweep in §4 step 11)

**Structural (fix first, so it stops recurring):** the module template's "Signals shown" asks only what a
number *means* (`modules/README.md:81`). Add a required **"Decision it drives / next step"** column.
17 of 31 Signals tables have no action word at all (M8, M10–M12, M19, M20, T1, T3, T4…).

**Contradictions to resolve:**
- Projects → workspaces is half applied: M15:30, M16:24–48 and :105, T2:64/78/87, T3:72/209.
- Client binding (T3:113, T3:225, T2:118, objectives:76–78) vs D34 ("a client is just another dōjō")
  and D11/D20 (dōjō = org | personal).
- The manager persona (personas:92–98, "holds quality gates") vs D8 (a manager is a read-only viewer;
  maintainer+ set gates).
- No lead persona (D38). decisions.md:151 still defines Lead as the client-confidentiality role and is
  not marked superseded.
- Governance precedence: M13:11–12 ("higher rung wins regardless of scope") vs M13:79–81 and D25 ("most
  specific scope wins").
- The ladders disagree: governance dōjō→workspace→repository; gates dōjō→workspace→sprint→task type→task
  (no repository, no epic, no team); memory workspace→user→org→collective.
- Pairing (vision:115 "no pairing" vs objectives R6); self-hosting (vision:133 vs objectives:72);
  the solo persona "no dōjō" vs the free personal dōjō (D46); T3:155 lists code hosts as trackers; P2
  still points at `website/`.
- Tracker ownership: the PRD mixes D2 (dōjō-owned) with D22 (daemon pulls); the code is daemon-only Jira.

**Missing:**
- The sync spec of *what* syncs (metric, graph, diagram payloads), its cadence, its failure and empty
  states, and how an unshared repository behaves.
- Gate failure remediation (beyond "close with a note").
- A per-metric gating-eligibility list.
- A measure of governance adoption (are adopted rules followed, and with what effect).
- The failure → lesson loop.
- D40 (a level is its own board) is not in the PRD.

**Stale status docs:**
- Roadmap 01–05 status cells are all placeholders.
- `backlog.md` is frozen at 2026-10-13.
- CHECKPOINT says WP3 is unpushed (it is pushed).
- §1 of `dojo-sync-handoff.md` still lists G1/G2 (fixed by WP1/2).
- Issues #184 and #234 may already be fixed; check before closing.

## 7. Decisions needed from the user (each with a recommendation)

1. **Gate ladder.** T3 has Dōjō → Workspace → Sprint → Task type → Task.
   *Recommend:* add **Repository** between Workspace and Sprint, which matches D25 and the place where
   metrics live. Leave out Team: teams are groupings (D13), and they report target results but do not own
   targets. Leave Epic out of v1: epics have no reader yet.
2. **Naming.** "Gate" already means relay's human approval gates.
   *Recommend:* a **target** is a metric threshold. A target set to `block` **gates**. The UI says
   "target missed (blocking)", never "gate".
3. **Tracker ownership.** The journeys and D2 say the dōjō holds tracker credentials; the code connects
   Jira in the desktop daemon.
   *Recommend:* keep the daemon path for solo/personal use. For org workspaces, make the dōjō-owned
   connection (Wave C, which needs the #310 secret store) the target design. Tell the integrations helper
   now, before the GitHub Issues/ADO adapters are built on the daemon path.
4. **D3 vs ac7822d2b.** D3 says trackers stay read-only, but ac7822d2b writes usage back as a comment on
   each ticket.
   *Recommend:* make it opt-in per workspace (off by default) and record it as a D3 amendment. Or revert
   it, if read-only was a hard promise to clients.
5. **Menu changes in §5.** Approve, amend or reject. If approved, Cowork mocks it first and Claude Code
   implements it.

## 8. Standing review checks (every handoff)

Each review has these sections, even when one is empty ("checked: none"):

- **Pattern.** Presentation | state | load, worst at a boundary. Known hot spots:
  - Instruments (1,035 lines, `$effect` loads, the component writes state).
  - The dōjō `org/[slug]/[section]/+page.svelte` (~20 write closures, loads in `$effect`).
  - Six app pages that fetch in `onMount`.
  - `overview` passes `confirm()`.
- **Tokens.** Literals are near zero. The drift is hand-written CSS:
  - 78 scoped colour declarations in app `<style>` blocks (`AssistantCard` has 32).
  - 13 `font-family` stacks, one in `app/src/lib/tokens.css:39`.
  - Inline `style=` colours in Instruments and the codemap lenses.
  - Raw `oklch` in `app.css:59` and `dojo/tokens.css:45`.

  Fix these in rokkit upstream, not with app overrides.
- **Shared components.** The app and dōjō each have their own empty state, load error (ScreenState vs
  LoadError), banner (DaemonStatusBanner vs Banner vs Notice), page header (PageHeader vs Screen),
  sparkline (`kit/Spark.svelte` wraps `dojo/src/lib/components/Spark.svelte`; both are live. Corrected 2026-11-03: an earlier draft called the base file dead), and stat cards. **Cowork's
  Notice streamlining should take empty state, load error and banner into scope**, so the result is one
  `@sensei/ui` family.
- **Types across the boundary.** Same name, different meaning: `WorkspaceMetricRow` (app daemon row vs
  dōjō wire row), `Identity` (×3), `StatusMeta` (×2). Rename them when touched.
- **Actionability.** Every panel names its next step. Informational-only today: Sessions, Libraries,
  Sources, Logs, most diagram lenses, workspace Metrics, My metrics, Audit, Health, Teams, org Home,
  Knowledge.
- **Security.** Authenticate the caller before resolving the tenant (no existence oracle). Fail closed;
  a failed read is a 5xx, never `[]`. A definer function is registered in
  `definer_functions_gate_the_caller.sql`. Tracker secrets wait for #310.
- **Index hygiene.** One checkout is shared by many streams, so `git diff --cached --stat` must show only
  the stream's own paths.
