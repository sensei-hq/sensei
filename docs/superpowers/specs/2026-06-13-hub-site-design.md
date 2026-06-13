# Design — Sensei HQ Hub + Sensei site, one combined website

> Date: 2026-06-13
> Status: approved (design), pending implementation
> Repo: `sensei-hq/sensei`, app: `website/` (SvelteKit + adapter-static, GitHub Pages)

## Problem / goal
We have a new **Sensei HQ** hub mockup — an independent-studio landing page that showcases the studio's tools (Sensei, DBD, Rokkit, Kavach + incubation). We want the hub to be the **root** of `sensei-hq.com`, with the existing Sensei marketing site living under **`/sensei`**, all in **one** SvelteKit app / one GitHub Pages deploy, reusing the existing rokkit + zen/sumi design system. Avoids a second repo/deploy and duplicated design-system config.

Source mockup: `docs/mockups/Sensei/hq/site.jsx` (+ `docs/mockups/Sensei/Sensei HQ.html`). Zen-sumi tokens: `docs/mockups/Sensei/site/tokens.css`.

## Decision (chosen: Option X — single combined site)
One Pages deploy serves the same files to both `sensei-hq.com` and (today) `sensei.sensei-hq.com`. A single static site can't serve different content per domain, so subdomain-split-from-one-deploy is impossible without a second deploy. Therefore:
- Hub at `/` (`sensei-hq.com`).
- Sensei product site under `/sensei` (`sensei-hq.com/sensei`).
- `sensei.sensei-hq.com` is **dropped** (DNS-side).
- Rejected: a separate `sensei-hq/hub` repo (Option Y) — would duplicate rokkit/sumi/uno config + a second deploy for a hub that mostly links out.

## Architecture — routing & layout
Restructure `website/src/routes`:
- **`+layout.svelte`** → slim *global* root only: `uno.css` + `app.css` imports, rokkit `themable`/`vibe` theming, default `<head>`. No surface-specific nav. Keep `+layout.ts` `prerender = true`.
- **`+page.svelte`** → **the Hub** (port of `hq/site.jsx`), self-contained with its own hub nav + footer.
- **`sensei/`** → the current Sensei site moved wholesale:
  - `sensei/+layout.svelte` — Sensei's own nav/footer shell.
  - `sensei/+page.svelte` — today's root landing.
  - `sensei/docs/`, `sensei/faq/` — moved from root.
- **Root-level (org), unchanged location:** `privacy/`, `terms/` stay at `/privacy`, `/terms` (org-level legal, linked from both hub and Sensei footers).
- Internal links resolved via `$app/paths` `base` (now `''`). Hub Sensei card → `/sensei`.

Two self-contained surfaces (hub, sensei) sharing a thin theming root.

## Design-system reconciliation
Keep the website's existing **rokkit + uno + `src/lib/tokens.css`** (`--paper`/`--sumi`) as canonical. Port the hub by:
- Extending `tokens.css` with the vars the hub adds: `--accent`, `--ink`/`--ink-soft` (alias/add), per-product accents (`--acc-sensei`/`--acc-dbd`/`--acc-rokkit`/`--acc-kavach`).
- Translating the mockup's `zs-btn`/`zs-card`/`text-ink`/`bg-paper-soft` classes to the site's rokkit/uno utility classes (or adding the few `zs-*` helpers to the site's CSS once).
- Use the **semantic-styles-rokkit** skill during build; follow the project mockup-porting rule (inline literal OKLCH from the mockup tokens; render the artboard before trusting the JSX).
- Dark mode via the existing rokkit theming (`vibe` + `themable`).
Result: one coherent token system serving both surfaces.

## Hub content & links (port of `site.jsx`)
Sections: Nav, Hero, Portfolio (products), Incubation, Approach, OpenSource, Footer.
- **Products** (cards): Sensei → **`/sensei`** (internal); DBD → **`https://dbd.sensei-hq.com`**; Rokkit → **`https://rokkit.sensei-hq.com`**; Kavach → **`https://kavach.sensei-hq.com`** — externals open in a new tab (`target=_blank` + `rel=noopener`). (Mockup placeholders `#dbd`/`#rokkit`/`#kavach` replaced with these subdomains.)
- **Incubation** (Magpie/Kata/Burn-E): follow the mockup's treatment (no live subdomains yet — repo link or non-linked card as the mockup has them).
- **OpenSource** section: GitHub repo links as in the mockup (`sensei-hq/rokkit`, `sensei-hq/kavach`, …).

## Deploy & domain
- `.github/workflows/deploy-website.yml`: **remove `BASE_PATH: /sensei`** (serve at root). One Pages deploy.
- Custom domain: **apex `sensei-hq.com`** (Pages CNAME). Add a `website/static/CNAME` with `sensei-hq.com` if not already present.
- **Drop `sensei.sensei-hq.com`** — remove the subdomain DNS record + from Pages config. **Manual/DNS step** (out-of-repo); called out in the plan, not automated.
- **Canonical domain = `.com`; `sensei-hq.org` 301-redirects to `sensei-hq.com`** (not served interchangeably — avoids duplicate-content SEO and GitHub Pages' one-custom-domain limit). Implemented as **registrar/DNS domain forwarding** (or a Cloudflare redirect rule), not in-repo. The repo `CNAME` and app stay `.com`-only.
- **Fresh release — no redirects.** Old top-level Sensei URLs (`/docs`, `/faq`) simply move to `/sensei/*`; no redirect stubs for old paths. `/privacy`, `/terms` remain at root.

## Testing
Playwright e2e (site already has Playwright + `tests/`):
- Hub renders at `/` with all sections present.
- Sensei product card links to `/sensei` (internal); DBD/Rokkit/Kavach cards have the correct subdomain `href` + `target=_blank`.
- `/sensei` landing renders; `/sensei/docs`, `/sensei/faq` render; `/privacy`, `/terms` render.
- Theme toggle persists (rokkit `themable`, storageKey).
- `bun run build` (prerender) succeeds with no missing-id/broken internal links.

## Non-goals / follow-ups
- The other product sites (dbd/rokkit/kavach subdomains) are separate properties — not built here; the hub only links to them.
- DNS changes (apex confirm, drop subdomain) are manual ops, listed for the user.
- No CMS/content model — hub copy is ported from the mockup as static content.
