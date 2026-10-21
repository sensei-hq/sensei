# Dōjō Core · T1

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
org admin ([persona 4](../../../03-personas.md#4--org-admin--the-plane-owner)),
engagement / client lead ([persona 5](../../../03-personas.md#5--engagement--client-lead--the-confidentiality-guard))

## Purpose

Dōjō Core is the **federation backbone**: it routes a contributor's local sensei
knowledge to the right Dōjō, applies the org's governance, and returns the
resolved constitution — all without leaking client work. The Dōjō is **optional**
(there is always a personal ladder); a solo user gets a free personal Dōjō with
no plan. The job is **one wire protocol, one membership model, one resolution
engine** — the developer installs sensei, signs in with GitHub or a magic link,
and the Dōjō resolves their effective rules for every repo they touch. The user
feels this as *I sign in once, and the right rules just work — wherever I code.*

> **Status honesty.** The membership model, routing, tenant resolution, the
> contribution/accumulation/triage/approval/distribute pipeline, the Worker `/v1`
> API (rules, artifacts, triage, audit, knowledge), anonymization (origin-based
> strip + universal strip), and the dojo_protocol wire are built and tested.
> The **role-scoped surfaces** (contributor/maintainer/lead/admin) are built: the
> org context `/org/[slug]/[section]` has its own nav, gated by role
> (`dojo/src/lib/nav.ts` `navForOrg`); gap T-2 is resolved (#307 A3: invites). The
> hand-made "create a Dōjō + starter constitution" flow is retired (D11/D20; R1
> 7478fe8d) — a dōjō comes from a git integration (#308). The free personal ladder
> exists for solo users.

## In scope / out of scope

- In: membership kinds (employer · client · community · personal) + role
  ladder (`contributor < maintainer < lead < admin` — `dojo.member_role`, canonical since 2026-10-21); the `dojo`
  schema (`repositories`, `repository_elections`, `repository_metrics`,
  `memberships`, `shared_rules`, `tenants`; `tenant_scopes` is planned, not built);
  routing (`dojo/routing.rs` — client binding routes to client Dōjō and excludes
  employer; employer binding routes to employer; personal is the fallback);
  the federation wire (`dojo_protocol` — rules + artifacts push/pull);
  contribution pipeline (`dojo/contribute.rs` — publish, already-sent guard,
  artifact kinds: `memory`, `rule`, `skill`, `command`, `agent`, `pattern`);
  triage/approval with scoring (auto-approve 0.8, per-contributor weight,
  FTR-delta credit, k-anonymity ≥3); anonymization (origin-based strip +
  universal strip); audit ledger + compliance export; governance resolution
  (`governance.rs` — gather → order by enforcement desc + scope level desc →
  mandatory-lock → dedup, cached 5 min); the `get_rules` / `get_layered_context`
  MCP tools.
- Out: the role consoles (T2), workspace planning against trackers (T3), billing
  and metering (T4), the website claims (P2).

## Surfaces

- **API (Worker `/v1`)** — `/v1/t/{tenant}/rules` · `/artifacts` · `/triage`
  · `/triage/promote` · `/audit` · `/compliance/export` · `/engagements`
  · `/projects` · `/knowledge` · `/rules/resolved` · `/policies` · `/members`
  · `/identities` · `/relay/{gates,inbox}` · `/health`.
- **Daemon** — `dojo/client.rs` (tenant base URL, artifacts/relay URLs,
  membership resolution); `dojo/routing.rs` (binding inference); `dojo/mod.rs`
  (federation client); `dojo/contribute.rs` (publish/accumulate);
  `dojo/relay_*.rs` (constitution, workspace, run, nudge); `governance.rs`
  (resolution); `attribution.rs` (origin-based strip).
- **App** — `dojo/src/routes/(app)/you/*` (personal landing); `dojo/src/routes/(app)/org/[slug]/*`
  (org context, role-gated sections via `[section]`); `dojo/src/routes/v1/*` (Worker API); sign-in
  (GitHub, magic-link).
- **Data** — `dojo.repositories` (visibility: public/private/not-captured),
  `repository_elections`, `repository_metrics`, `memberships`, `shared_rules`,
  `tenants`, `audit_log`, `audit_events`, `engagements` (`tenant_scopes` is
  planned, not built).

## Data invariants

- **Lead.** A repository belongs to **exactly one Dōjō** via its membership
  binding (`dojo.repository_elections`); client work routes to the client Dōjō
  and **excludes the employer** — the owning org wins ordinary conflicts.
- **Lead.** An election and a mandate are **different rows** — a repo going
  public does not silently convert one into the other.
- **Lead.** `principal_id` in metrics is a **principal, never a git email** —
  commit trailers are unverified assertions.
- **Lead.** `NULLS NOT DISTINCT` on election/metric keys makes re-push
  genuinely idempotent (two identical inserts collide).
- **Lead.** Anonymization: `origin` (open-source · personal-closed · employer
  · client) determines the strip; the **universal strip** means client work is
  anonymized the same way every time, or it doesn't leave.
- **Lead.** Governance resolution order: ① confidentiality first → ② mandatory
  locks (★ / mandatory) so no narrower scope can relax → ③ otherwise more
  specific scope refines broader. The single canonical statement across
  confidentiality · enforcement · priority · scope — and how confidentiality
  interacts with a P0 — is [design/governance.md §Resolution
  order](../../../../design/governance.md#resolution-order-the-one-order); this
  module links it rather than restating it.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Membership kind | employer / client / community / personal | `employer` |
| Role floor | `contributor < maintainer < lead < admin` | `maintainer` |
| Tenant | the Dōjō this repo federates to | `dojo.sensei-hq.com/t/org/sensei-hq` |
| Effective rules | resolved ladder for this repo | 12 rules (3 company, 2 workspace, 7 personal) |
| Contribution | a shareable finding | `memory#m-42 → rule (mandatory)` |
| Triage | auto-approve / human / reject | `auto-approve score 0.87` |
| Audit | immutable event | `promote rule#r-12 by u-3` |

## Done gate

- A contributor signs in (GitHub or magic link), their repo binds to the right
  Dōjō via `routing.infer_binding`, and `get_rules` returns the resolved
  constitution for that repo.
- A contribution published from the daemon reaches the Worker `/v1/triage`,
  auto-approves at score ≥0.8 with k-anonymity ≥3, and the promoted rule
  appears in the next `get_rules` call (5 min cache).
- Client work is anonymized by the universal strip; the employer never receives
  the client's finding.
- The audit ledger records every promote/triage/decide event immutably.

```bash
curl -s "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/rules/resolved?repo=..." | jq '.rules|length'
```

## Wrong gate

- **A repo binds to the wrong Dōjō** (employer receives client work, or personal
  when a Dōjō exists) — routing regressed.
- **A promoted rule doesn't appear in `get_rules`** after cache expiry —
  federation/push regressed.
- **A Dōjō-authored rule never reaches the member's `get_rules` at all** — the
  pull/sync path is dead, not merely stale (`F-3`). The "shared" half of
  governance silently no-ops while every local path still looks healthy.
- **An election row silently becomes a mandate** — the two-row invariant
  regressed.
- **Two identical metric pushes create duplicate rows** — `NULLS NOT DISTINCT`
  regressed.
- **Anonymization leaks a client identifier** — universal strip regressed.
- **A mandatory rule is relaxed by a workspace scope** — mandatory-lock regressed.

## Scenarios

- [`scenarios/dojo-core.feature`](scenarios/dojo-core.feature) — membership,
  routing, federation, contribution pipeline, anonymization, governance
  resolution (pinned).
- [`scenarios/dojo-core-gaps.feature`](scenarios/dojo-core-gaps.feature) — the
  org context `/org/[slug]`, a dōjō imported from a git integration, role-scoped
  surfaces, and the Dōjō → member rule sync (F-3).

## Open gaps

- [T-2](../../../../roadmap/06-gaps.md) — join/membership flow. **Resolved**:
  invites (#307 A3) and GitHub-org provisioning create memberships, and the org
  sections are built and role-gated.
- [F-3](../../../../roadmap/06-gaps.md) — Dōjō sync (reorg §12). Works against the
  local dōjō; **production is unverified**. Member-facing consequence if it
  breaks: a Dōjō-authored rule never reaches the member's `get_rules`, so the
  shared half of governance silently no-ops (the wrong gate above; scenario in
  `dojo-core-gaps.feature`).
- **A dōjō from a git integration** (D20) — not built; tracked by #308. The
  hand-made create flow is retired (R1).

## Related

- [features/dojo/](../../../../features/dojo/) — the Dōjō feature dossier
  (legacy depth, folds here).
- [spec/dojo/](../../../../spec/dojo/) — the Dōjō pipeline specs
  (`dojo-auth-provisioning.md`, `dojo-lifecycle.md`, `governance.md`,
  `dojo-deployment.md`).
- [design/dojo-web.md](../../../../design/dojo-web.md) — the Dōjō web app
  sitemap, routes & IA.
- [design/governance.md](../../../../design/governance.md) — governance module
  design.
- [database/dojo.md](../../../../database/dojo.md) — DDL design notes.
- [journeys/dojo.md](../../../../journeys/dojo.md) — the Dōjō journey map.
- [prd/02-objectives.md §Dōjō](../../../02-objectives.md) · [prd/03-personas.md §2,3,4,5](../../../03-personas.md).
- Crates: `crates/senseid/src/dojo/`; Worker: `dojo/src/routes/v1/`.