# Client audit — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/clientaudit` — `(app)/org/[slug]/[section]` with section=`clientaudit` (distinct from admin section `audit` → `ScrRoleSurfaces`). Read: `GET /v1/t/{origin}/{org}/audit` (ADMIN floor).
- Mockup: dojo2-app.jsx `ScrClientAudit` (L1240)
- Access axis: tenant-primary — org console, keyed by `tenant_id` (entity-access-model §3). Backed by `dojo.audit_events` filtered by `tenant_id`. Canon §5: `audit_events` columns are a subset covered by the universal strip so the exported ledger never leaks source refs — this screen's whole job is to *prove the always-on dereference held* (a `false`/blocked row is the red-fail), not to toggle it.
- Status: PARTIAL — screen renders real rows, BUT it is bound to the **admin action-audit** (`dojo.audit_events` via `GET …/audit`), not the intended per-engagement artifact-strip ledger; `ok`/`kanji`/`client` are regex heuristics, and Filter/Export + footer chips are unwired fixtures. The component even claims "distinct from the admin action-audit" while reading exactly that.

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
| --- | --- | --- | --- | --- |
| Header "Client audit trail" | title | static | have | no |
| Banner "immutable ledger…" | static copy | literal in `ScrClientAudit.svelte` | have (static) | no |
| Row glyph | `r.kanji` (却/共/録) | `ledgerKanji(action)` heuristic — block/reject→却, publish/share→共, else 録 (`incidents-map.ts`) | **bind** — derived by regex on `action`, not a stored classification | no |
| Row event | `r.event` | `dojo.audit_events.action` (e.g. `member_added`, `incident_opened`) | have (but action verbs are admin/governance, not strip events) | no |
| Row detail | `r.detail` | `audit_events.target` else `JSON.stringify(detail)` | have | no |
| Row client chip | `r.client` | `audit_events.engagement_id.slice(0,8)` else `'—'` | **bind** — short uuid, not client name; most audit rows have null `engagement_id` → `'—'` | no |
| Row held ✓ / blocked shield | `r.ok` | `entryHeld(action)`: `false` iff action matches `block\|decline\|reject\|deny` | **bind** — heuristic, not a real strip verdict | no |
| Row time | `r.t` | `relativeAge(audit_events.ts)` | have | no |
| Filter button | — | none (inert) | **plumb** — no filter wired (by engagement/date) | no |
| Export button | — | none (inert) | **plumb** — `GET …/compliance/export?engagement=&format=csv\|json` exists in `client-data.ts` but not called | no |
| Footer "Retention · 7 years" | static chip | hardcoded in `ScrClientAudit.svelte` | **bind/plumb** — real source = tenant retention policy | no |
| Footer "Client read-access · Globex on" | static chip | hardcoded (literal "Globex") | **bind/plumb** — fixture, not per-tenant | no |

## APIs / loaders
- Loader `+page.ts` → `guardedFor('clientaudit', [], listAudit, toKitClientAudit)`, behind `guardTenantScope`. `listAudit` = admin-data client → `GET …/audit` (ADMIN floor). Degrades to `[]` + `clientAuditError`.
- Read API: `GET /v1/t/{origin}/{org}/audit?limit=N` (`audit/+server.ts`, `ACCESS.admin`) → `{ events: AuditEvent[] }`, most-recent-first, over `dojo.audit_events` filtered by `tenant_id`. Store: `server/admin-data.ts::listAudit`.
- **Intended (not wired) source:** the confidentiality ledger the mockup/spec describes is the per-engagement artifact-strip trail — `dojo.artifacts` with `dereferenced` status, served by `GET …/audit/artifacts?engagement=&dereferenced=true` (`AuditArtifact` in `client-data.ts`, where `dereferenced=false` is the red-fail that blocks export). Mapper: `incidents-map.ts::toKitClientAudit`. `dojo-lead-console.md` done-gate: audit-view row count = `count(dojo.artifacts where engagement_id=x and dereferenced=true)`.

## Interactions & states
- Read-only list; no row actions.
- Empty → dedicated `EmptyState` ("The ledger is empty.").
- Filter / Export → present but inert.
- Loader failure → empty ledger + surfaced error (honest-empty).
- Append-only by nature (`audit_events` is insert-only, `bigserial`); no client-side mutation.

## Gap / to-do (vs mockup)
- **Repoint the source**: bind to the per-engagement artifact-strip ledger (`…/audit/artifacts` over `dojo.artifacts.dereferenced`), or explicitly redefine this screen as the tenant action-audit and remove the "distinct from admin action-audit / what was stripped" framing. Today it is the admin action-audit under a confidentiality-ledger UI — a source/semantics mismatch.
- `ok` should be the real strip verdict (`artifacts.dereferenced`), not a regex on the action verb.
- Wire Export to `GET …/compliance/export` (CSV/JSON), gated red-fail-blocks-export per the done-gate.
- Wire Filter (by engagement / date range).
- Client chip should resolve the client name; footer chips should reflect real per-tenant retention + read-access policy.
- Enforce the wrong-gate: a client-work artifact in the audit without strip info, or any `dereferenced=false`, must render a red fail and block export.

## Open questions (for Jerry)
- Is this screen the **artifact-strip ledger** (`dojo.artifacts` / `…/audit/artifacts`, per the lead-console spec + mockup copy) or the **tenant action-audit** (`dojo.audit_events`)? The two are conflated today — pick one and align the copy + source.
- If the artifact ledger: does it need an engagement selector, or list all engagements' artifacts for the tenant?
- Export format (CSV / JSON / PDF) and the exact strip-covered column subset — confirm before wiring `compliance/export`.

### Resolved design (2026-07-30)
- **Source → CONFIDENTIALITY/CONTAINMENT LEDGER** over `dojo.audit_events` filtered to `{published/shared, contained/held}` events, **per engagement** (engagement selector = yes). NOT the general action-audit; NOT the retired artifact-strip ledger.
- **Rule B reframe:** the old `…/audit/artifacts?dereferenced=true` source + `AuditArtifact.dereferenced` red-fail are RETIRED (removed in `a7140fbf`; the `dereferenced` column is dropped). Rebind to the audit-event filter. Semantics flip: `published` = crossed (source-stripped **by construction**, always-on); `held/contained` = the guard blocked it (the containment events defined on the **health** screen, `action='contained'/'held'`). Drop the "dereferenced=false = broken strip red-fail" framing — a held row is the guard WORKING, not a failure.
- **`held` domain field** = "was this a contained/held event" (guard blocked) vs published (crossed) — a reframed observed result, not the old per-artifact strip verdict.
- **Export → CSV** (source-ref-free subset; universal dereference respected; consistent with role-surfaces audit export).
- **Depends on:** the containment events on `dojo.audit_events` (health-screen seam) + a per-engagement audit-event filter/read + CSV export. Copy reworded (no "broken strip" / "distinct from admin action-audit" theater).
- Retention (7 yr) + client read-access — real `dojo.policies` values, or display copy for now?

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB — the wrong-ledger finding lives here.** Two candidate sources; the Load layer is the seam that PICKS (cols in *Elements → data* / *APIs* above — not restated):
- **currently bound (wrong):** `dojo.audit_events` (admin action-audit) via `GET …/audit` — `ok`/`kanji`/`client` are regex heuristics on `action`, not stored verdicts, and most rows have null `engagement_id`.
- **intended:** the per-engagement artifact-strip ledger `dojo.artifacts` (`dereferenced` status) via `GET …/audit/artifacts?engagement=&dereferenced=true` — this is what "prove the strip held" means. `dereferenced=false` is the **red-fail** that blocks export.

Tenant-scoped, append-only. Canon §5: the screen's whole job is to prove the always-on dereference held — a blocked/`false` row is the red-fail, never a mode toggle (`attribution_mode = named | anonymous` is credit only).

**API** — reuse the documented `GET …/audit` (bound, `ACCESS.admin`) vs the intended `GET …/audit/artifacts` + `GET …/compliance/export?engagement=&format=csv|json` (exists in `client-data.ts`, uncalled).

**UI** (components / state / types):

**Domain type** — shaped for the INTENDED artifact-strip ledger so the real swap is body-only:
```ts
type AuditEntry = { id; event: string; detail: string|null; clientName: string|null;
  held: boolean; when: string }   // held = artifacts.dereferenced (real verdict), NOT a regex on action
```
`held=false` (or a client-work artifact missing strip info) → red-fail row. No attribution/strip toggle — `held` is the observed result of the always-on dereference.

**State** — `client-audit-state.svelte.ts` → `clientAuditState`
- data: `entries: AuditEntry[]`, `filter` (engagement / date range), `error`
- `$derived`: `shown` (filtered), `hasRedFail = entries.some(e => !e.held)`, `exportBlocked = hasRedFail`
- methods: `load(entries)`, `setFilter(f)`, `export(format)` (calls `…/compliance/export`, guarded by `exportBlocked` — the wrong-gate)

**Load** — `client-audit.ts` → `loadClientAudit(tenantKey, { engagement? })` — **THE source-repoint seam:**
- mock-first: `AuditEntry[]` carrying a real `held` verdict (include a `held:false` red-fail case) shaped from the artifact-strip ledger → build to fidelity NOW
- real (body-swap only): call `GET …/audit/artifacts` over `dojo.artifacts.dereferenced` — **NOT `listAudit`/`…/audit`**. Component/state stay put; only this body changes. (Decision gate, open below: OR redefine the screen as the tenant action-audit and drop the "what was stripped / distinct from admin action-audit" framing — pick one before building.)

**Components** (pure, semantic, own styles + `md:`) — replace `ScrClientAudit`:
- `ClientAuditLedger` — shell: header + banner + Filter + Export (disabled when `exportBlocked`) + `AuditRow[]` + policy footer (Retention / Client-read-access from state, not literals)
- `AuditRow` — `KanjiToken 却/共/録` (or Solar icon) from the stored classification, event/detail, clientName, held ✓ / blocked-shield (**red when `!held`**), when. **Mockup-match + `md:` here.**
- Filter / Export via `@rokkit/forms` (engagement selector + date range; export format)

**Copy** — paraglide `m.<key>()` (banner "immutable ledger…", labels, empty/error); held/blocked glyph as `KanjiToken`/Solar; footer retention/read-access from policy; universal-strip framing (the ledger proves the strip; no per-item toggle).

**Realtime = State**: none (append-only ledger; refetch). **Test seams:** state — assert `exportBlocked` true when a `held:false` row is present (the wrong-gate); `AuditRow` red-fail render with a mock; Load mock → shape.

**New open question:** confirm the source pick (artifact-strip `…/audit/artifacts` vs action-audit `…/audit`) **before** building — the `held` verdict assumes the artifact ledger; if action-audit wins, `held` has no real backing and the confidentiality framing must be dropped.
