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
bunx wrangler dev --port 5175 --local          # loads .dev.vars
# mint a magic link (local supabase service_role) → visit → /orgs → Enter → /console
```
Local `/v1` data routes may 404 (dev-only artifact) — pages degrade gracefully; that
is not a layout bug.

## Local dōjō for the daemon (`dojo_sync`)

The chain the daemon's `dojo_sync` needs, in order (#186):

1. **`make dojo-local`** (from the repo root). It refuses a busy `:5173`, runs `supabase start` if supabase is not
   already up, then runs `bun run dev` in the foreground.
   - The dev server binds **IPv4 `127.0.0.1:5173`** (`dev-server.ts`, #185). Plain `localhost` bound IPv6
     `::1` only, and the daemon dials IPv4, so `dojo_sync` logged "could not reach dōjō" on every tick.
2. **Enrol the daemon.** In Sensei.app go to Settings → Dōjō → Connection and sign in to `http://127.0.0.1:5173`.
   This writes the `sensei.dojo_memberships` row that `dojo_sync` dials.
3. **Connect a forge org to the tenant** so repositories map to it. The GitHub sign-in does this in production;
   locally, add the row by hand (local supabase only):
   ```sql
   insert into dojo.tenant_connections (tenant_id, provider, external_slug, connected_by)
   select t.id, 'github', 'sensei-hq', '<auth.users id>' from dojo.tenants t where t.key = '<tenant key>';
   ```

**Working looks like** this in `~/.sensei/logs/senseid.<date>.log`, within 60 s:
- `dojo_sync: cycle complete … mapped=N pushed=M`. Steps 1–2 alone give `mapped=0 unmapped=<all>`, and each
  repository logs `reason="no_connection"`, which means step 3 is missing.
- `could not reach dōjō` means step 1 is not running, or is on another address.
