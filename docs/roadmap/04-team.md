# Roadmap — Team plane (T1–T6)

The Dōjō: core (T1), consoles (T2), workspace planning (T3), billing/seats (T4),
releases and DORA (T5), the workspace viewer (T6).
Status legend: [`README.md`](README.md).

> **Status populated 2026-11-03.** The cells follow the readiness table in
> [`../review/2026-11-03-orientation.md`](../review/2026-11-03-orientation.md) §2 and the capability
> audit in [`../review/2026-10-27-roadmap-vs-state.md`](../review/2026-10-27-roadmap-vs-state.md) §1.4.
> The last column names what proves each cell; a status is a snapshot, so move it when the module moves.

| ID | Module | One-line "done" | Status | Evidenced by |
|---|---|---|---|---|
| T1 | Dōjō Core | Knowledge travels up, is governed, and comes back down | 🟡 **Partial** | Membership, tenancy, the federation wire and resolved rules pulled by `dojo/client.rs` (`/v1/t/*/rules/resolved`, `tenancy_access.sql`) are built; the path upward has never sent a row — memberships stay `authenticating` with `last_seq 0` and the outbox row has been `held` since 2026-08-29 (#184) — production is unverified (#314), and the sync work packages are at WP1–3 with WP4 staged (readiness table: ~40%) |
| T2 | Dōjō Consoles | All four roles (contributor · maintainer · lead · admin) have a live console | 🟡 **Partial** | Role-gated groups in `dojo/src/lib/nav.ts`, the four-role model and the lead's team metrics (R8) are built; the org `Scopes` section loads nothing (`[section]/+page.ts:90`), leads have no menu of their own, and there is no viewer shell (D68) |
| T3 | Dōjō Workspace Planning | A workspace plans against its tracker and gates on code health | 🟡 **Partial** | The daemon's Jira adapter (`crates/senseid/src/tracker/`), boards per level (D40), sprints and the Tickets screen are built; GitHub Issues and Azure Boards are not — D49 puts them on the dōjō-owned path, behind the secret store (#310) — there is no target store (metrics as gates are ~0%, D47/D48), and 0 trackers are live (readiness table: ~35%) |
| T4 | Billing & Seats | A private shared Dōjō can be subscribed and metered | 🟡 **Partial** | `billing_accounts.ddl`, `seats.ddl`, `/v1/t/*/billing` and the `ScrBilling` screen are built; there is no payment provider and no subscription table, nothing writes `dojo.seats`, and the price is to be announced until a provider exists (D75) — readiness ~15% |
| T5 | Releases, DORA & Release Gates | Deployments are read from CI, the five DORA keys are computed, and a release that would ship a regression is held | ⛔ **Gap** | Nothing in DDL or code — [09-dojo-expansion](09-dojo-expansion.md) Wave D; the CI/DORA puller (#316) is the open prerequisite |
| T6 | Workspace Viewer | A granted viewer reads one workspace's outcomes, read-only | 🟡 **Partial** | `workspace_viewers`, `dojo_viewers`, `viewer_audit.sql` and per-workspace and per-team grants are built (#307 A3); there is no `/v/{tenant}` shell, no expiry, export flag or relationship, and the minimal viewer shell (metrics first) is this week's D68 item |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| T-1 | **T3's tracker adapters: the daemon-side Jira adapter is built (PR #274); Azure Boards and GitHub Issues are not**; 1 of 7 gating targets buildable today, 2 need scoped reads, 4 blocked on #156 | P1 |
| ~~T-2~~ | ~~join/membership flow dead~~ — **resolved (#307 A3, 2026-10-21):** an admin invites a member or a viewer (7e547a19); accepting makes the membership or the dōjō viewer | — |
| F-3 | Dōjō sync: **works against the local dōjō** (2026-10-20/21: repositories, metrics, graph and relay; #301/#299 verified there); **production unverified** until a daemon is enrolled with dojo.sensei-hq.com. #184 (`sync_status`) still open | P1 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

T2's consoles were gated on T-2 (membership flow), resolved in #307 A3. T3's spec (reorg §9) is
written — the gate/improvement split and own-baseline ratchet (reorg
§9.3–9.4) are its fairness model. T4 rides the business model in `../prd/01-vision.md`:
every dōjō starts on the free tier, and billing pays for what is beyond it (D20).
