# sensei/app

SvelteKit + Tauri desktop app. Frontend.

## Read before working in this folder

**Canonical rules: `sensei/docs/architecture/frontend-svelte-guidelines.md`** — token
system, the 8-stop type scale (never literal px), spacing grid, **responsive `md:`
prefixes (§1.7)**, per-surface config parity (§1.8), state separation, component
patterns, voice, testing. Shared across app/ · dojo/ · website/. Every new screen and
every refactor follows it.

Companion references:
- Mockups (visual source of truth): `sensei/docs/mockups/Sensei/screenshots/` — 61
  PNGs, the current reference. Only one prototype `.jsx` survives
  (`lib/project/project-metrics.jsx`); read the screenshots, not the JSX.
- Design system: `sensei/docs/mockups/Zen-Sumi Design System/`
- Rokkit preset: `node_modules/@rokkit/unocss/README.md`

## Non-negotiable summary

1. **Tokens** — canonical 24 named tokens only (`paper`, `paper-soft`,
   `paper-mute`, `paper-edge`, `ink`, `ink-mute`, `ink-soft`, `ink-faint`,
   `primary`, `on-primary`, `accent`, `accent-soft`, `success`/`-soft`,
   `warning`/`-soft`, `danger`/`-soft`, `error`/`-soft`, `info`/`-soft`,
   `focus-ring`, `shadow-tint`). No z-scale. No OKLCH/hex in components.
   No `<style>` color blocks. Tune values via `rokkit.config.js` `overrides:`
   — never via `shortcuts:` or `rules:`.

2. **State** — `*.svelte.ts` files own derivations, status mapping, copy,
   and actions. Components are pure templates: props in, markup out.

3. **Data** — `+page.ts` / `+page.server.ts` `load()` for pre-fetched data.
   Client-side state only when data must come from runtime.

4. **API contracts win** — type primitive props against the wire enum from
   `*-types.ts`, not against the mockup's prototype values.

5. **DRY** — 2+ uses becomes a component or snippet. Shared primitives in
   `src/lib/components/`; screen-local primitives co-located until a second
   consumer appears.

6. **Mockup fidelity** — when current code disagrees with the mockup, rebuild
   against the mockup. When the mockup disagrees with the wire API, defer to
   the wire.

7. **Voice** — no exclamations, no emoji, sentence case, lowercase "sensei",
   no marketing speak. See guidelines §4.

8. **Testing** —
   - `Foo.harness.svelte` + `Foo.spec.svelte.ts` per component (existing
     pattern — see `Eyebrow.harness.svelte`). Harnesses are mounted in tests,
     not exposed as routes.
   - Vitest snapshots: checked in (`__snapshots__/`).
   - State specs: `*-state.spec.svelte.ts`, checked in.
   - Playwright behaviour (e2e): `e2e/tests/*.spec.ts`, CI gate.
   - No checked-in PNGs. Visual baselines are not the regression strategy —
     eyeball against the mockup HTML in dev.
