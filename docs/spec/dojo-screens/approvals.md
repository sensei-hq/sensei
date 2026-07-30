# Approvals — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/approvals` — served by `(app)/org/[slug]/[section]` (`section === 'approvals'`); no own endpoint — derived from `GET /v1/t/[origin]/[org]/triage`.
- Mockup: dojo2-app.jsx `ScrApprovals` (L1089)
- Access axis: **tenant-primary** — org-console governance surface. Canonical `docs/architecture/entity-access-model.md` §3 (Governance + Org console → `tenant_id`). Same MAINTAINER-floor, tenant-scoped triage read as Triage.
- Status: **PARTIAL** — the queue is REAL but derived (the high-impact slice of the triage rows); `first` (first approver) is a literal `'pending'`, and the Review/Approve buttons are dead (the component accepts `onReview`/`onApprove` but `+page.svelte` passes neither).

> **Clarification (re: "relates to relay gates").** This governance **Approvals** screen (a second maintainer signs off high-impact/safety learnings before publish) is NOT the relay-gate surface. Relay gates — `GET /v1/.../relay/gates` over `dojo.relay_inbox` (`direction='agent_to_human'`, `status='pending'`), rendered by `RelayGateCard` on `/you` + `(console)/console/relay` — are the *live-run* "needs you" approvals (run a command / write a secret). Different table (`relay_inbox` vs `triage_queue`), different plane, different screen. Don't conflate the two "approve" verbs.

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Section count | `items.length` | `data.approvals.length` ← `toKitApprovals(rows)` | have | no |
| Banner (承) copy | literal | literal in component | have | no |
| Row kanji | `a.kanji` | `kindKanji(row.kind)`; `kind` ← `dojo.artifacts.kind` | have | no |
| Row title | `a.title` | `TriageRow.title` ← `dojo.artifacts.title` | have | no |
| Scope | `a.scope` | `scopeLabel(row.owner_scope)` = `dojo.triage_queue.owner_scope` | have | no |
| First approval (who) | `a.first` | **literal `'pending'`** in `toKitApprovals` — the first-approver lives on `dojo.decisions`, not the queue row; not read | plumb | no |
| When | `a.when` | `relativeAge(row.created_at)` = `dojo.triage_queue.created_at` (**queue age, not first-approval age**) | bind | no |
| Impact chip | `a.impact` | literal `'high'` — the filter already selected high-impact rows (`impactForConfidence(confidence)==='high'`) | bind | no |
| Review button | `<K2Btn>Review` | `onReview?.(a)` prop — **not passed by `+page.svelte`** (dead) | plumb | no |
| Approve button | `<K2Btn>Approve` | `onApprove?.(a)` prop — **not passed**; the real 2nd-approval write path isn't built | plumb | no |
| Empty state | `items.length ? … : EmptyState` | EmptyState (静) when `approvals.length === 0` | have | no |

## APIs / loaders
- **Loader** `(app)/org/[slug]/[section]/+page.ts` L132–147: SHARED with Triage — one `listTriage` fetch, then `approvals = toKitApprovals(rows)`. On failure → `[]` + `triageError`.
- **Mapper** `$lib/triage-map.ts::toKitApprovals(rows)` — filters `impactForConfidence(r.confidence) === 'high'`, maps `{ id: signature, kanji: kindKanji(kind), title, scope: scopeLabel(owner_scope), first: 'pending', when: relativeAge(created_at), impact: 'high' }`.
- **No dedicated endpoint.** A true second-approval queue would read `dojo.decisions` (rows with exactly one `status='approve'` awaiting a second, for high-impact artifacts) — that read does not exist.
- **Write path (unbuilt for 2nd-approval):** the only decide write is `POST /v1/.../triage/{signature}/decide` (one verdict per maintainer). Two-signature quorum (one proposes, a second publishes) is not modelled server-side yet.

## Interactions & states
- Pure presentational list; no selection/state store. Row order = the triage rank order, filtered to high-impact.
- Review/Approve currently no-op (handlers undefined). The shared `+page.svelte` `act()`/`actionError` toast wrapper exists for console mutations but isn't wired here.
- Empty: honest EmptyState — also the degraded-fetch fallback (loader returns `[]`).

## Gap / to-do (vs mockup)
1. **Model the second signature.** Today one `dojo.decisions` `approve` resolves the row. The mockup's "one proposes, a second publishes" needs: a quorum/threshold on high-impact/safety artifacts, a queue read of "approved-once, awaiting-second", and a distinct 2nd-approval write that flips `triage_queue.state` → resolved/published only on the second signature.
2. **Real `first` approver.** Read the first `dojo.decisions` row (`maintainer_id` → display name) instead of the `'pending'` literal; `when` should be the first-approval age, not the queue-row age.
3. **Wire Review/Approve** — `+page.svelte` must pass `onReview`/`onApprove`; Review deep-links to the candidate (Triage detail), Approve calls the 2nd-approval write then `invalidateAll()`.
4. Impact is a confidence proxy (shared with Triage) — a real safety/impact tag would make "nothing safety-relevant on a single signature" enforceable rather than heuristic.
5. Published output is source-dereferenced on the publish path (universal invariant); `attribution_mode = named | anonymous` decides credit only — surface the approver's attribution mode if shown.

## Open questions (for Jerry)
- Is two-maintainer approval a real quorum gate for v1, or does Triage's single Approve suffice and this screen is just a high-impact review list? That decides whether we build the `dojo.decisions` second-signature model or keep this a derived view.

### Resolved design (2026-07-30)
- **REAL 2-signature quorum.** Build the `dojo.decisions` second-signature model:
  - **Threshold:** high/safety-impact artifacts (per triage's `dojo.artifacts.impact` field) require **2 signatures**; normal-impact resolves on Triage's single Approve.
  - **Queue read:** "approved-once, awaiting-second" — `dojo.decisions` rows with exactly one `status='approve'` awaiting a second, for high/safety artifacts.
  - **2nd-approval write:** a distinct second-signature write that flips `triage_queue.state` → resolved/published **only on the second signature**.
  - **Real `first` approver:** first `dojo.decisions` row (`maintainer_id` → display name via identity); `when` = first-approval age, not the queue-row age.
- **Depends on:** triage's impact field (gates which need 2 sigs) + the second-signature model on `dojo.decisions` (threshold + awaiting-second read + 2nd-write) + WS-1 identity (approver name).
- What routes a candidate to Approvals — confidence ≥0.90 (current proxy), or an explicit "needs 2nd approval" flag set at first-approval time?
- Should Review open the full Triage candidate detail in place, or a lighter modal? (Depends on the Triage rich-detail read landing.)
- Confirm the naming split with relay gates stays — this screen is governance-publish approval, `/you` relay gates are live-run approvals. Any desire to unify the "needs you" surfacing across both?

## Components & state (three-layer, per `sensei:ui-state-pattern`)

> **Not relay gates.** This governance Approvals surface (a 2nd maintainer signs off high-impact/safety
> learnings before publish) stays distinct from `/you` relay gates (`dojo.relay_inbox`, live-run
> "needs you"). Different table, plane, screen — the three layers below are the triage/`decisions`
> plane only.

**DB** (reference §Elements→data): today = the high-impact slice of `dojo.triage_queue` (derived; `first`/`when` are literals). A real 2nd-signature queue would read `dojo.decisions` (rows with one `status='approve'` awaiting a second, for high-impact/safety artifacts) — **that quorum model does not exist server-side yet** (NEW table/threshold).

**API** (reference §APIs/loaders): **no dedicated endpoint** — shares the Triage read `GET /v1/t/{tenant}/triage`. The 2nd-approval write (flip `triage_queue.state`→published only on the second signature) is **unbuilt**; the only decide write is the single-verdict `POST …/triage/{signature}/decide`.

**Domain types** (UI-shaped):
```ts
type Approval = { id; signature; kanji; title; scope;
  first: Approver|null; when: string; impact: 'high'|'safety' }
type Approver = { name: string; attributionMode: 'named'|'anonymous' }
```

**State** — `approvals-state.svelte.ts` → `approvalsState`
- data: `approvals: Approval[]`
- `$derived`: `count`, `shown` (rank order, high-impact)
- methods: `load(approvals)`, `review(id)` (deep-link to the Triage candidate detail),
  `approve(id)` → 2nd-signature write → `invalidateAll` (no-op until the quorum write exists)

**Load** — `approvals.ts` → `loadApprovals()`
- mock-first: hand-crafted `Approval[]` with a real `first` approver + `when`, high/safety impact,
  and empty — build UI + tests to fidelity NOW
- real (body-swap only): interim = derive `toKitApprovals(listTriage rows)`; the correct read is the
  "approved-once, awaiting-second" `dojo.decisions` query once the quorum model lands. `first.name`
  via the shared `user_id → display name` resolution (WS-1); honor `attribution_mode = named|anonymous`

**Components** (pure, semantic, own styles + `md:`; NO `K2*`)
- `ApprovalList` — banner (承 `KanjiToken`) + `ApprovalRow[]` from `approvalsState.shown` + EmptyState (静).
- `ApprovalRow` — kind glyph (Solar) · title · scope · first-approver · when · impact chip ·
  Review/Approve. **Mockup-match + `md:` here.**
- Review/Approve via **`@rokkit/forms`** (Approve = confirm + optional note); `+page.svelte` passes
  `onReview → state.review` / `onApprove → state.approve` (today it passes neither → dead buttons).
- Shell reuses `(app)/org/[slug]/[section]`; `+page.ts` = Load wiring → `approvalsState.load`.

**Copy** (paraglide `m.<key>()`): banner / empty / button copy in `messages/en.json`; 承 stays a
`KanjiToken`, kind/action glyphs are **Solar icons**. Approver credit honors `attribution_mode =
named|anonymous` (dereference is separate + always-on on the publish path).

**Realtime = State**: none. **Test seams:** state methods (no DOM); `ApprovalRow` with a mock prop
(fidelity); Load mock → shape.
