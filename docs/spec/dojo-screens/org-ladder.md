# Constitution (org authoring) — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/ladder` — `(app)/org/[slug]/[section]/+page.svelte` (section id `ladder`, Overview group) → `ScrOrgLadder.svelte`
- Mockup: dojo2-app.jsx `ScrOrgLadder` (L666); overlay `RuleEditor` (L610), `RULE_FAMILIES` (L606)
- Access axis: tenant-primary (`entity-access-model.md` §3 — "Governance: **rules** · ladder/scopes · rule-packs · constitution → Tenant `tenant_id`" + "Org console → Tenant"). The dōjō authors its OWN rules; the auth boundary is the tenant — `dojo.shared_rules` has no `tenant_id` column but the device token must resolve to a membership in the path tenant (`rules-data.ts` header; audit is written to tenant-scoped `dojo.audit_events`). This is NOT the resolution ladder (that appears at project-preview time).
- Status: PARTIAL — richest governance component. `org-ladder-state` (active section · include-map · show-excluded · RuleEditor) + `constitution-map.rulesToSections` mapper built & unit-tested. Loader feeds `orgConstitutionFor(slug)` **fixture** (only acme). RuleEditor save is presentational (`closeEditor` only). The federation endpoint `POST/GET/DELETE /v1/.../rules` (publish/pull/retract `dojo.shared_rules`) EXISTS — the natural authoring backend — but the UI is **not** wired to it; include-toggle and "Adopt pack" are local/noop.

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Header "Constitution" + "New rule" | `ladder.openNew` | opens RuleEditor (local `$state`) | have | no |
| Banner 掟 | static copy | — | have | no |
| Left rail sections (Company/Teams/Stacks) | `ladder.groups` | **FIXTURE** `orgConstitutionFor(slug)`. Real → `rulesToSections(ConstitutionRule[])` over `dojo.shared_rules ⋈ sensei.namespaces` grouped by `scope_key`, ordered by `sensei.scopes.level`. | bind→plumb | no |
| Section "N rules · M pack" caption | `s.rules.length` / `s.packs.length` | `shared_rules` per namespace / `rule_pack_adoptions` per namespace | bind→plumb | no |
| Right: "Rule packs for this stack" + "Adopt pack" | `section.packs[]` (string[]) | **FIXTURE**. Real → `sensei.rule_pack_adoptions ⋈ rule_packs.name` for the stack namespace. "Adopt pack" = noop. | bind→plumb | no |
| Right: section rules (RuleRow) | `section.rules[]` | `dojo.shared_rules.title` (→ `text`); `enforcement='mandatory'` → hard ★ (`isHardRule`) | bind→plumb | no |
| Include toggle per rule | `ladder.toggleInclude(i)` | local `$state` only; **no matching column** (`shared_rules.status` active/tombstoned is the nearest) | plumb (schema decision) | no |
| Show / hide excluded | `ladder.toggleShowExcluded` | local `$state` | have | no |
| Edit pencil → RuleEditor | `ladder.openEdit` / `onSave` | Write → `POST /v1/.../rules` (publish) exists; UI unwired; `RuleEditor.onSave` only calls `closeEditor()` | plumb | no |
| RuleEditor family chips (守/紋/理/検/技/盾) | `RULE_FAMILIES` (`org-ladder-view`) | maps to `dojo.shared_rules.rule_type`? (glyph vocabulary ≠ `enforcement`) — unconfirmed | plumb | no |
| RuleEditor ★ Non-negotiable toggle | `hard` | `sensei.enforcement = 'mandatory'` | plumb | no |

## APIs / loaders
- load(): `(app)/org/[slug]/[section]/+page.ts` returns `sections: orgConstitutionFor(slug)` from fixtures (loader comment L86-87 lists the Overview ladder as still fixture-backed, "Tier 3, needs DDL"). `rulesToSections` is ready but uncalled; `GET /v1/.../rules?since=` returns raw `dojo.shared_rules` deltas (`PulledRule[]`) that could feed the mapper after a `PulledRule → ConstitutionRule` shape adapt.
- mutations: the endpoints exist but the UI calls none — `POST /v1/.../rules` publishes (contributor+, attribution server-controlled), `DELETE /v1/.../rules/[id]` retracts (tombstone). Include-toggle and "Adopt pack" have no endpoint.
- realtime: none — federation is a pull-cursor (`seq`), not a live subscription.

## Interactions & states
- Section select (left rail) → `ladder.setActive`, resets `showExcluded`.
- Include toggle → local include-map (`includeKey = sectionId:index`); excluded count drives the "Show N excluded" control; not persisted.
- New/Add/Edit → RuleEditor overlay; Save → `closeEditor()` only (no write).
- Empty (real org): `orgConstitutionFor` returns `[]`; `createOrgLadder([])` yields empty groups — screen degrades to header + banner.

## Gap / to-do (vs mockup)
- Wire loader → `rulesToSections(...)` from a rules read (adapt the federation `GET /v1/.../rules` pull shape, or add a resolved-by-tenant read).
- Wire RuleEditor Save → `POST /v1/.../rules`; wire a delete/retract path → `DELETE /v1/.../rules/[id]`.
- "Include/exclude" per rule has no schema meaning — decide: deactivate (`status`), tombstone, or a new per-tenant "disabled" flag; currently non-persistent.
- Per-stack "Adopt pack" → `sensei.rule_pack_adoptions` (org stack namespace) — no endpoint.
- Rule `family` glyph vs `rule_type`/`enforcement` mapping is undefined (the kit stores only a glyph).
- Known federation divergence to carry: republish/retract don't advance `seq` via PostgREST inline UPDATE (documented in `rules-data.ts`) — a DDL trigger follow-up.

## Open questions (for Jerry)
1. `POST /v1/.../rules` requires `content_hash`, `namespace_slug`, `rule_type`, `enforcement`, `content`, etc. Who computes `content_hash` and picks the namespace/`rule_type` for a **dōjō-authored** rule (vs a daemon-federated one)? Is dōjō-side authoring of `dojo.shared_rules` even intended, or is that registry only ever written by the daemon's federation publish?
2. What does the per-rule "include/exclude" toggle mean in the schema — deactivate, tombstone, or a new per-tenant disable flag?
3. `RULE_FAMILIES` (守/紋/理/検/技/盾) — do these map to `shared_rules.rule_type`, and is `rule_type` a free string or a fixed vocabulary? The kit persists only the glyph.
4. Per-stack "Adopt pack" here vs personal adopt on `/you/packs` both write `rule_pack_adoptions` at different `namespace_id`s (org stack namespace vs user namespace) — confirm the split and which role may adopt at the org scope.

### Resolved design (2026-07-30)
- **Q1 → dōjō-side authoring IS intended.** maintainer/admin author org rules directly in the console via `POST /v1/…/rules`; the **Worker computes `content_hash`** server-side; the UI picks `namespace` (stack) + `rule_type`. `dojo.shared_rules` gets a dōjō-authored write path alongside daemon-federated rules.
- **Q2 → include/exclude = per-tenant SOFT-DISABLE flag** (non-destructive; re-includable). NOT a tombstone/delete. Needs a new per-tenant rule-disable mechanism (flag on the adoption / a disable row) — schema addition.
- **Q3 → `rule_type` = FIXED vocabulary = the 6 RULE_FAMILIES** (守/紋/理/検/技/盾). The editor picks from these; glyph maps to type.
- **Q4 → org-scope adoption = `maintainer`|`admin`** (same as rule-packs Q3).
- **New-Q → FIX the seq DDL-trigger:** republish/retract must advance `dojo.shared_rules.seq` so a re-pull reflects the edit (root-cause fix; the `rules-data.ts` divergence). Not an optimistic-local band-aid.
- **Depends on:** the `/v1/…/rules` author/retract wiring + the soft-disable schema addition + the seq-trigger DDL fix + role checks.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

Makes the three layers explicit; references the **Elements → data** + **APIs / loaders** sections
above for the tables/endpoints (not restated).

- **DB** — `dojo.shared_rules ⋈ sensei.namespaces` grouped by `scope_key`, ordered by
  `sensei.scopes.level` (`title`→`text`, `rule_type`, `enforcement='mandatory'`→hard ★, `status`
  active/tombstoned, `content_hash`, `namespace_slug`) · `sensei.rule_pack_adoptions ⋈ rule_packs`
  (per-stack adopt) · audit → `dojo.audit_events` (tenant-scoped). **Tenant**-primary; auth boundary =
  the device token resolving to a membership in the path tenant.
- **API** — loader `loadOrgLadder` (mock → real); real read = adapt the federation pull
  `GET /v1/…/rules?since=` (`PulledRule[]` → `ConstitutionRule` shape) → `rulesToSections`. Save =
  `POST /v1/…/rules` (publish), retract = `DELETE /v1/…/rules/[id]` (tombstone) — **both exist,
  UI unwired**; adopt-pack + include-toggle have no endpoint. **No realtime** — federation is a
  pull-cursor (`seq`), not a subscription.
- **UI** — `ScrOrgLadder` shell composing the section rail + `RuleRow`s + the `RuleEditor` overlay,
  reading `orgLadderState`.

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
// KitConstitutionSection { id; kanji; scope; group; caption; packs?: string[]; rules?: KitRule[] }
//   ← rulesToSections(ConstitutionRule[]) — grouped Company/Teams/Stacks, broad→specific
// KitRule       { kanji; text; hard; level }   ← hard = isHardRule(enforcement==='mandatory')
// RuleFamily    { kanji; label }               ← RULE_FAMILIES (守/紋/理/検/技/盾) for the editor
// EditorTarget  { rule?: KitRule }             ← RuleEditor: absent rule ⇒ add-new
```

**State** — `org-ladder-state.svelte.ts` → `createOrgLadder(sections) → OrgLadderState` (**exists**;
convention target singleton `orgLadderState`) — delegates all grouping/include math to pure `org-ladder-view`
- data: `active`, include-map, `showExcluded`, `editing: EditorTarget|null`
- `$derived`: `groups` = `sectionsByGroup(sections)` · `section` (focused) · `excluded` = `excludedCount(...)`
- methods: `setActive`, `toggleInclude`, `toggleShowExcluded`, `openNew`, `openEdit`, `closeEditor`,
  `isIncluded(i)` — the include-map is **client-ephemeral** (no schema meaning yet); Save is presentational

**Load** — `org-ladder.ts` → `loadOrgLadder()` (wired in `(app)/org/[slug]/[section]/+page.ts`, section `ladder`)
- mock-first: hand-crafted `KitConstitutionSection[]` exercising each group, a mandatory ★ rule, an
  adopted pack, excluded rules, and empty (`createOrgLadder([])` → empty groups) → build UI + tests NOW
- real (body-swap only): adapt `GET /v1/…/rules?since=` → `ConstitutionRule[]` → `rulesToSections`;
  `RuleEditor` Save → `POST /v1/…/rules`; retract → `DELETE /v1/…/rules/[id]`

**Components** (pure, semantic, own styles + `md:`)
- `ScrOrgLadder` — shell: header + "New rule" + 掟 banner + left section rail + right panel (adopted
  packs + section `RuleRow`s + show/hide-excluded). Reads `orgLadderState`.
- `RuleRow` — one `KitRule` with include toggle + edit pencil; surfaces the mandatory ★ (`rule.hard`).
- `RuleEditor` — overlay: family chips (`RULE_FAMILIES` 守/紋/理/検/技/盾, `familyKanji` default 守) +
  ★ Non-negotiable (→ `enforcement='mandatory'`) + Save/Cancel. **Mockup-match + `md:` live here.**

**Copy** (paraglide `m.<key>()` — no inline literals): `m.ladder_title()` "Constitution",
`m.ladder_new_rule()`, the 掟 banner, "N rules · M pack" caption, "Rule packs for this stack" /
"Adopt pack", "Show N excluded", editor labels, empty copy. Kanji (掟/社/組/技 + the family glyphs)
stay `KanjiToken` brand marks. Publish **attribution is server-controlled** (`attribution_mode =
named | anonymous`; dereference is the always-on publish transform, not an authoring choice).

**Realtime = State**: none — a pull re-runs `load`; there is no live `patch`. **Test seams:**
`OrgLadderState` methods + `org-ladder-view` pure helpers (no DOM, already spec'd); `RuleRow`/`RuleEditor`
with a mock prop (fidelity, incl. ★ + family); `loadOrgLadder` mock → `KitConstitutionSection[]` shape.

**New open question (three-layer):** after a `RuleEditor` Save via `POST /v1/…/rules`, how does
`orgLadderState` reflect the new/edited rule given republish/retract **don't advance `seq`** (the
documented `rules-data.ts` divergence) so an immediate re-pull may miss it — optimistic local insert
into the section, or block the Save-then-refresh path on the DDL-trigger `seq` fix?
