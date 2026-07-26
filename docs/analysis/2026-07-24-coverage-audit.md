---
title: Coverage audit — dōjō critical paths + shared-component consolidation (#42)
date: 2026-07-24
status: audit + designer directives; one consolidation applied (getInitials)
relates: docs/mockups/DESIGN-BRIEF.md · dojo/src/lib/components/kit/ · docs/spec/MOCKUP-INDEX.md
trigger: the logout 404 (a designed-but-uncovered critical path) — what else is uncovered?
---

# Coverage audit — what's uncovered, what's duplicated

The logout bug (`<a href="/logout">` → 404, because logout is a client `signOut()` not a
route) was a **critical path nobody had designed**. This audit sweeps for the same class of
gap across the dōjō, plus the shared-component duplication Jerry flagged. Read-only
inventory by an Explore pass; directives below feed [`../mockups/DESIGN-BRIEF.md`](../mockups/DESIGN-BRIEF.md).

## A. Critical-path coverage gaps (the logout class)

Flows every app has but this one never designed. **None have a mockup or an
implementation** — they fall through to SvelteKit defaults (raw 404 / white error page):

| Flow | State | Why it matters |
|---|---|---|
| **404 / not-found** | MISSING | A mistyped `/org/typo/...` or a stale link shows the framework's bare 404, not a Zen-Sumi page with a way back. |
| **Error boundary** (`+error.svelte`) | MISSING | A failed `+page.ts` load (network, 500) bubbles to SvelteKit's default error page — off-brand, no retry. |
| **Permission-denied** | MISSING | Org screens role-gate the *nav*, but a direct URL to an admin-only section has no "you don't have access here" surface. |
| **Session-expired / re-auth** | MISSING | Token expiry mid-session silently bounces to `/signin` with no "your session ended" cue and no return-to-where-you-were. |
| **Rate-limit / quota (429)** | MISSING | No friendly surface for throttling. |

**Designed & implemented but never given a discrete spec** (fine, but note them so they're
not "rediscovered" as gaps): sign-in (`DojoSignIn.svelte`), logout (`LogoutButton.svelte`),
relay offline (`RelayOfflineBanner.svelte`), relay blocked/nudge (`RelayBlockedHome.svelte`).

**In good shape:** empty states — the shared **`kit/EmptyState.svelte`** (空 · "Still
listening.") is adopted by 7+ screens (`ScrMyDojos`, `ScrOrgHome`, `ScrApprovals`,
`ScrIncidents`, `ScrClientAudit`, `ScrProjects`, `ScrPlaceholder`). Jerry's "tell the
designer to include EmptyState" is **already done** — the directive becomes *keep using it,
don't hand-roll empties* (a few screens like `ScrYourWork`/`ScrRelayApprove` may not cover
the empty branch yet — verify).

## B. Shared-component duplication (the "organize, don't repeat" concern)

The kit is large (53 components) and mostly good. The duplication is in **screen-level
markup that re-implements the same shell instead of composing a kit primitive**:

| Pattern | Occurrences | Fix | Risk to extract |
|---|---|---|---|
| **`getInitials(name)`** avatar monogram | 3 (`TopBar` 40–48, `MobileShell` 32–40, `ChatThread` 12–20) — byte-identical | `kit/initials.ts` util | **Low → DONE this pass** |
| **Card shell** `bg-paper-soft border-paper-edge rounded-lg border` + inline `padding:16px` | ~23 across `ScrHealth/Identity/Engagements/Billing/RoleSurfaces/…` | `kit/Card.svelte` (`tone`, grid padding) | Medium (23 files) → designer |
| **List row** `flex items-center gap-4 border-b` + inline padding | 8+ | `kit/ListItem.svelte` (or lean on `ListSection`) | Medium → designer |
| **Section field-label** uppercase + `letter-spacing:0.18em` inline | 10+ | `kit/FieldLabel.svelte` (`size`,`tone`) | Low-Med → designer |
| **Icon + label meta pair** | 6+ | `kit/LabelWithIcon.svelte` | Low → designer |
| **`Eyebrow` vs `SectionHead` eyebrow slot** | redundant | collapse to one | Low → designer |

The big ones (Card ×23, ListItem ×8) touch many files and change rendered markup —
**spec them for the designer** (batch, browser-verify by computed style per DESIGN-BRIEF
B1), don't sweep them unattended. The `getInitials` extraction is pure + 3 files → applied
now as the proof-of-direction.

## C. Directives for DESIGN-BRIEF (append there)

1. **Cover the five missing critical paths** (A): design 404, `+error.svelte`,
   permission-denied, session-expired, rate-limit — each a calm Zen-Sumi surface with a
   way forward (back / retry / re-auth-and-return). Treat them as first-class screens, not
   framework fallbacks.
2. **EmptyState is the law for empties** — always compose `kit/EmptyState`; never hand-roll
   an empty branch. Audit the few screens that skip it.
3. **Consolidate the duplicated shells into named kit primitives** (B): `Card`, `ListItem`,
   `FieldLabel`, `LabelWithIcon`; retire `Eyebrow` into `SectionHead`. One component per
   pattern, data-driven props, no inline `padding`/`letter-spacing` literals (§ design
   system: 4px grid + named tokens).
4. **Shared logic goes to a util, not a copy** — `getInitials` is the template; any 2nd
   copy of a computation is a refactor signal.

## D. Applied this pass

- **`kit/initials.ts`** — extracted `getInitials(name)` (the 3-copy monogram logic) + unit
  tests; `TopBar`, `MobileShell`, `ChatThread` now import it. Proof-of-direction for
  directive C4; the rest is speced for the designer batch.
