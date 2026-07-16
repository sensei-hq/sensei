# Dōjō mockup — outstanding fixes (for the designer)

> The 11 `Sensei/lib/dojo/*.jsx` screens (+ new `dojo-identity`), judged against the
> Dōjō journey map and `Sensei/CLAUDE.md`. The **product-critical** gaps are
> resolved (redaction preview, create-Dōjō→plan funnel, SSO/SCIM screen, Relay
> offline/session-ended states, recall, billing seat-roster/meters/dunning,
> maintainer reach trust-bug, governance in-context inheritance). **This lists only
> what still needs fixing before build.** Each item = issue + concrete fix. Priority
> order at the bottom.
>
> *(Kept at `docs/mockups/` — outside `Sensei/` — so it survives replacing the whole
> `Sensei/` mockup folder.)*

## 0 · Urgent — live breakage
- **`site/tokens.css` not synced.** `--accent-edge` / `--warning-edge` were added to
  `lib/tokens.css` (4 refs) but **not** `site/tokens.css` (0 refs). Any screen served
  from `site/` referencing them gets an **undefined var**. **Sync the two files first.**

## 1 · Design-system migration (S1 · S4 · S2) — highest leverage; dark mode is broken
- **S1 · ~0% adoption.** Every screen — incl. the new shared kit, `dojo-identity`,
  and `InappRedact` — is hand-rolled inline `style` over **deprecated numbered
  tokens** (`--paper-2/-3`, `--ink-2/-3/-4`, `--edge`, `--hairline`). Migrate to
  semantic utilities (`bg-paper-soft`, `text-ink-mute`, `border-paper-edge`) + `zs-*`
  components (`zs-btn`, `zs-card`, `zs-input`, `zs-badge`), per `lib/assistant-card.jsx`.
- **S4 · off-scale type** everywhere (`9 / 10.5 / 11.5 / 12.5 / 13.5 / 14.5 / 26 / 34 / 42`)
  — now baked into the shared primitives too. Snap to the scale
  (11/13/15/17/22/28/40) or named type classes.
- **S2 · raw color literals persist** — amber `oklch(0.52 0.13 60)` in **admin (7×)**,
  **maintainer (6×)**, **inapp** (bind / share / travel / downstream);
  `color-mix(in oklch…)` in **relay** (`DojoTag`, needs-you cards) + **billing**
  (enterprise dark tier); `rgba()` in **extensions**. The tokens now exist — use
  `--warning` / `--warning-edge` / `--danger*`. Delete every inline `oklch`/`rgba`/`color-mix`.

## 2 · Danger-vs-warning (S3)
- Route **Retract** (admin), **Decline** + **Supersede** (maintainer), and
  **declined** (inapp = amber, developer = grey) to the **`danger`** family (the
  `DojoBtn danger` variant now exists). Keep amber only for caution/inferred/flagged.

## 3 · Responsive (S5) — half-done
- Still fixed-grid / won't stack on mobile: saas **orgs** + **create**, admin
  **Monitor** + **Scopes**, **governance**, lead **6-col audit ledger**, **billing
  tiers**, developer **contributions**, extensions **toolbar** — and the *new*
  `dojo-identity`, `InappRedact`, `DojoOrgsEmpty` (shipped without the responsive
  contract). Thread `mobile`, stack columns, card-per-row for the ledger.

## 4 · Unwired stubs (the "never-blind / what-happens-next" promise)
- Revise **"Save revision"** discards edits (calls `setRevising(false)` only);
  **"Preview recipients"**, **"Preview onboarding"**, and the **stance-dial
  consequence preview** have no action. Wire them (or show the real preview modal/state).
- **Stall** (relay) has no acknowledged post-action state (Approve/Decision do).

## 5 · Fabricated provenance (reappeared — the §4 anti-pattern)
- `DojoApprovals` round-robins first-approver names + times
  (`["Keiko T.","Marco D."][i%2]`); Candidate hardcodes **"Sven K."** as the
  suggested approver. Derive from real data (`SCOPE_OWNERS`) — don't fabricate on a
  trust surface.

## 6 · Routing · terminology · logic gaps
- **`dojo-identity` has no nav route** — the shell's "Settings · SSO" item is
  disabled (`opacity 0.6`), so the finished screen is unreachable. Route to it.
- **Terminology residue** — **lead** still says "strip" / "dereference"; **inapp**
  keeps "dereferencing" and three synonyms coexist (*anonymized / source dropped /
  stripped*). Retire everything to **"anonymize"**.
- **Governance** — overridden inherited rules are **hidden, not marked** (show them
  struck-through / "overridden"); inheritance **ignores Stack scopes** (only walks
  `parent`, so a project never inherits its Stack governance).
- **`InappShare` "Share N" count** is hardcoded — track the checkbox selection.

## 7 · Primitive duplication residue (S6)
- `IdPanel` is a **3rd** local `Panel`; relay keeps its own `MOBILE_TABS` / `Live`
  alongside the shared ones; per-kind `color-mix` maps re-appear in relay instead of
  `DojoChip` / `OriginChip`. Consolidate into `dojo-shared`.

## Priority order
1. **Sync `site/tokens.css`** (§0) — live-breakage fix, do first.
2. **S1 / S4 / S2** token + utility + type migration (§1) — unblocks dark mode; highest leverage.
3. **Danger routing** (§2).
4. **Wire the stubs** (§4) + **de-fabricate provenance** (§5).
5. **Finish responsive** (§3) + **route `dojo-identity`** into the nav (§6).
6. **Terminology** + governance **override marking** / **Stack inheritance** + `InappShare` count (§6).
7. **Primitive consolidation** (§7).

*Updated 2026-07-15 (round 2) — resolved items removed; open items only.*
