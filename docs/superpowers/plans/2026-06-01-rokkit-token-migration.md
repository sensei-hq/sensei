# Rokkit 1.0.6 token migration — color system + typography

**Status:** plan only, not started
**Author:** Claude (handoff)
**Date:** 2026-06-01
**Branch target:** `develop`
**Estimated scope:** ~34 files (CSS/Svelte) for raw vars + ~70 files for utility classes; two focused commits + one cleanup commit

## Why

The current app is pinned to `@rokkit/* ^1.0.5`, which only emits the **z-scale** vocabulary (`--color-surface-z0..z10`, `--color-primary-z5`, etc.). Every border/background/text in the app reaches for that scale through wordy `oklch(var(--color-surface-z3) / 1)` expressions, and the mockups use a **different** flatter vocabulary (`--paper`, `--paper-soft`, `--paper-mute`, `--paper-edge`, `--ink`, `--ink-mute`, `--ink-soft`, `--ink-faint`). This forces every component port to translate by hand, and translation drift produced the 2026-06-01 AssistantCard rendering gap (see `feedback_mockup_porting.md`).

Rokkit `1.0.6` (already published in `~/Developer/rokkit`) emits **both** vocabularies side-by-side: the named/flat tokens AS WELL AS the z-scale aliases. Migrating in two passes — first adopt 1.0.6 and use the flat tokens for new work, then convert existing call sites — lets us match mockups verbatim without breaking anything.

## Where 1.0.6 lands

The new `@rokkit/themes/base.css` emits:

| Group | Tokens |
|---|---|
| Surface (named) | `--paper`, `--paper-soft`, `--paper-mute`, `--paper-edge` |
| Ink (named) | `--ink`, `--ink-mute`, `--ink-soft`, `--ink-faint` |
| Brand (named) | `--primary`, `--on-primary`, `--accent`, `--accent-soft`, `--success`, `--success-soft`, `--warning`, `--warning-soft`, `--danger`, `--danger-soft`, `--info`, `--info-soft` |
| Misc | `--focus-ring`, `--shadow-tint` |
| Z-scale (alias) | `--color-surface-z0..z10`, `--color-ink-z0..z10` (light) and auto-inverted in `[data-mode="dark"]` |
| Typography | `--font-display`, `--font-sans`, `--font-mono` (driven by `rokkit.config.js typography:` block) |

This means after the version bump, **nothing breaks** — the z-scale aliases stay, just with `--paper` etc. now also available.

## Phase 1 — version bump + config (one commit)

**Goal:** Sensei consumes 1.0.6, the new named tokens become available, app continues to function unchanged.

### 1.1 `package.json`
Bump every `@rokkit/*` dep from `^1.0.5` → `^1.0.6`. Keep the `@rokkit/ui@1.0.5` patch (`patchedDependencies`) — verify it still applies cleanly against 1.0.6; if not, regenerate or remove the patch.

### 1.2 `rokkit.config.js`
Update typography keys to match the 1.0.6 vocabulary:

```diff
  typography: {
-   sans: "'Inter Variable', ...",
-   mono: "'JetBrains Mono', ...",
-   heading: "'Fraunces', ...",
+   sans:    "'Inter Variable', ...",
+   mono:    "'JetBrains Mono', ...",
+   display: "'Fraunces', ...",          # heading → display
    kanji: "'Yu Mincho', ...",
  },
```

The preset emits `--font-display` (canonical name) plus `--font-heading` and `--font-ui` as back-compat aliases. No need to maintain a separate manual block in `src/lib/tokens.css`.

### 1.3 `src/lib/tokens.css`
Delete the manual `--font-*` declarations — they collide with Rokkit's emission:

```diff
- :root {
-   --font-display: 'Fraunces', ...;
-   --font-ui:      'Inter Variable', ...;
-   --font-mono:    'JetBrains Mono', ...;
-   --hairline: 1px solid oklch(var(--color-surface-z9) / 0.08);
- }
+ :root {
+   --hairline: 1px solid var(--paper-edge);  # now using flat token
+ }
```

`--font-ui` ⇒ Rokkit emits `--font-sans` canonically with `--font-ui` aliased; the few call sites that read `var(--font-ui)` keep working.

### 1.4 Smoke
- `bun install` (verify lockfile resolves)
- `bun run check` (svelte-check must remain at zero errors)
- `bunx vitest run` (510 must stay green)
- **`bun run build`** (catches UnoCSS preset preflight errors — e.g.
  using a reserved named token in `rokkit.config.js custom` block.
  Neither check nor vitest exercises the preset; only build does)
- `make app-dev` — open the app, spot-check 3 stages (welcome, assistants, projects); no visual regression expected

**Acceptance:** `--paper`, `--ink`, etc. resolve to non-empty values at runtime; existing z-scale call sites unchanged; all gates green.

---

## Phase 2 — color system migration (one commit per surface area, gated)

**Goal:** Convert every raw `oklch(var(--color-*-z*) / α)` and `oklch(var(--color-*-z*) / 1)` to its flat-token equivalent. Keep the alpha-based call sites where the design legitimately needs translucency (use `color-mix(in oklch, var(--paper-soft) X%, transparent)`).

### Mapping table

| Old (z-scale) | New (named) | Notes |
|---|---|---|
| `oklch(var(--color-surface-z0) / 1)` | `var(--paper)` | Page background |
| `oklch(var(--color-surface-z1) / 1)` | `var(--paper-soft)` | Card surface |
| `oklch(var(--color-surface-z2) / 1)` | `var(--paper-mute)` | Recessed surface, button hover |
| `oklch(var(--color-surface-z3) / 1)` | `var(--paper-edge)` | Hairline borders |
| `oklch(var(--color-surface-z6) / 1)` | `var(--ink-soft)` | Secondary text |
| `oklch(var(--color-surface-z7) / 1)` | `var(--ink-mute)` | Body text |
| `oklch(var(--color-surface-z9) / 1)` | `var(--ink)` | Headings |
| `oklch(var(--color-surface-z9) / 0.08)` | `var(--paper-edge)` | Drop the alpha; the named token bakes the right tone for both modes |
| `oklch(var(--color-primary-z5) / 1)` | `var(--primary)` | Brand fill |
| `oklch(var(--color-primary-z6) / 1)` | `var(--accent)` (if accent === primary, same value) | Highlight text |
| `oklch(var(--color-success-z6) / 1)` | `var(--success)` | Success text |
| `oklch(var(--color-success-z6) / 0.10)` | `color-mix(in oklch, var(--success) 10%, transparent)` | Soft tints — explicit |
| `oklch(var(--color-danger-z5) / 1)` | `var(--danger)` | Danger text |
| `oklch(var(--color-danger-z5) / 0.10)` | `color-mix(in oklch, var(--danger) 10%, transparent)` | Soft tints — explicit |

### Audit numbers (today, 2026-06-01)

- **126 occurrences** of `oklch(var(--color-*-z*) / *)` across **34 files** (raw CSS in `<style>` blocks)
- **541 occurrences** of `*-z*` utility classes (e.g. `bg-surface-z1`, `text-ink-z6`) across ~70 Svelte files

The raw-CSS conversions are the high-value cleanup; the utility classes still resolve correctly through the alias layer in 1.0.6, so they can be migrated lazily.

### Sequencing (smallest blast radius first)

1. **Shared primitives** — `src/lib/tokens.css`, `src/lib/components/{StatusDot, Eyebrow, Kanji, Switch, PageHeader, TabBar, MemoryList, AssistantCard, AssistantBrandMark}.svelte`
2. **Health surfaces** — `src/routes/(health)/**` (Remedy, Hero, Ledger, upgrade, logs)
3. **Setup wizard** — `src/routes/(config)/**` (preferences, roots, scan, inference, assistants, libraries, instruments, done)
4. **Observatory** — `src/routes/(observatory)/**` (settings, sessions, projects, libraries, instruments, help)
5. **Project window** — `src/routes/(project)/**` (per-section pages)

Each step:
- Convert raw `oklch(var(--color-*-z*))` to flat tokens via the mapping table
- Re-run `bun run check` + `bunx vitest run` after each surface area
- Live-smoke the affected stage via `make app-dev`
- Commit per surface area (5 commits in this phase)

### Utility-class migration (deferred)

Rokkit 1.0.6's preset may add **named-token utilities** (`bg-paper`, `text-ink-soft`, `border-paper-edge`). If it does — verify via `bunx unocss --watch` on a scratch file — convert utility-class usage in a separate cleanup pass after Phase 2. If 1.0.6 doesn't ship those utilities, leave the 541 z-scale class usages in place; they continue to work through the alias.

---

## Phase 3 — typography conformance (folds into Phase 2 commits)

**Goal:** Every component reads font via the canonical `--font-*` names; no inline font stacks.

### Steps

1. Search for any inline `font-family: 'Fraunces'`, `font-family: 'Inter'`, `font-family: 'JetBrains Mono'` — replace with `var(--font-display)`, `var(--font-sans)`, `var(--font-mono)`.
2. Replace stale `var(--font-ui)` references with `var(--font-sans)` (preset emits both as aliases, but `--font-sans` is canonical in 1.0.6).
3. Replace stale `var(--font-heading)` references with `var(--font-display)`.
4. `var(--font-kanji)` stays — it's a Sensei-specific addition not in the Rokkit base.

### Audit numbers

- 12 occurrences of `var(--font-*)` across 3 files (AssistantBrandMark.svelte, AssistantCard.svelte, logs/+page.svelte)
- The `:root { --font-* }` block in `src/lib/tokens.css` is the **only declaration site**; once Rokkit's preset emits these (Phase 1.3), the manual block is dead code.

This phase is small enough to fold into Phase 2's per-surface-area commits — each commit fixes the colors AND the font references in the same files.

---

## Risks & open questions

1. **`@rokkit/ui@1.0.5` patch** — `app/patches/@rokkit%2Fui@1.0.5.patch` exists for the Switch shim discovery (W2/W8 from the wizard rehab). Verify it applies to 1.0.6 cleanly before bumping; if 1.0.6 already fixes the underlying issue, drop the patch.

2. **Theme switching** — confirm that 1.0.6's `[data-mode="dark"]` selector matches the app's `body[data-mode="dark"]` toggle (set by `app.html`). If the preset emits `[data-theme="dark"]` instead, the dark-mode swap won't trigger and we'll need a small CSS shim or a script change.

3. **`--paper-edge` in dark mode** — the rokkit/learn `koan/compat.css` explicitly overrides this in dark mode to a darker (lower-lightness) tone because the default sumi-400 is LIGHTER than paper, giving a "raised edge" that the design rejects. We'll need the same override in Sensei's `src/lib/tokens.css`. Easy fix, but worth noting because it's the exact same gotcha we just hit in AssistantCard.

4. **`@rokkit/themes` import order** — the demo imports base + 4 theme variants + zen-sumi; Sensei probably only needs base + zen-sumi. Verify nothing else (rokkit, minimal, material, frosted) is referenced; remove unused imports from `src/app.css`.

5. **Mockup file as a source of truth** — once Phase 1 is in, `docs/mockups/lib/tokens.css` and the app's `tokens.css` should resolve to the same color values. If they drift again (e.g. mockup uses `--paper`-edge variant we don't emit), update the mockup OR add the missing token, but never silently substitute.

6. **The AssistantCard's local `--ac-*` block** — added today (`f6eeca1e`) as a defensive measure to anchor the card to mockup-literal OKLCH. After Phase 1 + Phase 2.1 (shared primitives) ship, that block becomes redundant — the component can drop the local vars and read `var(--paper-soft)` etc. directly. List it as a cleanup item in the Phase 2.1 commit.

---

## Acceptance for the whole migration

- `bun run check` zero errors
- `bunx vitest run` 510+ tests green
- `make test-app-e2e` running the assistants suite: 6/6 pass
- Live smoke: each (config), (observatory), (project), (health) layout renders without visual regression — borders subtle, type hierarchy intact
- `grep -r 'oklch(var(--color-.*-z' src/` returns **zero** raw call sites (all converted to flat tokens)
- The manual `:root { --font-* }` block in `src/lib/tokens.css` is gone
- `--hairline` is defined once (in `tokens.css`) as `1px solid var(--paper-edge)` — no inline duplicates
- `AssistantCard.svelte` no longer carries its `--ac-*` shim block

---

## Out of scope for this plan

- Density tokens (`--space-1..9`) — Rokkit emits these; current app uses Uno's `gap-N` utilities which are independent
- Icon system — `@rokkit/icons` integration is a separate concern (already discussed for AssistantBrandMark, deferred)
- Component-level Rokkit primitives (Switch, Buttons, Forms) — those are component migrations, not token migrations; queued separately

---

## Suggested first commit (after your approval)

`chore(deps): bump @rokkit/* 1.0.5 → 1.0.6, switch typography to display key`

— covers Phase 1 only, gives you a clean review surface to confirm nothing visually drifts before the bigger Phase 2 conversion starts.
