# Dōjō + Relay mockup review — issues & corrections for the designer

> Scope: the 11 `Sensei/lib/dojo/*.jsx` screens, reviewed after Relay was folded
> into the Dōjō and the surface became a **responsive SaaS site**. Judged against
> the **Dōjō journey map** (`Sensei/Sensei Dōjō Journey Map.html`) and the
> design-system conventions (`Sensei/CLAUDE.md`). Each item is an issue + a
> concrete fix. Work the priority order in §6.
>
> *(Kept at `docs/mockups/` — outside `Sensei/` — so it survives replacing the
> whole `Sensei/` mockup folder.)*

---

## 1 · Systemic — fix once, across every screen (highest leverage)

These recur in **all 11 screens**; fixing them at the primitive level clears most per-screen noise.

| # | Issue | Fix |
|---|---|---|
| S1 | **~0% design-system adoption.** Every screen is hand-rolled inline `style` over the **deprecated** numbered tokens (`--paper-2/-3`, `--ink-2/-3/-4`, `--edge`, `--hairline`) — none use the semantic utilities (`bg-paper-soft`, `text-ink-mute`, `border-paper-edge`) or `zs-*` components. | One migration pass to semantic tokens + utility classes + `zs-btn`/`zs-badge`/`zs-card`/`zs-input`, matching `lib/assistant-card.jsx`. This is the #1 fix; everything else rides on it. |
| S2 | **Raw hex / oklch / color-mix literals** in ≥7 screens: `#fff` (relay Send-answer), repeated `oklch(0.58 0.15 35/.NN)` accent-alpha borders, `oklch(0.52 0.13 60)` amber, per-kind `color-mix`. **Breaks dark mode.** | Add the two missing tokens **`--accent-edge`** and **`--warning-edge`** (only `--success-edge` exists today), bake per-kind chip colors into `-soft`/`-edge` pairs, and delete every inline color. |
| S3 | **"declined / never / retract" are colored amber (warning)** though a `--danger` family exists. | Route declined/never-share/decline/supersede/retract → **`danger`**; keep amber strictly for *caution/inferred/flagged*. |
| S4 | **Off-scale type everywhere** — `9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 26, 34` recur (scale is 11/13/15/17/22/28/40). | Snap to scale / named type classes: hero→`zs-hero`, section headers→`zs-display-lg`, eyebrows→`zs-eyebrow`, mono meta→`zs-meta`. |
| S5 | **Responsive is inconsistent — the whole point of this revision.** `relay` + `shared` adapt (`wide`/`mobile` props); **`saas`, `inapp`, admin·Monitor/Scopes, `governance`, maintainer·Candidate/Knowledge, `lead`, `billing`, `extensions`** use fixed multi-column grids + fixed-px columns and do **not**. Worst: (a) `saas` sign-in/orgs/create — the *first* screens a phone user hits; (b) maintainer **Candidate** — the primary **Approve button is off-canvas on mobile**. | One responsive contract: thread `mobile`, stack columns, turn fixed-px tables into card-per-row, make primary actions a sticky bottom bar. Adopt the `relay`/`shared` pattern everywhere. |
| S6 | **Duplicated primitives, drifting:** 5 button recipes; `Metric`/`Signal`/stat blocks; **3** "Live" pills; **3** condensed mobile headers; **3** kind/origin→color maps; **2** competing mobile tab bars (`DojoMobileTabs` derived vs fixed `MOBILE_TABS`); `Panel` ×2. | Hoist a shared kit into `dojo-shared`: `Live`, `MobileHeader`, `StatCard`, `Panel`, `ListRow`, `Toggle`, `SegmentedControl`, `MeterRow`, one `DojoChip(kanji, tone)`, one exported kind/origin map. Pick **one** mobile tab bar (the fixed Projects/Inbox/Chat/More matches the journey best). |
| S7 | **Empty / loading / error / offline states absent cluster-wide.** Given Relay's realtime architecture, **offline / session-ended / daemon-asleep are primary failure modes, not edge cases.** | Add a state matrix per screen. First-class Relay states: "nothing needs you", "connection lost — can't reach this session", "session ended", "daemon asleep". |
| S8 | **Terminology drift — same concept, different words to the same user.** Client confidentiality is **anonymize** (billing, lead header, journey) vs **dereference** (lead audit + events, developer role strings) vs **strip** (chips). Also **Client vs Dereferenced**; **Knowledge / Library / Catalog / Extensions** (4 names, 2 things); lifecycle **active/deprecated/disabled/retracted/evicted/quarantined**. | Pick one canonical verb — **"anonymize"** — everywhere in UI copy; retire dereference/strip from the surface. Settle the knowledge-vs-catalog taxonomy. Define **one** published-item state machine + one alert state machine and reuse the words + chip colors everywhere. |
| S9 | **Kanji used as load-bearing iconography with no legend** (e.g. bare 客/隔/盾 markers). Enterprise buyers / non-JP readers can't decode them. | Keep kanji **decorative-beside-label only**; add a shared glyph legend/tooltip; never a glyph as the sole marker of a functional state. |

---

## 2 · Theme promises stated in copy but not operational (product-critical)

The journey map's trust themes are asserted in labels but the screens don't deliver them. These are **more important than the cosmetic pass** — they're the product's core promises.

- **"Never blind — always preview before irreversible."** Missing or unwired in most places it's promised:
  - **In-app share → the raw-vs-stripped redaction preview screen does not exist** (the headline commitment of the contributor flow). **Build it.**
  - Maintainer **"who gets this" dry-run** ("Preview recipients →") is unwired; **"preview changes" before Approve** (relay) is missing (only prose risk chips).
  - Admin **"Retract downstream"** fires with no confirm/preview of *who gets un-taught* — it's destructive fan-out; add the same repos·devs dry-run.
  - Governance **stance dials** change what leaves a scope with no consequence line; **"Preview onboarding"** is unwired and has no defined target.
- **"Specificity wins conflicts — both rules shown, winner marked."** Only the maintainer conflict-diff does this. The two places it should live — the **admin precedence ladder** ("drag to reorder" with no effect) and **governance inheritance** — show neither the losing rule nor a live winner; the precedence "simulator" is **static text**. Couple the ladder to a live "which rule wins" verdict; show inherited rules in-context, greyed + source-tagged, with overrides marked.
- **"Authored once, inherited on join."** Governance never shows inherited items in-context — only a rolled-up tally that **double-counts** a rule defined at two scopes. Show inherited-vs-defined per section; dedupe the count.
- **Provenance ("reached you through your Dōjō")** is on Relay **Approve** only — absent on **Decision / Stall / Chat** and the in-app downstream lane. Standardize a provenance line on every pulled/pushed surface (pull-never-push).
- **"What happens next" is missing** — terminal actions (Approve/Deny/Ask, Send-answer, Adopt/Defer, Share) are dead with no post-action state (approved→running, answer-sent→ack, share→queued). Relay **Inbox detail is hardcoded** and doesn't follow the selected row. Wire selection + add acknowledged states.

---

## 3 · Missing screens & flows (gaps against the journey map)

- **Create-a-Dōjō → pick plan** onboarding funnel — **the single biggest gap.** The journey says "plan is chosen at creation" (visibility public/personal = free · private/shared = paid), but billing + lead both *assume* an already-configured org. No creation/visibility→plan flow exists.
- **Redaction preview** (raw vs stripped, before share) — in-app. *(also §2)*
- **Register-client form** — routing only, but needs contract term, retention, severity tiers, read-access grant. Button exists; form doesn't.
- **Identity / SSO / SCIM setup + git-role mapping** — referenced (Members dropdown mentions SCIM) but no screen; a first-run admin pillar with no home.
- **Plan & billing** is routed from the admin nav but **`DojoBilling` isn't defined/imported in `dojo-admin`** — a dead route. (The screen exists in `dojo-billing.jsx`; wire it in.)
- **Browsable / filterable audit trail** — Monitor and Governance both *claim* an audit trail; no browsable log exists.
- **Second-approver's action queue** — "Approve & request 2nd" has no "awaiting my approval" inbox for the second approver.
- **Maintainer "Revise" editor** — a named journey step; button leads nowhere.
- **Recall flow** — in-app share copy promises "recall until approved"; no recall control in Contributions/Travel.
- **Relay logs screen** — "full log →" / "View logs" lead nowhere.
- **Billing:** payment-method / tax-id management, **past-due / dunning** state, **seat roster** ("which 34 are billable"), and **live Relay meters** (concurrency/inbox/presence usage, not just entitlement rows).

---

## 4 · Trust / data-integrity bugs (fix — these mislead)

- **Maintainer `REACH` silently falls back to `4 repos · 8 devs`** for any scope not in its hard-coded map — a false number on the "who gets this" **trust** surface. Reach must come from real scope data; never a silent default.
- **Governance inherit counts double-count** a rule defined at two scopes → "N rules inherited" can lie. Dedupe / apply override logic.
- **Extensions approver is fabricated round-robin** (`["Keiko T.","Marco D.","Sven K."][i%3]`) — reads as a real reviewer assignment. Don't fabricate provenance.
- **Billing "active contributor" is never defined** yet it's the entire billing basis. Define it in-product (e.g. "contributed or had a lesson attributed this period") + tooltip + seat drill-down.
- **In-app `InappShare` "Share 2 to Dōjō"** count is hardcoded and won't track checkbox selection.

---

## 5 · Cognitive-load hotspots (per screen)

- **Governance** — front-loads scope picker + 3 stance dials + 4 authoring sections + project memory + inherit card at once; can't tell *authored-here* vs *inherited*. Separate the two; give rules/guards primacy; collapse skills/agents/commands.
- **Admin · Overview** — 5 competing focal regions (4 metrics + triage + confidentiality + activity + published-health table). Lead with triage + confidentiality; demote the rest below the fold / behind a tab.
- **Admin · Scopes** — 3 dense mental models side-by-side (tree + ladder + simulator). Give the **simulator** primacy; collapse the tree.
- **saas · DojoOrgs** card packs 8+ data points/row; **DojoCreate** stacks 4 radio groups. Demote meta to one muted line + one primary Enter; progressive-disclosure the create form (default SaaS + GitHub, "Advanced" for self-host/solo).
- **In-app · Share** — 5-column row with 4–5 chips each. Move attribution/confidence to a second muted line.
- **Billing** — the plan story is told 4× (header + stats + tiers + metering); **Lead** states the universal model ~4×. Say each once; let the canonical table/grid carry it.
- **Extensions** — toolbar = 2-tab toggle + up to 7 kind chips wrapping to multiple rows. Collapse kinds to a "Kind ▾" dropdown or one scrolling row.

---

## 6 · Suggested priority order for the designer

1. **S1–S4 token/utility/type migration + the two missing tokens** (`--accent-edge`, `--warning-edge`) and the danger-vs-warning split — unblocks dark mode + kills most drift. *(mechanical, high value)*
2. **S6 shared primitive kit** + **S8 terminology/lifecycle unification** — stops future drift.
3. **§3 missing funnel: Create-a-Dōjō → visibility → plan** + **§2 redaction preview** — the two load-bearing missing flows.
4. **S5 responsive contract** across the non-adapting screens (start with `saas` entry screens + maintainer `Candidate`).
5. **§2 make the theme promises operational** — preview dry-runs, precedence simulator, in-context inheritance, uniform provenance, post-action states.
6. **S7 state matrix** (empty/loading/error/offline) — with Relay offline/session-ended first-class.
7. **§4 trust bugs** + remaining **§3 screens** (SSO/SCIM, audit browser, second-approver queue, Revise, recall, logs, billing seat roster/meters/dunning).

---

*Source: three parallel screen reviews (SaaS+Relay+In-app · Governance+Admin+Maintainer · Lead+Developer+Billing+Extensions), 2026-07-15. Per-screen detail beyond this synthesis is available on request.*

---

# Round 2 — re-review of the updated mockups (2026-07-15)

**Verdict:** the designer resolved most of the **product-critical** backlog (§2 theme promises, §3 missing screens, §4 trust bugs) — strong, high-value progress — but the **priority-#1 systemic pass (S1 tokens / S4 type / S2 raw colors / S5 responsive) was essentially skipped.** Dark mode is still broken. Net clearly better, but **not yet build-ready** — one more focused pass is needed, and it's mostly mechanical.

## ✅ Resolved (the load-bearing gaps)
- **Redaction preview built** — `InappRedact` (raw *Travels upstream* vs *Dropped — never leaves*, sticky "Confirm & share anonymized"). The #1 missing screen. ✔
- **Create-a-Dōjō → visibility → plan funnel** — `DojoCreate` (hosting + who-joins + visibility→plan, live plan summary). The "single biggest gap". ✔
- **SSO/SCIM identity screen** — new `dojo-identity.jsx` (IdP presets · SCIM last-sync · git-access→role map · test-connection · read-only auto-provision cap). ✔
- **Relay** — first-class **offline / session-ended / daemon-asleep** states (`RELAY_CONN`), post-action acks (Approve→"session resumed", Decision→"answer sent"), **inbox detail now follows selection**, and a **logs screen**. ✔
- **Recall flow** — `InappTravel` status stepper (Queued→Triaged→Decided) + Recall on in-flight + decline reason + adopt credit. ✔
- **Shared primitive kit** — `dojo-shared` now hoists `DojoBtn` (with a `danger` variant), `DojoChip`, `DojoHead`, shell, mobile bar. The two missing tokens **`--accent-edge` + `--warning-edge` now exist**. ✔ (S6, S2-half)
- **Billing** — "active contributor" **defined** in-product + **seat roster** + **live Relay meters** (concurrency/inbox/presence) + **past-due/dunning** (danger family, correct) + wired into admin nav. ✔
- **Maintainer** — the fake `4 repos · 8 devs` reach bug is **gone** (→ "scope not sized yet"); **Revise editor** + **second-approver Approvals queue** (with empty state). ✔
- **Governance** — inherited rules **shown in-context** (greyed + "↑ scope"); the onboarding **double-count bug fixed** (Set dedup); stance-dial help lines. ✔
- **Admin** — a live **"which rule wins" verdict** panel; lifecycle vocabulary reconciled (active→deprecated→retracted); governance/billing/identity **dead routes wired**. ✔
- **Extensions** — fabricated round-robin approver **removed** (real `e.author` provenance) + empty state. ✔
- **Terminology** — largely converged on **"anonymize"** (saas, inapp, developer).

## ⚠️ Still open — the systemic pass (highest leverage, mostly skipped)
- **S1 · ~0% design-system adoption (unchanged).** Every screen — *including the new shared kit, `dojo-identity`, and `InappRedact`* — is hand-rolled inline `style` over the **deprecated numbered tokens** (`--paper-2/-3`, `--ink-2/-3/-4`, `--edge`, `--hairline`); no `zs-*` / semantic utilities. This is why dark mode stays broken. **Do this pass.**
- **S4 · off-scale type pervasive** (`9 / 10.5 / 11.5 / 12.5 / 13.5 / 14.5 / 26 / 34 / 42`) — now also baked into the shared primitives, so it propagates everywhere.
- **S2 · raw color literals persist** — amber `oklch(0.52 0.13 60)` in **admin (7×)**, **maintainer (6×)**, **inapp** (bind/share/travel/downstream); `color-mix` in **relay** (`DojoTag`, needs-you cards) + **billing** (enterprise dark tier); `rgba()` in **extensions**. The tokens exist now — use them.
- **S3 · danger-vs-warning still mis-routed** — **Retract** (admin), **Decline / Supersede** (maintainer), **declined** (inapp = amber, developer = grey) should all be `danger` (the variant exists, unused).
- **S5 · responsive half-done + inconsistent.** Fixed: saas sign-in, relay (fully), maintainer **Candidate** (sticky mobile action bar — the worst prior bug, resolved), `DojoDevTeams`, admin Overview. **Still fixed-grid (won't stack):** saas **orgs/create**, admin **Monitor/Scopes**, **governance**, lead **6-col audit ledger**, **billing tiers**, developer **contributions**, extensions **toolbar** — and the **new** `dojo-identity`, `InappRedact`, `DojoOrgsEmpty` shipped **without** the responsive contract.

## 🆕 New issues / regressions (introduced this round)
- **`site/tokens.css` NOT synced** — `--accent-edge` / `--warning-edge` were added to `lib/tokens.css` (4 refs) but **not `site/tokens.css` (0 refs)**. Any screen served from `site/` that uses them gets an **undefined var** → **live breakage**. Sync the two files.
- **Fabricated provenance reappeared** (the §4 anti-pattern, on *new* screens) — `DojoApprovals` round-robins first-approver names/times; Candidate hardcodes **"Sven K."** as suggested approver (should derive from `SCOPE_OWNERS`).
- **Flagship affordances are unwired stubs** — Revise **"Save revision"** discards edits; **"Preview recipients"**, **"Preview onboarding"**, and the **stance-dial consequence preview** have no action. The "never-blind / what-happens-next" promise is labeled but not operational.
- **`dojo-identity` has no nav route** — the shell's "Settings · SSO" item is disabled (`opacity 0.6`), so the otherwise-complete screen is unreachable (echoes the old `DojoBilling` dead-route bug).
- **Governance new-logic gaps** — overridden inherited rules are **filtered out, not marked** (a dev can't see a local rule shadows an inherited one; the ask was "overrides *marked*"); inheritance **ignores Stack scopes** (only walks `parent`).
- **Terminology residue** — lead still says **"strip" / "dereference"**; inapp keeps **"dereferencing"** (line ~231) and three synonyms coexist in one file (*anonymized / source dropped / stripped*).
- **Trust residue** — `InappShare` "Share 2 to Dōjō" count still hardcoded (won't track selection).
- **Primitive duplication residue** — `IdPanel` is a 3rd local `Panel`; relay keeps its own `MOBILE_TABS`/`Live` alongside the shared ones; per-kind `color-mix` maps re-appear in relay instead of `DojoChip`/`OriginChip`.

## Priority for the next (final) pass — before build
1. **Sync `site/tokens.css`** with the new edge tokens (live-breakage fix — do first).
2. **S1/S4/S2 migration** — semantic tokens + `zs-*` + scale type; delete every inline `oklch`/`rgba`/`color-mix`. Unblocks dark mode; highest leverage.
3. **S3** — route Retract / Decline / Supersede / declined to `danger`.
4. **Wire the stubs** — Revise-save, Preview-recipients, Preview-onboarding, stance consequence.
5. **De-fabricate provenance** — real approver data on Approvals + Candidate.
6. **Finish S5** — Monitor, Scopes, governance, lead ledger, billing tiers, developer contributions, extensions toolbar, + the 3 new screens; route `dojo-identity` into the nav.
7. **Terminology** — retire "strip"/"dereference" from lead + inapp; mark (don't hide) governance overrides; include Stack in inheritance.

*Source: three parallel re-reviews against this doc, 2026-07-15 (round 2).*
