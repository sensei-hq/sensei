# Contributions — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.

- Route: `/you/contributions` = `(app)/you/[section]/+page.svelte` (`section === 'contributions'` branch) + `[section]/+page.ts`
- Mockup: dojo2-app.jsx `ScrContributions` (L862) — board "1d · Contributions — shared upstream + approved for you"
- Access axis: **user/membership-primary** — canonical `entity-access-model.md` §3 row 2: "Projects, contributions (`/you`) … Primary axis = User." What you shared (upstream) + what's approved for you (downstream), across ALL your dōjōs; keyed off the user, not one tenant.
- Status: **STUB** — the screen (`ScrContributions.svelte` + `personal-view.tallyContributions`) is built and faithful, but the loader returns empty arrays + a zeroed stat. No `/v1` contributions endpoint feeds it.

## Elements → data (contract)
Live: `[section]/+page.ts` → `{ contributionsMine: [], contributionsDownstream: [], contributionsStat: {0,0,0} }` → `ScrContributions.svelte`.

| Element | Mockup field | Source (loader/API/table.field) | Status | Realtime? |
|---|---|---|---|---|
| SectionHead eyebrow/title | `You · sharing` / `Contributions` | static | have | — |
| stat: approved | `stat.approved` | `tallyContributions(mine)` — derived from rows (0 while empty) | have (empty) | — |
| stat: in triage | `stat.pending` | `tallyContributions(mine)` | have (empty) | — |
| stat: devs helped | `stat.helped` "lifetime" | `contributionsStat.helped` — **no source** (fixture had 612; loader passes 0, honest) | plumb | — |
| banner (propose/decide + anonymize) | static copy | static | have | — |
| "What you've shared" section | `mine.length` | `data.contributionsMine` — **empty; NO source** | plumb | — |
| shared row: kanji/title | `c.kanji`/`c.title` | dōjō `artifacts` / `upstream_queue` (contribute path) — not read | plumb | — |
| shared row: dest chip | `c.dest` (+ `c.client` shield) | destination dōjō (bound membership) | plumb | — |
| shared row: scope · note | `c.scope`/`c.note` | proposed scope + outcome note | plumb | — |
| shared row: status chip | `c.status` approved/pending/declined → `CSTAT` | `dojo.upstream_queue` / `dojo.engagement_status` state | plumb | maybe |
| shared row: when | `c.when` | created/decided timestamp | plumb | — |
| "Approved for you" section | `downstream.length` | `data.contributionsDownstream` — **empty; NO source** | plumb | — |
| downstream row: title/from/scope/when | `it.{title,from,scope,when}` | knowledge approved-for-you from the user's dōjōs (`artifacts` adopted / distribution) | plumb | maybe |
| downstream row: adopted vs new | `it.adopted` | local adoption state | plumb | — |
| downstream row: Pin btn | `pin` | `onPin` — **not passed** on this route; inert | plumb | — |
| empty state | (mockup always has rows) | sections render empty (no dōjō-added empty-state; `count=0`, no rows) | have (empty) | — |

## APIs / loaders
- **load()** (`[section]/+page.ts`): returns `contributionsMine: []`, `contributionsDownstream: []`, `contributionsStat: { approved:0, pending:0, helped:0 }`. Comment: "No backing route yet → honest empty, never fabricated (F4)."
- **mutations**: none. `onPin` (adopt a downstream item locally) is unwired.
- **realtime**: none. (A contribution's status flip — pending→approved/declined — is a natural realtime candidate on the dōjō triage tables.)
- **Would-be source**: the contribute pipeline exists daemon-side (`crates/senseid/src/dojo/{contribute,attribution,routing}.rs`) and lands in dōjō tables (`dojo.artifacts`, `dojo.upstream_queue`, `dojo.engagements`/`engagement_status`). "What you've shared" = the user's rows across those queues; "Approved for you" = adopted/distributed knowledge targeting the user. **No `GET /v1/…/contributions` route exposes either.**

## Interactions & states
- **Two sections**: what you've shared upstream (per-destination status) + what's been approved for you downstream (pin-to-adopt). Both render empty today.
- **Stat row**: `tallyContributions` recomputes approved/pending from `mine` so the header stays honest against the list; `helped` is a carried lifetime figure (0, no source).
- **Pin**: mockup `onPin` adopts a downstream item locally; unwired here.
- **Client work**: shared rows with `c.client` show a shield chip — anonymized before leaving. Align copy/behavior with universal source-dereference (canon §5): dereference is always-on for ALL work, not only client (banner currently says "Client work is anonymized before it leaves").
- **Responsive**: `ScrContributions` hard-codes `p-8 gap-6`; no `mobile` prop from the route (shell handles phone chrome). The stat row is desktop-only in the mockup (`!mobile`); here it always renders — verify on phone.

## Gap / to-do (vs mockup), ranked
1. **No data source** — add a user-wide `GET /v1/…/contributions` returning `{ mine, downstream, stat }` from `dojo.artifacts`/`upstream_queue`/`engagement_status`, keyed off the user across all memberships. Everything blocks on this.
2. **Access axis** — the endpoint must aggregate across all the user's dōjōs (user-primary), not filter one tenant.
3. **Dereference wording (canon Rule B)** — reword the banner + the `client` shield framing to the universal invariant (`attribution_mode = named | anonymous`; dereference is always-on, not client-only). Tracked in `data-model-fix-impact-register.md` Part 2.
4. **`helped` (devs helped, lifetime)** — define the real metric or drop the stat (don't ship a fabricated number; 0 is the current honest placeholder).
5. **Pin / adopt** — wire `onPin` to a local-adopt mutation, or mark downstream as read-only for now.
6. **Empty states** — the sections render header-with-0-rows; consider an explicit per-section empty message for the honest-empty case.

## Open questions (for Jerry)
1. Which dōjō tables are the contributions source of truth for the web app — `dojo.upstream_queue` (proposed→triaged) for "mine" and `dojo.artifacts` (adopted/distributed) for "approved for you"? Confirm before shaping the endpoint.
2. "Approved for you" — is downstream adoption a dōjō concept (pin here) or purely a desktop-app/daemon concern (the app pulls approved knowledge)? If the latter, this section may be app-only.
3. `attribution_mode` after the Rule-B change is `named | anonymous` (credit only) and dereference is always-on — does the shared-row "client shield" become just an `anonymous` marker, and does the copy stop implying non-client work is un-stripped?
4. "Devs helped (lifetime)" — is there a real downstream-impact metric, or drop it?
5. Is a cross-dōjō personal contributions view in scope pre-release, or does it stay honest-empty until the contribute pipeline is federated to a dōjō-readable route?

### Resolved design (2026-07-30)
- **Q1 'mine' source → `dojo.upstream_queue`** (proposed→triaged), user-scoped, carrying `attribution_mode`.
- **Q2 + (a) 'Approved for you' → FULL dōjō concern:** show the approved-for-you list (from `dojo.artifacts` distributed to the user) AND build the Pin/adopt WRITE — `POST …/contributions/adopt`.
- **Q3 + (c) (Rule B):** `attribution_mode = named|anonymous`, dereference always-on → the "client shield" becomes an **`anonymous` marker rendered from `upstream_queue.attribution_mode`** (data-driven, not a heuristic); copy stops implying non-client work is un-stripped.
- **Q4 + (b) 'Devs helped (lifetime)' tile → DROP** (no real metric; no fabricated 0).
- **Q5 scope → HONEST-EMPTY until federated:** the endpoints/mutation/source are buildable now, but the LIST stays honest-empty until the **contribute pipeline federates** daemon contributions into `dojo.upstream_queue` via a `/v1` route. No fabricated rows in the meantime.
- **Depends on:** contribute-pipeline federation (daemon → `dojo.upstream_queue`/`/v1`) + WS-0 Rule A (user-wide read) + the adopt mutation endpoint.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** `dojo.upstream_queue` (mine: proposed→triaged) · `dojo.artifacts` (approved-for-you:
adopted/distributed) · `dojo.engagement_status`; Pin = a local-adopt write · **API** Load `loadContributions`
in `contributions.ts` (today `{mine:[], downstream:[], stat:0}`); would-be `GET /v1/…/contributions`
(user-wide) + `POST …/contributions/adopt` for Pin · **UI** `contributionsState` + `ContributionsView` over the
`Contribution`/`DownstreamItem`/`ContributionStat` types.

**Domain types** (UI-shaped; Load maps the queue/artifact rows → these):
```ts
type Contribution = { id; title; icon?: string /* Solar */; dest: string; anonymous: boolean;
  scope; note?: string; status: 'approved'|'pending'|'declined'; when: string }
type DownstreamItem = { id; title; from; scope; when: string; adopted: boolean }
type ContributionStat = { approved: number; pending: number; helped: number }
```
`anonymous` replaces the mockup's `client` shield — per canon Rule B (`attribution_mode = named | anonymous`,
dereference always-on for ALL work) it's a plain anonymity marker, not a client-only flag.

**State** — `contributions-state.svelte.ts` → `contributionsState`
- data: `mine: Contribution[]`, `downstream: DownstreamItem[]`, `helped: number`
- `$derived`: `stat` (recompute `approved`/`pending` from `mine` — replaces `tallyContributions`; `helped`
  carried through)
- methods: `load({mine, downstream, helped})`, `pin(id)` (optimistic local-adopt → flip `adopted`),
  `patch(contribution)` (realtime status flip)

**Load** — `contributions.ts` → `loadContributions()`
- mock-first: hand-crafted `mine`/`downstream` exercising approved/pending/declined · anonymous vs named ·
  adopted vs new · empty → build to fidelity NOW
- real (body-swap only): **user-wide** aggregate across ALL the user's dōjōs (not one tenant) —
  `upstream_queue`→`mine`, `artifacts`→`downstream` → domain types.

**Components** (pure, semantic, own styles + `md:` — fidelity verified per component)
- `ContributionsView` — shell: `SectionHead` · `StatRow` · banner · `SharedSection` (mine) · `ApprovedSection`
  (downstream) + per-section `EmptyState`. Reads `contributionsState`. (replaces `ScrContributions`)
- `StatRow` — approved / in-triage / devs-helped tiles (`helped` shown only when sourced).
- `ContributionCard` — one `Contribution`: Solar icon · title · dest chip (+ anonymous marker) · scope·note ·
  status chip (Solar-toned approved/pending/declined, replaces `CSTAT`) · when. **Mockup-match + `md:` here.**
- `DownstreamCard` — one `DownstreamItem`: title · from · scope · when · adopted-vs-new · Pin btn
  (`onpin→state.pin`).

**Copy** (paraglide `m.<key>()`): `m.contributions_title()`, stat labels, banner reworded to the **universal**
dereference invariant (§5 — not "client work is anonymized"), status labels, per-section empty copy. Kanji
stays a `KanjiToken` brand mark; status/kind glyphs are Solar icons.

**Realtime = State**: a contribution's pending→approved/declined flip is a natural channel →
`subscribe()`→`patch`. **Test seams:** `contributionsState` stat recompute + `pin` optimistic flip (no DOM);
`ContributionCard`/`DownstreamCard` with mock rows (each status/anonymous/adopted variant); Load mock → shape.

**New open questions (from this exercise):** (a) Pin needs a real write — is downstream adopt a dōjō mutation
(`POST …/contributions/adopt`) or purely a desktop-app/daemon concern (then `DownstreamCard` Pin is
read-only/dropped here)? (b) `helped` (devs-helped lifetime) still has no source — the type carries it but Load
can't fill it: define the metric or drop the tile. (c) is `anonymous` a per-row field readable from
`upstream_queue.attribution_mode`, so the marker renders from data, not a heuristic?
