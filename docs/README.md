# Sensei documentation

> sensei is the OS for AI-assisted work: it **observes** the pair (you + your AI
> assistant), **learns** from what happens, and helps you **improve**. The
> north-star is **FTR** — first-turn resolution.
>
> **Structure (reorg 2026-10-13, [`reorg.md`](reorg.md)):** what we're building
> (`prd/`), where we are (`roadmap/`), how it's built (`architecture/`), what it
> looks like (`mockups/`), and the few superseded docs live docs still cite (`history/`).

## The four doors

| Door | Folder | Answers | Start here if you… |
|---|---|---|---|
| **What** | [`prd/`](prd/README.md) | what we are building and why — vision · objectives · personas · 29 module specs | are new, or want the product intent |
| **Where** | [`roadmap/`](roadmap/README.md) | is it built? how far? what's next? — per-module status + gaps | want the current truth |
| **How** | [`architecture/`](architecture/README.md) | implementation per surface | are building or reviewing code |
| **Seen** | [`mockups/`](mockups/) · [`journeys/`](journeys/README.md) · [`design/`](design/README.md) | what it looks like, the paths through it, the design system | are designing or validating UX |

**Precedence** ([`prd/README.md`](prd/README.md) §Precedence): `prd` > `roadmap`
> `architecture` > `mockups`. If two disagree, the higher one wins and the lower
one is the bug.

## Top-level files

- [`backlog.md`](backlog.md) — **open work, as live GitHub issue references.** Start here for what's being worked.
- [`decisions.md`](decisions.md) — the decision log.
- [`reorg.md`](reorg.md) — the 2026-10-13 reorg proposal (agreed): target tree, gap register, migration phasing.
- [`history/`](history/README.md) — the few superseded docs live docs still cite; everything else was deleted on 2026-10-21 and lives in git history.

## The module map in one line

| Plane | Modules | PRD | Roadmap |
|---|---|---|---|
| Foundation | M1–M6 | [`prd/modules/01-foundation`](prd/modules/01-foundation/README.md) | [`roadmap/01-foundation.md`](roadmap/01-foundation.md) |
| Insight | M7–M13 | [`prd/modules/02-insight`](prd/modules/02-insight/README.md) | [`roadmap/02-insight.md`](roadmap/02-insight.md) |
| Delivery | M14–M22 | [`prd/modules/03-delivery`](prd/modules/03-delivery/README.md) | [`roadmap/03-delivery.md`](roadmap/03-delivery.md) |
| Team | T1–T4 | [`prd/modules/04-team`](prd/modules/04-team/README.md) | [`roadmap/04-team.md`](roadmap/04-team.md) |
| Platform | P1–P3 | [`prd/modules/05-platform`](prd/modules/05-platform/README.md) | [`roadmap/05-platform.md`](roadmap/05-platform.md) |

## Monorepo structure

| Directory | Language | Purpose |
|---|---|---|
| `app/` | SvelteKit + Tauri | Desktop app (observatory + project window) |
| `crates/` | Rust | `senseid` · `cli` · `mcp` · `bootstrap` · `dojo-protocol` · `logger` |
| `dojo/` | SvelteKit | Dōjō web app — developer · maintainer · admin · lead consoles (SSO-gated) |
| `website/` | SvelteKit | Marketing site — **P2 module**, claims must trace to the roadmap |
| `database/` | SQL (dbd) | DDL definitions — one DB `sensei`, port 7744 |
| `homebrew/` | Ruby | Homebrew tap (subtree → `sensei-hq/homebrew-tap`) |
| `marketplace/` | Markdown | Skills · commands · plugins · agents (subtree → `sensei-hq/marketplace`) |
| `docs/` | Markdown | This documentation |

> **Sibling repo:** the LLM router is `sensei-hq/gateway`, consumed as the
> `gateway-embedded` git dependency (formerly the in-tree `crates/gateway/`).