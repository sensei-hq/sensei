# Website — sensei-site

## Overview

The marketing website is a SvelteKit static site in the `website/` directory. Package name: `sensei-site`. It presents the product, links to downloads, and hosts documentation, FAQ, privacy policy, and terms.

---

## Architecture

### Stack

- **Framework:** SvelteKit with Svelte 5 (runes mode enabled)
- **Adapter:** `@sveltejs/adapter-static` — fully pre-rendered, no server runtime
- **Styling:** UnoCSS + Rokkit design system (`@rokkit/core`, `@rokkit/themes`, `@rokkit/ui`, `@rokkit/states`, `@rokkit/icons`)
- **Visualization:** D3 (architecture diagram)
- **Testing:** Playwright E2E

### Routes

| Route | Purpose |
|-------|---------|
| `/` | Landing page — hero, how-it-works, gallery (5 screen mockups), philosophy, privacy summary, FAQ summary, pricing, support |
| `/docs` | Documentation |
| `/faq` | Full FAQ page |
| `/privacy` | Privacy policy |
| `/terms` | Terms of service |

### Landing page sections

The landing page follows a single-scroll narrative:

1. **Nav** — sticky with backdrop blur, logo (kanji + text), section links, theme toggle
2. **Hero** — watermark kanji, tagline, heading, platform-aware download button, mock screen
3. **Stats** — four metrics (0 external requests, <60MB memory, MCP protocol, free preview)
4. **What it is** — two-column prose explanation
5. **How it works** — three-step grid: Watch / Notice / Adopt (kanji-headed cards)
6. **Gallery** — five mock screens alternating layout (Today, Sessions, Insights, Memories, Instruments)
7. **Philosophy** — centered prose with watermark kanji
8. **Privacy** — three privacy commitments (local storage, no telemetry, easy to delete)
9. **Pricing** — free during preview, early adopter discount promise
10. **FAQ** — four-card grid with link to full FAQ
11. **Support** — Ko-fi donation link
12. **Footer** — product/legal/source columns, version number from `__APP_VERSION__`

### Rokkit design system

The site uses Rokkit for theming, state management (dark/light mode via `vibe.mode`), icons, and UI components. Color tokens follow the Japanese ink aesthetic — `shu` (vermillion) for accent, `sumi` (ink) for text, `kami` (paper) for surfaces. The kanji throughout the site are not decorative — each names a phase of practice.

### Platform detection

The download button detects the user's OS from `navigator.userAgent` and links to the appropriate artifact (`.dmg` for macOS, `.exe` for Windows, `.AppImage` for Linux). Downloads are served from the GitHub releases page.

---

## Architecture visualization

The landing page previously included a D3-based architecture diagram. This needs updating to match the current system topology described in the design docs:

- Desktop app (Tauri + SvelteKit) communicating with the daemon via HTTP
- Rust daemon (senseid) serving API, MCP, and hook ingestion
- PostgreSQL for persistent storage
- Ollama for local inference
- Hook scripts capturing assistant events
- Marketplace plugin providing commands, skills, agents

The diagram should reflect the component relationships from the monorepo layout: `app/`, `crates/` (senseid, cli, mcp, bootstrap, gateway), `website/`, `marketplace/`, `homebrew/`.

---

## Deployment

Static build output. No server-side rendering.

```bash
make website-build    # cd website && bun run build
make website-dev      # cd website && bun run dev (local development with Vite HMR)
```

The build produces a fully pre-rendered static site. `adapter-static` with `fallback: null` means every route must be pre-renderable — no dynamic server routes. `BASE_PATH` can be set via environment variable for non-root deployments.

Version is managed by `make bump v=X.Y.Z`, which updates the footer version string in `+page.svelte` and `package.json`.
