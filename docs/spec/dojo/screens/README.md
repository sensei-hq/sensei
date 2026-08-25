# Dōjō screens — implementation roadmap

One spec per screen in this folder (21 screens), each keyed to its mockup component
(`docs/mockups/Sensei/lib/dojo2/dojo2-app.jsx`), tracing every bound field to a real source and
marking status. Canonical model: `docs/architecture/entity-access-model.md`. Conflict/impact list:
`docs/plan/2026-07-27-data-model-fix-impact-register.md`.

## Spec convention — three-layer state pattern (added 2026-07-28)

Every screen is built as three layers per the **`sensei:ui-state-pattern`** skill (invoke it before
implementing): **Component ← State ← Load**. Each spec names all three + the domain types.

- **Component** (pure, semantic name — no `K2*`/`dojo2*`): renders from state, routes user intent
  through state methods. Owns its styles → **mockup-fidelity verified per component** with a mock
  prop; adapts `md`/`md+` internally. A shell composes components.
- **State** (`<screen>-state.svelte.ts`, e.g. `relayInboxState`): the single source of truth —
  domain array + `$derived` views + named transition methods (`load`, `select`, realtime `patch`).
  Never mutated directly by the component. Testable with no DOM.
- **Load** (`<screen>.ts`, e.g. `loadRelayInbox`): the **mock/real seam**. Returns UI-typed data →
  `state.load(...)`. Starts as **hand-crafted mock** (exercises needs/running/finished/stalled/
  empty/error — matches the mockup) so the STUB screens (no `/v1` yet) build to fidelity NOW; later
  swap the body to a real user-wide/tenant fetch + transform — component & state untouched.
- **Domain types** describe what the UI renders, NOT the wire shape (the load layer maps): e.g.
  `RelaySession { id, title, goal, status, progress, phase, plan: SegmentGraph }`, `SegmentGraph`.
- **Realtime = a State concern** (`subscribe()` → `patch`), never the component's → UI is a pure
  function of state.
- **Copy = a 4th layer** (`@inlang/paraglide-js`, the SvelteKit-recommended i18n): NO inline
  literals — components reference `m.<key>()` from `$lib/paraglide/messages` (compile-time,
  tree-shakeable, i18n-ready). The **sensei voice** lives in the `messages/en.json` catalog.
  English-only now; locales slot in later with no component change. **Kanji glyphs are brand marks
  via `KanjiToken`, not messages.** See [[reference_dojo_i18n_paraglide]].
  *One-time prerequisite:* `npx @inlang/paraglide-js init` + the vite plugin, before screens use `m`.

This is why "STUB" is cheap here: mock the load, build the component to the mockup + tests, wire
`/v1` later. Worked exemplar: `inbox.md`. The other 20 specs get this three-layer section next.

## The one big finding

**Every screen's chrome is already built — 0 MISSING, 0 fully-DONE.** The work is not "build
screens," it's **data wiring**: PARTIAL screens read some real `/v1` data + some fixtures/hardcodes;
STUB screens render faithfully but their loader returns empty/fixture. So the roadmap is mostly
*loaders + `/v1` read endpoints + a few new tables + canon-conflict fixes*, not net-new UI.

## Status (21 screens)

| Screen | Zone | Axis | Status | Core gap |
|---|---|---|---|---|
| inbox | personal | user | PARTIAL | single-tenant (should be user-wide); no pips/repo/realtime; old gate card not mockup AskCard |
| projects | personal | user | STUB | loader returns `[]`; no projects endpoint/source |
| project-detail | personal | user | STUB | project/ladder/conflicts all fixtures; no resolve endpoint |
| my-dojos | personal | user | PARTIAL | real memberships, but kind/counts hardcoded |
| contributions | personal | user | STUB | loader empty; no contributions endpoint |
| constitution | governance | tenant | PARTIAL | W1 mapper built but wired to nothing; reads fixtures |
| rule-packs | governance | tenant | PARTIAL | fixture-backed; no read/adopt endpoint |
| scopes | governance | tenant | STUB | **no ownership backing table at all** |
| org-ladder | governance | tenant | PARTIAL | fixture; no authoring endpoint |
| org-home | org | tenant | PARTIAL | makes NO `/v1` call; members/needs hardcoded 0 |
| role-surfaces | org | tenant | PARTIAL | real members/audit, but names=shortId, policies tab hardcoded |
| identity | org | tenant | PARTIAL | real identity counts, but no IdP/SCIM table (synthesized) |
| billing | org | tenant | PARTIAL | real seat count; tiers/invoices fixture (no provider) |
| triage | org-ops | tenant | PARTIAL | real list; detail/conflicts/impact are proxies; decide unwired |
| approvals | org-ops | tenant | PARTIAL | derived from triage; no 2-sig quorum model; actions dead |
| knowledge | org-ops | tenant | STUB | 100% fixture; no endpoint |
| health | org-ops | tenant | PARTIAL | real signal cards; chart+alerts empty |
| engagements | client/lead | tenant | PARTIAL | real list+CRUD; `client` needs tenant split; counts hardcoded |
| incidents | client/lead | tenant | PARTIAL | real list+CRUD; client=uuid (no name join); no detail |
| client-audit | client/lead | tenant | PARTIAL | bound to WRONG ledger (action-audit vs artifact-strip) |
| signin | entry | auth | PARTIAL | OAuth+magic-link live; left panel diverges; self-host stub |

## Cross-cutting workstreams (the real shape of the work)

- **WS-0 · Data-model fixes** (`…-impact-register.md`) — PREREQUISITE. Canonical access
  (user-wide personal reads, membership-derived RLS, drop `user_id`) + universal dereference
  (`attribution_mode = named|anonymous`) + `dojo_url` dedup + `engagements.client` split.
- **WS-1 · Identity resolution** — a shared `user_id → display name` (via `dojo.identities`).
  Unblocks my-dojos, role-surfaces, incidents, members, and the audit actor.
- **WS-2 · Fixture → `/v1` read endpoints** — the bulk: governance (constitution mapper→endpoint,
  rule-packs, scopes, org-ladder), personal (projects, contributions), knowledge, org-home summary.
- **WS-3 · New tables / schema gaps** — real blockers, not just wiring: **scope-ownership**,
  **IdP/SCIM settings**, **pricing catalog**, **decisions/2-sig quorum**, **knowledge read**.
- **WS-4 · Wire mutations** — decide/approve/edit CTAs are inert across triage/approvals/identity/
  incidents/knowledge (endpoints often already exist; props just not passed).
- **WS-5 · Realtime + polish** — inbox pips/realtime, health chart, empty/error states.

## Suggested priority (dependency-aware)
1. **WS-0** data-model fixes (canon base) — with the two pending plans (inbox fidelity + RLS).
2. **Personal zone** (primary/user surface): inbox → my-dojos → projects → contributions →
   project-detail. Needs WS-1 + WS-2.
3. **Governance** (tenant): constitution (mapper→endpoint) → rule-packs → org-ladder → scopes (WS-3).
4. **Org console**: org-home → role-surfaces → identity → billing → health.
5. **Org ops**: triage → approvals (WS-3 decisions) → knowledge (WS-2/3).
6. **Client/lead**: engagements (client split) → incidents → client-audit (fix ledger).
7. **Entry**: signin (panel + dereference copy).

## Open questions for Jerry — consolidated (full detail in each spec)

**A. Draw the pre-release scope line** (many screens ask "is X in scope pre-release, or stay
honest-empty?"): cross-dōjō projects list · contributions · billing invoices/payment provider ·
incident detail · knowledge read · self-host auth.

**B. New tables / schema gaps — build now vs defer/derive** (WS-3):
scope-ownership · IdP/SCIM settings · pricing catalog · decisions (2-sig quorum) · knowledge read.

**C. Identity resolution** — join `dojo.identities` for real names (my-dojos, role-surfaces,
incidents, members, audit actor) — in scope this pass?

**D. Confirm these ride the WS-0 data-model pass** (register): inbox user-wide + drop `user_id`
+ persist `project_slug`; `engagements.client` split; dereference/attribution rewording on
project-detail · contributions · signin · scopes · engagements; role-surfaces `dojo_url`/
`attribution_default`.

**E. Per-screen semantic decisions** (the ones that change the build):
- **inbox** — 4 mockup ask-kinds ↔ 5 wire kinds; is human→sensei chat in scope; "Open in Observatory" on web?
- **constitution** — which tenant's governance resolves on `/you/rules` for a multi-dōjō user?; **rung-id bug** (`personal|stack` vs mapper's `user|technology` → empty); can the dōjō edit user-scoped stance (no cross-DB link)?
- **rule-packs** — which `namespace_id` is "you" for a personal adoption?
- **org-home** — single `/summary` endpoint vs compose; definition of "needs a maintainer".
- **client-audit** — the artifact-strip ledger OR the tenant action-audit? (conflated today — pick one).
- **approvals** — is 2-signature quorum real for v1 (build `dojo.decisions`) or just a derived review list?
- **health** — adopt the real 4 signals vs restore the mockup's 4; leak-guard/containment event source for the alert feed.
- **my-dojos** — does the user's OWN personal dōjō appear here?; does a row navigate into the org (dup of OrgSwitcher)?
