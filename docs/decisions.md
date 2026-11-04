---
name: Decisions log — sensei
updated: 2026-11-03
---

# Decisions

> Append-only. One entry per decision: date · decision · why · alternatives.
> The anti-rework memory — never re-derive a settled choice.
> The **design journey** (approaches tried, what worked, what didn't) lives in
> [`history/analysis/2026-07-24-design-journey.md`](history/analysis/2026-07-24-design-journey.md).

---

## 2026-11-03 — PRD alignment round: D47–D70 (user, answering the orientation review)

Decided with the user over the 2026-11-03 orientation (`docs/review/2026-11-03-orientation.md`), in
order to make the PRD, journeys and specs consistent before the parallel lanes start
(`docs/plan/2026-11-03-parallel-lanes.md`). **F4 supersedes the entry below it** ("WP4 F4 needed no
new decision"): the user revisited D10 for viewer pools and chose to withhold them; teams and sprints
bring the cross-team comparison D10 named as its trigger. D57 was first stated wrongly by the
reviewer ("mandatory final, else most specific") and corrected to the code before recording.

| # | Decision |
|---|---|
| D47 | A **target** is a metric threshold. A target set to `block` *gates* work. The UI says "target missed (blocking)". "Gate" stays relay's human-approval word. |
| D48 | Target ladder: **Dōjō → Workspace → Repository → Sprint → Task type → Task**, most specific wins. Teams report results and own no targets. No Epic level in v1. |
| D49 | Tracker connections: **dōjō-owned for org workspaces** (Wave C, needs the #310 secret store); the daemon path stays for solo/personal use. New adapters (GitHub Issues, ADO) go on the dōjō path. |
| D50 | Ticket writeback (ac7822d2b) is **opt-in per workspace, off by default**. This amends D3. |
| D51 | The menu order in orientation §5 is approved: Cowork mocks it, then implements it. |
| F4 | A viewer pool with fewer than 2 contributors is withheld (supersedes D10 for viewer pools). |
| D52 | **A solo developer gets everything free**, blocking targets included, on their personal dōjō. **Paid targets are an org feature, and only for private repositories.** An org's public repositories get targets free. This follows D46's line (org private = subscribed) and narrows D24's "gating is paid" to org-private. |
| D53 | **The dōjō is the client; there is no client separation** (reaffirms D34). A session in a repository is governed by **that repository's dōjō** alone: its rules and its targets. No employer floor carries into another dōjō. A person's own `user`-rung rules still apply locally, below the dōjō by the scope ladder. Remove T3:113/225 and T2:118 ("client binding", "client's tracker and gates", "employer floor") and the membership-contexts line at objectives:76–78. |
| D54 | **Who sets targets:** Dōjō + Workspace levels = admin; Repository = maintainer+ on it; Sprint / Task type / Task = maintainer+ on any repository in the workspace. Leads read results per person (D38); viewers read aggregates only (D8). The manager persona splits: a member manager → maintainer or lead; an outside manager or client → viewer. |
| D55 | **No pairing.** The relay acts only through the signed-in person's dōjō session, with explicit, revocable permissions. R6 is rewritten to say this; vision:115 stands. |
| D56 | **The knowledge promotion path runs opposite to the governance ladder:** user → repository → workspace → dōjō → global shared. Each step outward is a promotion; it is never a precedence. Crossing out of a dōjō into global shared keeps k ≥ 3 contributors (D26/T1). `memory_scope` must gain the `user`, `repository` and `dojo` steps (today: global, workspace, stack, task_type, module). That is a schema item for the learnings lane. |
| D57 | **Governance precedence = the code** (`pg_store/governance.rs:61,339`: enforcement desc, then scope desc): the **stronger enforcement wins** (mandatory > required > recommended > advisory, and mandatory is never overridden); between rules of equal enforcement, the **most specific scope wins** (general → user → dōjō → workspace → repository). A repository can tighten a dōjō rule but cannot weaken it. (Corrected 2026-11-03: L0 had first stated it as "mandatory final, else most specific".) |
| D58 | **Lead** = the team lead (D38). `decisions.md:151` (Lead as the client-confidentiality role) is marked superseded. Add a Lead persona: sees teammates' metrics per person across the workspaces the lead can read; sets targets only if also a maintainer. |
| D59 | **Pricing** (P1–P3): SSO and audit are **free** (audit is governance, D19). The subscription is **per org, priced by the seats who work on its private repositories**; contributors to public repositories and viewers are never counted. The relay's team on-call queue (R8) is **free**. |
| D60 | **Vocabulary:** "solution" was the earlier name for a **workspace**, "org" in a ladder is the **dōjō**, "module" in a ladder is the **repository**. The multi-repository cascade is general → dōjō → workspace → repository. |
| D61 | **A missed blocking target is information plus a handoff, not a hard stop** (F1). Sensei says which targets are missed and what to fix, and hands the fix to the assistant to clear them. On the dōjō side a **lead or admin accepts the deviation** with a note, and may open a follow-up ticket for the issues the miss identified. Opening a ticket writes to the tracker, so it follows D50: opt-in per workspace; otherwise Sensei drafts the ticket for the lead to file. |
| D62 | **Targets reach the assistant through MCP** (F2), together with the ticket's own rules, at the start of work on a ticket. |
| D63 | **A ticket is the unit of work handed to the assistant** (F3). The sync lists the tickets assigned to the user. The user hands the assistant a ticket key; the assistant fetches it through a **tracker-agnostic MCP tool served by the daemon**. That fetch **marks the start of work on the ticket** and links the session to it. Branch/commit/prompt key matching stays as the fallback, for an agent that fetched the ticket elsewhere (e.g. `gh issue view`). |
| D64 | **A personal dōjō is created at sign-in** (F4). |
| D65 | **Learning verbs:** one verb, **Share →**, along the D56 promotion path; **Reinforce / Retire**, driven by Impact's verdict; "skill" becomes a memory kind (F5). |
| D66 | **Two misses of a blocking target on the same cause propose a memory** at the user step; it is offered for promotion up the D56 path once it helps (F6). |
| D67 | **"What applies here"** on the desktop Dōjō screen: the resolved rules and targets for the current workspace, each with its source (F7). |
| D68 | **A minimal viewer shell this week** (F8): the granted workspaces, **metrics first**, to judge whether the overall workflow is healthy; target results in aggregate; nothing else. |
| D69 | **The dōjō Home is a "needs you" queue** (F9): missed targets awaiting acceptance, a sync that has stopped, approvals. No email or Slack this week. |
| D70 | **Menus** (a, b, c): "Needs you" becomes **Act**; Constitution moves into the dōjō's **Govern** group; setup's "Join your dōjō" step is **skippable**. |
| D71 | **Ticket data is read live** (Q1): `get_ticket` and "assigned to me" are live tracker reads made with the person's own access. No person data is stored; the `tracker_tasks` mirror is unchanged. For GitHub Issues the daemon uses the person's own forge sign-in, read-only, until dōjō-owned connections (D49) exist; write-back is `spec/daemon/tracker-metrics-writeback.md`. |
| D72 | **warn vs block** (Q2): a `warn` miss is shown and needs no acceptance. A `block` miss is handed to the assistant, enters the "needs you" queue (D69), and needs a lead's or admin's acceptance (D61). |
| D73 | **"The ticket's rules"** (Q3) = the repository's resolved rules (D57) plus the ticket's own acceptance criteria. There is no ticket level in governance. |
| D74 | **Neutral Impact verdict** (Q4): the memory is kept by default, and Retire is offered. |
| D75 | **The paid plan is called "Pro"** (Q5, user). Messages name it: "Targets on private repositories need Pro." **The price is "to be announced"** until a payment provider exists; the `$12` code constant is not a decision. |
| D76 | **"What applies here"** (Q6) lives on the desktop Dōjō screen, scoped to the current workspace; Workspace › About links to it. |
| D77 | **SCIM is free**, along with SSO (Q7, D59). |
| D78 | **The dead client routing** (`crates/senseid/src/dojo/routing.rs` and its two `.feature` scenarios) is removed (Q8, D53). |
| D79 | **Govern shows to everyone, with a minimum role per item** (the D70 Constitution move): Constitution for every member; Triage, Approvals and Knowledge for maintainer and up. |
| D80 | **Narrower target levels may only tighten** (user): a level below can raise the bar or turn `warn` into `block`, never lower either. Loosening needs the role that set the wider level (D54). Resolution takes the strictest applicable level, which keeps targets consistent with D57's "stronger enforcement wins". |
| D81 | **Target measurement** (user, accepting the spec's Q1–Q5): the rule-check target is absolute (zero failing hard rules on the change's tree); a hard rule is a checker rule whose resolved enforcement is mandatory or required, resolved as `get_rules` resolves it (adoption overrides included); baselines are per repository, metric and path filter; sprints and task types use the tracker's own keys (no invented issue-type mapping); a coverage reading counts once 50% of guarded files appear, and a graph reading once the index is complete (`folder_completeness.subtree_complete`). |
| D82 | **"Moving the wrong way"** (user, WP5): compare the second half of the window with the first. A repository is flagged when a metric that has a direction (`sensei.metrics.direction` higher_better or lower_better) moved the wrong way by **≥ 10%**, and only when **each half has at least 5 days** of readings. `neutral` metrics are never flagged. The panel names the worst repository as its next step. Direction reaches the dōjō wire through L1's own `workspace_metric_series`. |
| D83 | **No fabricated values, anywhere** (user, 2026-11-03): showing a made-up number, list, plan or status when the real data is unavailable is misinformation. Every layer (SQL, daemon, MCP, both apps, shared packages) surfaces a failed read as an error state and never as a plausible value. The system-wide audit (`coord/audits/fabrication-2026-11-03.md`) assigns each failure-mask to its owning lane, and a ratchet guard in `make test-fast` stops new ones. Honest-empty is correct only when the data truly is empty. |
| D84 | **S6 (diagram history) waits until every sync works** (user): after #314's production verification, not this week. S6's Q1–Q6 stay open until then. |
| D85 | **The billing screen is Cowork's new mockup work of 2026-11-03** (user). The earlier mockups in `docs/mockups/Sensei/` are its reference. The decision-driven fixes (Pro, "to be announced", nothing free listed as paid) apply. |
| D86 | **What the paid tier sells, in words** (clarifies D24 and D20 after D47/D52): the paid benefit is **blocking targets on an organisation's private repositories**, plus integrations and consolidated metrics. "Gating" in D20/D24 means exactly that. A target that only warns, and any target on public or personal repositories, is free. |
| D87 | **One screen shell for every screen** (user, 2026-11-04). A sticky header (icon/kanji, title, description, a toolbar toggling info/stats/filters/sidebar, an optional action bar); an info section (3 columns); a stats row from a stats state (`{title, icon|kanji, value, description?}` on a 12-column grid, cards spanning 1–3 columns); a filter band (a snippet, or a schema rendered by `@rokkit/forms`); body and sidebar scroll independently; sticky footer. Kanji and icon glyph modes both supported. Generic parts go upstream: the shell and stats grid into `@rokkit/app` (app chrome), StatCard `description` into `@rokkit/ui`; glyph-mode wiring stays in `@sensei/ui`. It replaces the app's `PageHeader` and the dōjō's `Screen`. Design (Cowork) designs it from the diagrams (incl. ER) and the sync screen; L0 coordinates rokkit; L1 moves the diagrams first; every other screen adopts it when its lane next touches it. **Remembered panels:** every view registers a stable view id; the first visit opens everything, later visits restore what the user left. All view states load once at startup into a shared app state (desktop: `appState` config from the daemon, synced across windows), never per-screen storage. The rokkit shell takes panel state as a bindable prop and stays storage-agnostic. **Actionable stats:** a stat may carry a filter (or an `href`); clicking it filters the screen's data to that problem and marks the stat active; a stat with neither isn't clickable. **The shell is chrome only:** body and sidebar are per-screen snippets that keep the existing components (the tree view, ER and structure diagrams, the inspector) unchanged. Every screen gets free-form search. Adopting the shell loses nothing: existing specs and e2e stay green unedited, and each migration reports a before → after parity list. |

---

## 2026-11-03 — WP4 F4 needed no new decision: D10 shows viewer pools of one

The WP4 review asked whether a viewer-pooled metric row over fewer than N = 2
contributors should be withheld. No new call: **D10 already answers it** —
"Hide aggregates over very small groups? **No.** An aggregate over one
contributor is shown" (`docs/roadmap/09-dojo-expansion.md`, the #307 round that
also designed the workspace viewer, D8) — and **D26 later reaffirmed D10 stands
for metric aggregates**. The review's N = 2 recommendation was not applied; the
lone-day pool in `database/tests/dojo/repository_metric_series.sql` pins
"shown, carrying its rate".

| Decision | Locked answer | Why |
|---|---|---|
| **F4 follows D10** | `workspace_metric_series` shows pools of one to viewers, labelled by `props.pooled_rows = 1`. | Re-deriving a settled choice is how the log rots; D10 was decided with the viewer design in hand. |
| **Not the team-totals floor** | `team_metric_totals`' k ≥ 3-others floor (user, 2026-10-22) stays where it is: member-included team totals, where a reader subtracts their own number. Viewer pools are a different shape and keep D10. | The floor's rationale is the subtraction attack on totals that include the reader. |

Known edge, left open: a viewer who also contributes rows could subtract their
own from a pool of two. D10's revisit clause (comparison charts across people or
teams) does not cover it — a future call if it matters.

---

## 2026-10-29 — SDLC metrics from AI sessions and tracker history, written back like usage

The daemon derives ticket, sprint and project SDLC figures:
- defects, build runs, timing, rework and holds, from the hook events of a ticket's linked
  AI sessions;
- commitment, scope change, carry-over and velocity, replayed from Jira changelogs;
- predictability, and tokens and prompts per ticket.

The Tickets screen shows them. The 21 S9 keys can be mapped into write-back fields beside
the 13 usage keys (`spec/daemon/tracker-sdlc-metrics.md`, analysis D1–D11, amendments A1–A10).

| Decision | Why | Alternatives |
|---|---|---|
| **A null always carries a reason code** (`reason_codes` domain `sdlc_metrics`, served by `GET /api/reason-codes/{domain}`) | Never fabricate: an unknown figure must not pass for zero, and the screen shows the reason in words | Zeros or "—" for unknowns (indistinguishable from real values) |
| **Only a derivation lag holds write-back back** (A9) | A failed derivation is permanent until its inputs change, so waiting on it froze a ticket's fields for ever | Skip any ticket with an underived figure |
| **Rate- and calibration-derived figures are never written** (S9) | `effort_cost`, `rework_cost`, `expected_minutes` and `time_variance_minutes` depend on settings, not measurement; `estimated_points` is Jira's own field (`spec/daemon/tracker-metrics-writeback.md`) | Write them all |
| **The hourly rate is project-level only** (D3) | No per-person cost | A rate per person |

---

## 2026-10-28 — A slash command is a prompt

The transcript parser listed `<command-name` and `<command-message` among the injected
markers (#73 v1, 2026-06-22, no recorded reason), so a session begun with
`/opsx:explore P057542601-784` kept no prompt: it parsed as 0 turns, and ticket linkage,
which reads prompts, never saw the key.

| Decision | Locked answer | Why |
|---|---|---|
| **A slash command is a prompt** | A `user` record naming a command is the prompt `/name args` (`human_prompt_text`), and a turn boundary. What a command prints (`<local-command-stdout>` etc.) stays injected. | The person typed it, and its arguments are where a ticket key or a task goes. |
| **Re-read once** | Claude and Cursor transcripts are read again once (`transcripts.slash_prompts_v1`, through `reset_watermarks_once`). | Their watermarks say "read" for files parsed under the old rule. |

Effect: more transcript turns per session where commands were used. Rejected: keeping only
commands with arguments (a bare `/compact` is still the person's turn).

---

## 2026-10-26 — Usage metrics are written back to the tracker (superseded 2026-10-29 (tracker-sdlc-metrics): SDLC metrics too)

Each ticket's usage (sessions, active time, tokens, cost) was visible only in sensei. The user
reversed the read-only rule for that one thing: the local daemon writes it back after every
successful sync, on by default for every board, existing ones included
(`spec/daemon/tracker-metrics-writeback.md`). Supersedes, for usage metrics only (superseded 2026-10-29 (tracker-sdlc-metrics)), the 2026-10-14
"Read-only means read-only" row and roadmap/09 D3.

| Decision | Locked answer | Why |
|---|---|---|
| **One write path: metrics** | Sync never writes sprints, statuses or memberships, and the dōjō never writes to a tracker. The local daemon writes usage metrics, nothing else (`spec/daemon/tracker-metrics-writeback.md`; superseded 2026-10-29 (tracker-sdlc-metrics)). | The rest of the read-only rule still holds; only the figures people asked to see in Jira move. |
| **Only the assignee's daemon writes** | A ticket is written only by the daemon whose token owner is its assignee. The assignee is compared, never stored. | Two teammates who both mention a ticket would otherwise overwrite each other with partial figures. |
| **Mapped fields, else one comment** | A board with custom fields mapped (`sensei.tracker_fields`) gets those fields; otherwise one sensei comment per ticket, edited in place. | Users who set up a field want it there; a comment works with no Jira admin. |
| **Every sync, on by default** (superseded 2026-11-03, D50: off by default, a workspace opts in) | Written after each successful sync; a per-board switch turns it off. | Asked for by the user; a figure that lags the sync is a wrong figure. |

Rejected: a manual export action; writing from the dōjō; storing the assignee.

---

## 2026-10-26 — The store's row types live in `pg_store/mod.rs` (#161)

**Decision (user):** the typed row/argument types of `db/pg_store` (`NodeRow`, `FolderRow`, `EdgeTarget`,
`MetricRow`, `HookEventRow`, …) are declared in `pg_store/mod.rs`, the path every caller names. They are not
declared beside their writers and re-exported.

**Why:** the Rust indexer does not follow `pub use`. A re-exported row was a reference to an identity no
declaration mints, at every use. #161's conversions pushed the first-party dangling-reference ratchet
(`indexer::resolve`) to 1,304 against its 1,300 ceiling. Moving 22 types took it to 1,045, and the ceiling
was lowered to 1,060.

**Cost:** a row is read in `mod.rs`, away from the writer that destructures it. Each one is labelled with
its writer file.

**Alternatives considered:** teaching the indexer to resolve `pub use` (the right long-term fix; it is
cross-file, stage-12 work, listed in `docs/backlog.md`), and raising the ceiling (rejected: the ratchet
exists to stop exactly this growth).

## 2026-10-25 — A database per test (#183)

Every Rust DB test shared one `sensei_test`. A database-wide operation in one test (a heal, a prune, a rescan
sweep) destroyed a sibling's fixture mid-flight, and the failure surfaced in an unrelated module. Five gates
had been added after five confusing failures, and two tests were found passing only on rows other tests left
behind.

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **Each test gets its own database (user, 2026-10-25)** | The first `PgStore::connect_test` on a test's thread clones `sensei_test` (`CREATE DATABASE … TEMPLATE … STRATEGY FILE_COPY`); later connects on that thread reuse it; it is dropped when the thread exits, and clones over two hours old are swept at the next run's start. `sensei_test` is the TEMPLATE, written by no test; `make test-template` rebuilds it. `SENSEI_TEST_SHARED_DB=1` runs against it directly. | Isolation by construction instead of a gate added after each collision. Cost: every clone forces a checkpoint, so clones serialise (~0.15s each on a 16 MB template) and the suite runs slower than the shared 184s. Rejected: keep sharing and gate each collision (five gates and counting, each found by debugging); per-test schemas (every table is schema-qualified); per-test transactions (the code under test commits through its own pool). |

---

## 2026-10-19 — Transcript reading on by default; token usage linked per turn (#280)

Token capture went silent on 2026-10-07. #218 had made reading conversation history its own consent, **off by
default**, and nobody had granted it. Meanwhile the #238 usage mod's per-turn usage was stored as events and never
summed into sessions.

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **Transcript reading defaults to ON — supersedes #218's default** | A missing `transcripts.consent.<source>` key means yes; an explicit `off` means no. The per-assistant switch in Settings → Assistants stays. When reading is off, the metrics that depend on it say so instead of going blank. | Core functionality (token, cost and session metrics) depends on it, and #218's default left every metric blank without saying why. The switch stays because a user can have real grounds (client confidentiality, an employer's rule). Open: whether to allow off at all. Recorded, not yet decided. |
| **Token usage is linked per turn, counted once, sourced** | `activity.turn_usage` holds one row per turn: the split counts, the model and `agent_id`, which sources saw it (hook, transcript, both) and whether they agreed. Main-loop hook turns (`turn.complete` without `agentId`) pair with transcript turns by order and time; agreeing usage is one value, and disagreement is flagged, never averaged. Subagent turns come from hooks (sidechain transcripts are not ingested). `sessions.tokens_*` are sums over it. | History needs transcripts; live sessions and subagents need hooks. The two must not double-count. No shared id exists: the hook's `turnId` is not in the transcript, and the transcript's `requestId` is not in the hook. Rejected: hooks only (loses history); transcripts only (blank without consent, no subagents); one total per source per session (no subagent attribution, no per-turn check). |

---

## 2026-10-17 — Rust compiler located without rustup (nix / distro toolchains)

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **`PATH` fallback for the compiler binary** | `CompilerEngine::compiler_on_path()` (default `None`). The Rust engine names `rust-analyzer`. The session runs `rustup which rust-analyzer` **first**. Only if that fails does it take the first **executable** `rust-analyzer` on `PATH`, resolved to an **absolute path** (`compiler_session::resolve_compiler`). | A nix toolchain ships `rust-analyzer` with no `rustup`, so every rust-analyzer compile failed on Linux/nix (6 tests, and the indexer's Rust references). The rule "located, never assumed on PATH" stands where a manager exists. The fallback is still a *located* absolute path, never a bare name. Rejected: `sh -c 'rustup which … \|\| command -v …'`, because there's no `sh` on Windows (planned). Rejected: dropping rustup, because rustup's pinned-toolchain answer is the right one when it's present. |

---

## 2026-10-15 — Claude Code mods: usage, model routing, drive, gating (#238)

Claude Code 2.1.300's mods API reaches what shell hooks cannot. Four policy calls
taken while building on it; the PRD carries the result
([`prd/…/03-capture-sessions`](prd/modules/01-foundation/03-capture-sessions/README.md) §Usage & cost capture).

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **List cost is not billed cost** | Claude Code's `/cost` total lands in a new `activity.sessions.list_cost_usd`, never in `metered_cost`. | On a subscription it is an API-equivalent estimate, not spend; `metered_cost` keeps a source's billed figure (`0.0` on a plan is a reading). Rejected: writing it into `metered_cost` as #176 first suggested. |
| **Subagent model by risk class, configured** | The mod's `agent.spawn` hook asks `POST /api/review/subagent-model` with the working tree's changed paths and the task; the daemon classifies (`auto`/`review`/`approve`) and answers the model the user's `routing.risk_models` config maps that class to. **No map, an unmapped class, or a caller-named model overrides nothing.** The main loop is untouched. | Policy stays in the daemon (governance owns it), off until a user opts in. Rejected: a built-in default map active for everyone; main-loop `turn.step` effort changes. |
| **Run drive is opt-in — supersedes AR-3's "drive stays OFF" for that case** | A plugin option, **off by default**. When on, the mod submits a run's dōjō nudges into the session as prompts (`$.prompt.submit`), only for a run this session started, only while idle, each nudge id once. With the option off, AR-3 holds unchanged: nudges are pull-only (`get_pending_nudges`). | `$.prompt.submit` is the one capability with no equivalent before mods; an opt-in keeps the steer-not-drive default for everyone who did not ask. Rejected: keep steer-only; notify-without-submit. |
| **Tool gating is opt-in and fail-open** | A plugin option, **off by default**. When on, `tool.check` asks `/hook/gate` and returns its decision; an unreachable or slow daemon **allows** (the gate's existing rule: infrastructure never blocks a tool call). It retires the never-registered shell `hooks/gate`. | `/hook/gate` raises a phone gate even without an allow-list and can hold a call ~50 s, so always-on would change every session. Rejected: always-on; fail-closed. |

---

## 2026-10-14 — T3 tracker & gating reconciliation

The T3 spec (Dōjō Project Planning) had drifted from its design source. Its seven
"gating targets" were a different set from the mockup's, one of them gated on FTR —
which reorg §9.3 lists as **improvement-only** — and its Done gate said the
adapters "export status back" while its own wrong-gate said read-only. Reconciled
to the mockup (`dojo2-project.jsx`) and reorg §9 as the single source. (Its read-only
row is superseded 2026-10-26 for metrics write-back: `spec/daemon/tracker-metrics-writeback.md`.)

| Decision | Locked answer | Why |
|---|---|---|
| **Targets gate on the code; insights never gate** | Every gated reading is code-derived. Behavioural readings — **FTR, corrections/session, AI-assisted share, gate wait time** — are insights only and can never block a merge or sprint close. The old "FTR ≥ baseline + δ" gate is **removed**. | reorg §9.1/§9.3 and the mockup both state it; an inherited codebase would otherwise fail every sprint on behaviour it did not cause. |
| **The seven targets are the mockup's seven** | 1 hard-rule violations `=0` · 2 coverage on guarded paths `≥75/80%` · 3 new layer violations `=0` · 4 new dependency cycles `=0` · 5 modules in the zone of pain `≤2/1` (warn) · 6 changed functions over complexity 10 `≤3/2` · 7 new high-severity anti-patterns `=0` (warn). Coupling/cohesion are **not** gates — they are absolute project-level ratios (improvement-only). | Matches `PJ_TARGETS` in the mockup and reorg §9.2's buildable map (1 ✅, 2 🟡 scoped reads, 4 ⛔ #156). |
| **Read-only means read-only** — *superseded 2026-10-26 (tracker-metrics-writeback) for usage metrics* | Tracker adapters **never write back**. There is no export-back path; the Done-gate line is removed and the gaps feature's export scenarios are replaced by one read-only scenario. | The mockup is unambiguous (*"synced read-only"*); the spec contradicted its own wrong-gate. |
| **Inheritance is most-specific-wins** | Dōjō → Project → Sprint → Task type → Task; a target with no value at a scope **inherits** the nearest ancestor's. Overrides narrow, never widen. | `PJ_CHAIN` in the mockup; reorg §9.1. |
| **Modes are `block` / `warn` / `off`** | Only `block` prevents a sprint close; `warn` surfaces; `off` disables the check. | `TargetEditor` in the mockup offers all three. |
| **A blocking miss needs a note; unlinked sessions don't count** | A sprint missing a blocking target cannot close without a note (maintainer+). A session counts toward a task only when its branch, commit or prompt carries the ticket key; the rest are unlinked and count toward nothing. | Mockup sprint-close rule + key-linkage row. |
| **One tracker owns sprints** | Several trackers may contribute projects/issues/task types; exactly one owns the sprint calendar. | Mockup *"Owned by Jira — 6 imported"*. |
| **Slack hooks** | Three: `miss` (blocking target missed mid-sprint), `close` (sprint closed, with its note), `gate` (a session waits on a gate > 30 m). | Mockup `ProjIntegrations` notifications block. |
| **The ratchet is own-baseline, for readings that can't be delta-scoped** | Gates are delta-scoped where possible (new violations, changed functions, guarded paths); the ratchet covers project-level readings by comparing against the project's **own trailing baseline**, and advances only when a reading improves **and holds**. Not FTR-based. | reorg §9.3–9.4; the ratchet was previously mis-attached to FTR. |

Full requirements: [`design/integrations-and-gating-requirements.md`](design/integrations-and-gating-requirements.md) ·
folded into [`prd/modules/04-team/03-dojo-workspace-planning/README.md`](prd/modules/04-team/03-dojo-workspace-planning/README.md).

---

## 2026-10-13 — Entry point, role vocabulary, and personas

Taken during the docs reorg (full text: [`reorg.md`](reorg.md) §16, decision 10).
Recorded here because they change the product surface, not just the docs —
the PRD states only the current result; the reasoning lives here.

| Decision | Locked answer | Why |
|---|---|---|
| **Entry point is minimal by design** | Getting started is one gate — *point sensei at your work* (add a scan root, press Begin). The full configuration surface (Preferences) is free-navigation, reachable at any time, never blocking, and covers folders, projects, assistants, libraries, instruments, inference, assignments and profile. No mandatory setup flow. | A lighter entry makes the first run simple; everything configurable stays reachable later to adjust how sensei behaves and which features and support are enabled. |
| **Dōjō role vocabulary** | Four roles: **developer · maintainer · org admin · lead**. **Lead** is the client/engagement confidentiality role — the lesson travels, the source never does. | Short, one-word role names; the engagement concept is unchanged. |
| **Personas** | Solo developer · team developer · maintainer · org admin · lead, plus **manager / delivery lead** (*the roll-up*) and **client / project stakeholder** (*the narrow lens*) — the multi-user, multi-team viewpoints. Authored in [`prd/03-personas.md`](prd/03-personas.md). | The multi-user, multi-team case needs the management and client/project viewpoints. Roles that are not yet built are covered by the roadmap/gap register, not by prose in the PRD. |

---

## 2026-10-13 — Indexer ignore: stored configuration, synced — not a dotfile

The indexer must be able to exclude paths — committed mockups, vendored or generated
code, fixtures — so they don't skew the code graph and the metrics derived from it.
The exclusion list is **stored configuration scoped to the repository**, not a
committed `.senseiignore`.

| Decision | Locked answer | Why |
|---|---|---|
| **Ignore lives in the database** | Exclusion rules are configuration rows scoped to the repository (the same store as scan roots and folder roles), editable from the Folders pane (M2). | One source of truth; the Folders pane already owns roots and roles; no dotfile clutter. |
| **Seeded from sources the repo already carries** | The list seeds from the repo's existing ignore inputs — `.gitignore`, and a tool config such as qlty's `exclude_patterns` in `.qlty/qlty.toml` — then the DB rules layer on top. | Don't make a developer re-declare what the repo already says. This repo's `.qlty/qlty.toml` already excludes `docs/mockups/**` (prototype JSX) and `crates/senseid/grammars/**` (vendored generated parsers) — the exact noise that would skew the graph. |
| **Shared via Dōjō sync** | A rule one user adds for a repository syncs to every other user of that repository through the Dōjō, so the whole team indexes the same files. | The first person to notice the mockup/vendored noise fixes it once for everyone. |
| **`.senseiignore` is not the primary mechanism** | A committed ignore file is rejected *as the mechanism*; it may be supported later as an **import seed** for a repo that already keeps one. | A file is a second source that can drift from the DB, and it can't be set for a repository you don't own. |
| **Accepted cost** | The first install may index a path once before the ignore is added; a fresh clone re-resolves the ignore from sync. | The team-wide fix outweighs one extra scan leg. |

Verified 2026-10-13: **no ignore mechanism exists** — no `.senseiignore` reader and no
ignore column or table; the indexer walks every file under a scan root. Tracked as gap
**G-2** ([`roadmap/06-gaps.md`](roadmap/06-gaps.md)).

---

## 2026-10-01 — Three decisions SUPERSEDED, with Jerry

All three were settled choices documented in the schema. Each was revisited
because the product moved and the old answer had started to cost more than it
saved. Recorded here rather than silently re-done, so the next reader finds the
override and not just the contradiction.

**The standing rule Jerry set while doing this: when an older decision blocks
progress, ASK — do not engineer around it, and do not blindly inherit it.**

| Decision | Superseded answer | New answer | Why it changed |
|---|---|---|---|
| **D-PROJ-JUNCTION** (was D1/D2/D10) | A project aggregates repositories **via the folders junction**; `repositories` has no `project_id`. Membership sat on `folders.project_id`, settable independently on all 13,697 folder rows. | Membership lives in **`sensei.repositories_in_projects (project_id, repository_id)`**. `folders.project_id` is derived, written only by inheritance from the repo anchor. | Nothing enforced that a repository's folders agree, and one had already drifted: the client-q `documentation` checkout's root sat in project `client-q` while its 362 subfolders sat in `documentation`. A junction keyed on the repository makes that unrepresentable. A `project_id` COLUMN on `repositories` was rejected — a repository is keyed on its REMOTE, so two checkouts collapse to one row, and four repositories legitimately serve two projects each (`kavach` in `kavach` + `vite-multi-adapter`, `bridge` in `bridge` + `sparsh`); a column would force each to pick one and drop the other silently. |
| **D-REPO-ANCHOR** (was I16) | `folders.repository_id` is set **ONLY on the repo-root/checkout folder**; subfolders resolve via nearest ancestor (`repo_anchor_for`). | **Every folder carries its own `repository_id`**, inherited from its anchor at WRITE time. `kind` (`git`/`standalone`/`subtree`) still marks which folder IS the anchor. | `repo_anchor_for` is a set-returning FUNCTION, and a predicate cannot be pushed into a function scan. Measured: `WHERE project = 'sparsh'` planned as a Seq Scan over all 13,715 folders with 13,715 function calls, to answer a question with 45 rows. A migration of eight views onto it was built and adversarially reviewed; **four failed outright** — `structure_edges` 81 ms → ~37 s (~450×), `task_health` ~70× on its own documented query, `project_metrics` made non-auto-updatable which silently broke a production WRITE path. After: 21 ms and 149 ms, via `Bitmap Index Scan on folders_repository_id_idx` — an index that already existed and was unusable only because the column was deliberately under-populated. A materialised view was designed twice and rejected both times: it needs a refresh hook, carries staleness a reader must be told about, and the first version (materialising folder→**project**) would have made a user editing a project see no effect until the next scan. |
| **D-SCAN-SENTINEL** | A source file's `files` row is written at the stage-3 barrier sentinel (`mtime=0`, empty hash) on every structure pass, so it always "reads as changed" and is certain to be parsed. | The structure pass writes the **real fingerprint**; `parsed_at IS NULL` alone decides what is fanned out. | The sentinel was written over EVERY row on EVERY pass, so an already-parsed file went `content_hash` real → `''`, the upsert saw `IS DISTINCT FROM`, and `parsed_at` was reset. Since that pass reruns on every reconcile tick, watcher batch and FSEvents overflow — and a `cargo build` overflows FSEvents reliably — **an actively worked repository could never finish indexing**. Measured: sensei reached 1,746 of 1,757 files and returned to 0; 55,501 files machine-wide sat at the sentinel. The sentinel was never needed for its stated purpose: `list_unparsed_files` selects on `parsed_at IS NULL`, never on the hash. |

### What these have in common

Each old decision was documented with a reason, and in each case **the reason was
the most persuasive part of the wrong answer**. The I16 comment read as a
constraint to design around; it was a choice that had stopped paying. The
sentinel's comment cited a real past defect (`8d488e2a`) that the current gate no
longer admits. The folders-junction comment stated a genuine M:N requirement that
a different shape satisfies better.

So the test for a settled decision is not "is there a reason written down" but
"does that reason still hold against what is measured today".

---

## 2026-07-24 — Auto-buildout decisions locked (P0–P5)

Locked with Dev ahead of the phased unattended build-out.

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **Relay-first** | Build **relay supervision first** (P1): publish the plan as a relay run (phases→segments), daemon federates status → Dōjō, Dev watches as `devuser` + nudges; **no update in 5 min = stall**. Relay **STATUS** (safe), distinct from relay **DRIVE** (off). | Makes the whole unattended run watchable/nudgeable — solves "am I stuck?" without pings. Rejected: task-list-only visibility (too coarse for a long run). |
| **D-INJECT** | Governance rules are **pushed**, not pulled: resolve the repo's rules → inject into `additionalContext` at **SessionStart**, re-inject the **mandatory tier at PreCompact**, and inject into the **relay driver** for daemon runs. Mandatory/required always; advisory on-demand. | Root cause of "not sticky" = pull-only (`get_rules` reminder). Rejected: nudge-only (status quo, doesn't stick); SessionStart-only (drops after compaction). |
| **D-RULEPACK** | A rule pack is a **new `dojo.rule_packs` table + pack↔rule join** (area/scope/source/enforcement first-class). | Clean provenance + adopt-at-scope + maps to the designer's original LIB_PACKS model. Rejected: tag/collection on `shared_rules` (weaker metadata). |
| **D-COORD** | Coordinator **v1 = task-typed router at `driver_for` + gateway single-shot leg for cheap types + attribution ledger, drive OFF.** | Smallest useful superpowers-mirror. Rejected (v1): local ACP coding-agent + Planner→Builder→Judge loop (bigger, riskier → follow-up). |
| **D-GATE** | Blocking PreToolUse hook-gate stays **OFF** (activation = Phase-5 human gate). | Enforcement on live sessions is irreversible-ish; Dev-gated. |
| **D-DRIVE** | `SENSEI_RUN_DRIVE` stays **OFF** the whole unattended run. | Creating/publishing runs is safe; driving a live agent is a human gate. |
| **D-SEED** | Invoke `dojo.seed_default_governance()` on the **Worker/Supabase** side + auto-register the global-dōjō pull `knowledge_source` on install. Prod-apply **gated**. | The seed has zero callers today ("reaches nobody"). |
| **D-ORIGIN** | Canonical provenance vocab = `authored | promoted | federated`; retire `remote`. | Doc/code mismatch (`remote` vs `federated`). |
| **D-STANCE-SCOPE** | Personal **stance** store is **user-scoped, not tenant-scoped**. | Personal governance follows the user with or without a dōjō. |
| **D-TIER3-DDL** | Tier-3 tables full-DDL-no-ALTER under `database/ddl/table/dojo/*.ddl`, `dbd deploy` against **e2e/local**; shapes proposed + confirmed as authored. Prod apply **gated**. | dbd is the schema tool; never Cloudflare-D1 (that's Decision-1). |
| **D-BILLING** | Billing = **schema + route only**; payment provider out of the unattended run until chosen. | No provider decided. |
| **D-CUTOVER** | `/console`→dojo2 redirect, `develop`→`main` merge, prod deploy, prod seed — **all human-gated (Phase 5)**. | Irreversible / trust-boundary. |
| **D-DECISIONS-HOME** | This file is canonical; per-feature → `features/<name>/decisions.md`. | The empty canonical slot; ADRs were misfiled. |

**Unattended boundary (standing):** safe-unattended = deterministic, local, reversible,
verifiable per chunk (DDL edits, Rust/TS on `develop`, tests, local browser-verify, dbd on
e2e/local, commits/pushes to `develop`). Human-gated = cloud/prod DB writes, Worker prod
deploy, secrets/VAPID/service-role, merge→main, cutover, gate activation, flipping
`SENSEI_RUN_DRIVE`, any payment-provider choice. The loop prepares + stages gated work,
writes the evidence, and waits.

---

## 2026-07-25 — Coverage-audit scope calls

Resolved during the mockup/journey/doc/code coverage audit (current build status
lives in [`roadmap/README.md`](roadmap/README.md)). These set what gets built next.

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **D-PLANNER (v0 SHIPPED)** | **Build a real planner** — takes a goal / spec / issue and generates a structured plan (phases → features → acceptance criteria) that the plan-as-run engine, the project-window **Tasks** tab, and the `sensei-plan-depth-reviewer` gate consume. **v0 shipped** (`crates/senseid/src/planner.rs`): gateway `reasoning`-chain decomposition (system prompt encodes the depth bar; 1 retry on unparseable JSON), pure `parse_plan` (rejects a feature with no observable criterion) + `render_plan_md` (canonical `docs/plan` shape) — 6 unit tests; daemon `POST /api/planner/generate {goal,context?}` → `{plan, markdown}` (502 on failure, no silent default); MCP tool `plan`. **Stateless** — no plan tables (the run engine is file-based via `plan_ref`; the Tasks tab isn't built → YAGNI). Live-verified: a real goal decomposed into observable-criteria features honoring the context. **Deferred:** DB persistence + Tasks-tab wiring (when the project window lands); automated depth self-review loop (v1). | The one missing spine: "project needs the planner" + plan-as-run wants phases→features→gates, but plans are hand-authored `docs/plan/*.md` today. Biggest single unlock. Rejected: keep plans human-authored only (no autonomous decomposition). |
| **D-SCREENS** | Build the **Consolidation** screen (backend already shipped — just the app route), the **Agent/Persona editors** (autonomy ceiling + tool envelope + **replay-test against past sessions**), and **Solution-track** (desktop multi-project engagement rollup — a distinct solo-consultant surface, kept alongside the dōjō Clients zone). **Benchmark runner CUT** for now (2026-10-19: now deferred epic #279, to be re-designed on transcript history). | Designed-but-unbuilt with real user value. Benchmark (A/B sensei-vs-no-sensei) is a research/marketing artifact, not a near-term product surface — deferred. |
| **D-CHECKER (v0 SHIPPED)** | **Build checker execution** — a rule with a `checker_ref` runs its checker and yields a **pass/fail verdict**. **v0 shipped** (`crates/senseid/src/checker.rs` + `POST /api/checkers/run` + MCP `run_checkers`): resolves a repo's adopted `verification='checker'` rules, maps each `checker_ref` (a canonical verb: lint/test/build) to the repo's manifest-discovered command (`sensei.project_commands`), runs it (tokio process, 300 s timeout, output tail) and records `pass|fail|skipped|error` in new `sensei.rule_check_runs` (daemon-local; `check_verdict` enum). **Trust boundary:** v0 runs ONLY manifest-declared commands — never an arbitrary string from a rule — so an adopted pack can't smuggle a command; regex/skill `checker_ref` kinds, per-stack `applies_to` gating, and CI/hook triggers are deferred. The 3 lint-checkable stack-template rules (clippy/strict-types/ruff) flipped to `verification='checker', checker_ref='lint'`. Verified: 4 unit + 1 DB test; live on the sensei repo — 3 checker rules → `cargo clippy` → **pass**, persisted. | `rule_pack_rules.checker_ref`/`verification` exist but nothing ran them. Rejected: advisory-only. |
| **D-SEED (SHIPPED)** | **Default governance bundle shipped as BUNDLED LOCAL PACKS** (via D-LOCAL-PACKS, not `dojo.shared_rules`). Retired the zero-caller `dojo.seed_default_governance`; replaced by `sensei.seed_default_constitution()` — four global-library packs by `rule_pack_area`: `default-principles` (4, mandatory), `default-architecture` (5), `default-process` (12), all **auto-adopted at the general `global-dojo` namespace** so a fresh install resolves them **offline** through both `get_rules` (per-repo) AND the always-on global `~/.sensei/rules.md` (`materialize_global_rules` folds `resolve_local_pack_raws(None)` — general/user adoptions, no folder; kept OUT of `resolve_global_rules` so the LLM consolidation still operates on learned memories only); `stack-templates` (9, Rust/TS/Svelte/Python) **seeded but not adopted** (opt-in per stack). **SEEDED BY dbd IMPORT, not by a procedure call** (corrected 2026-10-02): the seed functions `sensei.seed_default_constitution()` / `sensei.seed_ponytail_pack()` were replaced by the staging import procedures (`import_rule_packs`, `import_rule_pack_rules`, `import_rule_pack_adoptions`), which dbd auto-runs from `import/staging/*.jsonl` on every `apply`/`deploy`. There is NO `seed_bundled_packs` — the earlier claim that one was wired into `bootstrap::database::deploy` described a flow that no longer exists. Verified on a from-scratch database: 14 packs and 76 rules land from the import alone. The two retired functions survived only as orphan bodies in already-deployed databases and are dropped (#217). The adoption datafile carries THREE rows (`default-principles`, `default-architecture`, `default-process`) and deliberately no `ponytail`: **ponytail is opt-in** (settled 2026-10-05, #217 — see D-PACK-KIND). The live database had it adopted by `bundled` since 2026-07-26, which was the last trace of the retired `seed_ponytail_pack()`; that row was dropped so both planes now resolve the same three packs. Restore it with: `INSERT INTO sensei.rule_pack_adoptions(pack_id, namespace_id, pinned_version, adopted_by) SELECT p.id, n.id, '1', 'bundled' FROM sensei.rule_packs p, sensei.namespaces n WHERE p.slug='ponytail' AND n.slug='global-dojo';` Content = `spec/governance/default-constitution.md`, verbatim. Verified: DB-backed test (adopts 3 packs, 21 rules resolve, stack excluded, idempotent) + live `get_rules` offline. Prod Worker apply stays gated (D-CUTOVER). | A new project starting empty undercuts "grounding that sticks". Packs (not shared_rules) keep ONE source, deploy to both planes, and resolve with zero dōjō. |
| **D-PACK-KIND** | The **ponytail** coding discipline (and coding-convention libraries generally) = a **rule pack** (`dojo.rule_packs`, `area=principles`), **not** a new "skill pack" type. Rules are conventions — injected (D-INJECT), resolved on the ladder, verifiable (D-CHECKER). A "skill pack" (bundle of *callable* skills) is the marketplace `extensions`; a rule references a skill via `rule_pack_rules.skill_ref`. Shipped as a global-library pack (6 rules), resolution + never-weaken override proven. Seeded by the dbd import (`import/staging/rule_packs.jsonl`), NOT by the retired `seed_ponytail_pack()` procedure — see D-SEED. **Seeded but NOT adopted — on every install, settled 2026-10-05 (#217).** It is a discipline a developer chooses, like `stack-templates`, not part of the default constitution: the three `default-*` packs state what sensei asserts about process and architecture, where ponytail's six rules are a style of writing code. The live database's adoption (a leftover from the retired `seed_ponytail_pack()`) was dropped so the two planes agree. **Nothing surfaces the pack for adoption yet** — until something does, opt-in means "unreachable" in practice, which is the follow-on this decision creates rather than resolves. Spec: [`spec/dojo/governance/ponytail-pack.md`](spec/dojo/governance/ponytail-pack.md). | Don't overload rule_packs into skill_packs — the two compose via `skill_ref`. Ponytail's *satellite* skills (`-audit`/`-review`) would be marketplace extensions, separate from the discipline-as-rules. |
| **D-LOCAL-PACKS** | **Rule packs live in BOTH planes, in tandem.** The daemon keeps a **local replica** of the pack tables (`dojo.rule_pack*` deployed to the local `sensei` DB via the `default` scope AND to Supabase via the `dojo` scope — one definition, two planes; the **same seed procedures seed both**, so no content copy). Curated packs (default constitution, ponytail, user-authored) resolve **offline alongside memories** — never poured INTO `sensei.memories` (memories = *learned* knowledge; packs = *curated*). Works with **zero dōjō** ("just another shared mechanism"); **when a user runs a dōjō the two sync**. Resolution = memories + local adopted packs + the live dōjō fold-in, in tandem. | Keeps curated rules out of the learned-memory store; offline-first; no vendor lock-in. **dbd resolution (Dev):** NOT a dbd fix — `includes` is authoritative-by-design (`scope.rs::resolve`: non-empty includes = base, then excludes subtract → `includes:[dojo.rule_packs]`+`excludes:[dojo]` = ∅; making includes additive would break `includes:[app]`="only app"). Idiom = **put shared objects in a shared schema.** So **move the pack objects `dojo.* → sensei.*`** (enums `rule_pack_area`/`rule_check`, tables `rule_packs`/`rule_pack_rules`/`rule_pack_adoptions`, proc `seed_ponytail_pack`): `default` scope gets them free (local), `dojo` scope includes them explicitly (Supabase). REFACTOR (atomic): move DDL + `design.yaml` dojo-scope includes + daemon `resolve_rules_raw` reads LOCAL `sensei.rule_packs` (the DojoClient fold-in becomes the sync) + Worker `rules-data.ts` `sensei.rule_packs` + re-reconcile both DBs + re-seed ponytail + drop orphan `dojo.rule_pack*` on supabase. Verify get_rules offline. |
| **D-EXEC-TEAM** | **Design now, build later:** a sensei-owned **agentic execution team** — planner → coordinator → model-chain workers (coder · reviewers · tools · persona-tester) — that plans + executes + quality-gates feature work **as monitored relay runs**, so it's model-agnostic and controllable (not Claude-only). Full design: [`design/agentic-execution-team.md`](design/agentic-execution-team.md). Resolved sub-calls: spec-fidelity = rendered-DOM check + **React-mock reuse-not-rewrite**; **worktree per feature**; the **quality gate is authored into the spec by the skills**; **build the inference-usage ledger first**; the **planner self-reviews** for completeness/coverage; runs **drive-OFF**. Build v0(ledger)→v1(skinny slice)→v2(team+routing)→v3(parallel+consensus). | Controllers own their loop → can't gate/route/enforce mocks (torii+seiki: mocks ignored, tests missed it). Owning the orchestration = control + no vendor lock-in. Deferred build (Dev): want a robust base first. |

**Still open (mock hygiene / scope):** prune the superseded-but-not-in-`discarded/` mock
variants (sharing-review, mcp-replay-insights, the multi-option learnings/project-pages/
libraries/nav files); decide whether dojo2 ports the richer **extension governance**
(personas/hooks/plugins + org→team→project scoping + adoption tracking) from
`dojo-extensions.jsx`.

---

## 2026-07-26 — Automated runs, phase 1 (Claude-CLI-driven)

The phase-1 bootstrap of D-EXEC-TEAM: Claude Code owns planning + execution, the daemon is the
durable spine. Full design: [`design/automated-run.md`](design/automated-run.md).

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **AR-1 (packaging)** | Autonomous-run capability lives in **skills** (`planner`/`analyzer`/`executor`); the `/sensei:plan` + `/sensei:analyze` commands invoke them and **preserve their existing human-pipeline behavior** (no forked commands). `init` **dropped**. | Injection is already automatic (D-INJECT); a 4th initializer duplicates `sensei init` + `sensei scaffold`. `analyze` skill name is taken (health-check) → new skill is `analyzer`. |
| **AR-2 (plan store)** | Register the plan by **authoring `dojo.relay_segments`** (+3 cols `agent`/`model`/`spec_ref`) via a new `register_plan` MCP verb, reusing `publish_run`. Plan files live self-contained under `docs/plan/<plan-id>/` (`plan.md` + `tasks/<task-id>.md`); `spec_ref` points into `tasks/`. | Reuses shipped federation; whole plan visible in Dōjō (with models) before execution. Rejected: new `sensei.plans` tables (reverses D-PLANNER's stateless YAGNI, no consumer yet). |
| **AR-3 (exec MCP)** | **Minimal 4-verb contract:** `register_plan`, `update_task_status`, `report_run_outcome`, `get_pending_nudges` (agent-pull). Keep-alive feeds the **progress clock via `update_phase`, never `heartbeat_at`**. Drive stays **OFF**; do NOT add `update_run_phase` (the phase bridge already exists). | Smallest surface that makes the loop observable + steerable. Full claim/heartbeat/task-DDL contract is the deferred D-EXEC-TEAM coordinator. |
| **AR-4 (models)** | Phase-1 execution dispatches **Claude subagents only**; `model` (incl. local qwen/qwythos/gemma4) is **recorded** per task for Dōjō + phase-2 routing, local models used only for bounded gateway sub-steps (`infer`/`consensus`). | CC subagents are Claude-only. Recorded assignment seeds the phase-2 tier-router + the inference-usage ledger (#49). Rejected: wiring local models as autonomous workers now (that IS the deferred daemon executor). |
