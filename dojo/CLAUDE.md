# Dōjō app — conventions (auto-loaded)

SvelteKit + UnoCSS (`@rokkit/unocss` `presetRokkit`) + Supabase, deployed as a
Cloudflare Worker at dojo.sensei-hq.com.

## Styling — follow the shared design system

**Canonical rules: [`../docs/architecture/frontend-svelte-guidelines.md`](../docs/architecture/frontend-svelte-guidelines.md)** —
the same system the desktop `app/` follows. In particular: canonical named tokens
(never hex/`oklch`/`var()` in components), the **8-stop type scale** (`text-xs`…`text-4xl`
— **never `style="font-size:…"`**), the 4px spacing grid (`p-*`/`gap-*`, never literal
px), radii, **responsive `md:` prefixes** (§1.7, never `@media` in `<style>` for layout;
mobile-first, `<md` phone / `md:+` desktop), and **per-surface config parity** (§1.8).

**Dōjō-specific debt to fix (§1.8):** `uno.config.js` here is bare (`presetRokkit`
only) — it lacks the `theme.fontSize`/spacing/radius block that `app/uno.config.js`
has, so `text-sm` resolves to UnoCSS's default 14px (not the design's 13px), which is
why ~25 files hand-code `font-size`. **Adopt `app/uno.config.js`'s `theme` block
verbatim**, then convert inline sizes to `text-*` utilities opportunistically as you
touch files. Never add new inline `font-size`/`@media`/color literals.

## Dev gate (zero-errors before commit)

- `bun run check` — svelte-check (types + a11y). Must be 0 errors / 0 warnings.
- `bun run test` — vitest + @testing-library. This app has **no Playwright** in-repo;
  house pattern = unit-test `-data`/`-view` modules + component render specs, then
  browser-verify visual/responsive work with the local wrangler recipe below.
- **Every `.svelte` edit** validated with the Svelte MCP autofixer
  (`mcp__plugin_svelte_svelte__svelte-autofixer`) before commit.
- Virtual modules aren't generated under vitest — stub them in `vitest.config.ts`
  (`$app/paths`, `$env/dynamic/public` are stubbed in `src/lib/test-stubs/`).

## Local browser-verify recipe (auth'd console)

```bash
CF_PAGES=1 bun run build
bunx wrangler dev --port 5173 --local          # loads .dev.vars
# mint a magic link (local supabase service_role) → visit → /orgs → Enter → /console
```
Local `/v1` data routes may 404 (dev-only artifact) — pages degrade gracefully; that
is not a layout bug.
