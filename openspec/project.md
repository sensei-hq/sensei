# Project Context

## Purpose
sensei is an operating system for AI-assisted software work: a local daemon (`senseid`) indexes code, captures
assistant sessions, computes delivery and quality metrics, and brokers **Relay** — work handed between assistant
sessions (lanes) in a crew, run by the person from a desktop app and the dōjō (team web app).

## Tech Stack
- Rust workspace (`crates/`): `senseid` (axum HTTP daemon, PostgreSQL via sqlx), `cli`, `mcp`, `bootstrap`, `logger`.
- `app/`: SvelteKit + Svelte 5 + Tauri desktop app. `dojo/`: SvelteKit web app (Supabase, kavach auth).
- Shared TS packages: `packages/state` (`@sensei/state`: domain states, selectors, clients), `packages/ui`.
- UI: rokkit (`@rokkit/*`, our own component library — fix rokkit, never patch per screen) + UnoCSS
  (`presetRokkit`, named tokens; responsive `sm:`/`md:` variants, mobile-first).
- Database DDL: `database/` managed with dbd (file is source of truth; `dbd reconcile` pre-release).

## Project Conventions

### Code Style
- Plain, specific names; comments say why. Voice in UI copy: no exclamations, sentence case, lowercase "sensei".
- Tokens only (`paper`, `ink`, `accent`, …); no hex/OKLCH or `<style>` colour blocks in components.
- Never fabricate data on a failure path: a failed read is an error state, never an empty or default (D83).

### Architecture Patterns
- Component → State → Load: components are pure templates; `*.svelte.ts` / `@sensei/state` own derivations and
  actions; loaders read. Selectors are pure and unit-tested.
- Relay screens read `GET /relay/now` + the relay event stream; rows arrive on the stream, ids re-read.
- One shell (D87 `ShellScreen` on rokkit `ScreenShell`); shared components over local copies (catalogue spec
  `docs/spec/2026-11-14-shared-component-catalogue.md`).

### Testing Strategy
- TDD: a failing test is committed before the code that makes it pass (red commit, then green).
- Vitest unit/component specs (`app`, `packages/state`, `dojo`); Rust unit + DB-backed handler tests (serial DB);
  Playwright Tauri e2e (`app/e2e`), incl. the relay run journey; `/preview` bench + capture matrix
  (light/dark × wide/phone × kanji/icon) for visual checks.
- Pre-commit hook runs the fast suite; zero lint/test errors before and after work.

### Git Workflow
- Work on `develop` (or a lane branch merged to develop by PR, green). Releases ship from `main` by version tag
  after a develop → main PR (`make bump`, `make release`).
- Commit messages: `type(scope): what and why`; a completing commit names `Closes #N`.

## Domain Context
- Relay: a **crew** of **lanes** (assistant sessions: coordinator + contributors) works a **run** (sprint) of
  **tasks** from the person's **backlog**; tasks are claimed, checkpointed (steps), reported, reviewed, and merged
  by the broker's gate. Task 0 plans the run. Decisions ask the person; nudges chase waits.
- Lanes carry a codename (`suggested_name`), a family/model (the assistant), and presence (active/idle/not responding).

## Important Constraints
- Desktop-first, but every screen must work at phone width (≈430px) — mobile-first UnoCSS breakpoints.
- No transcripts in any relay field; text fields capped at 600 chars.
- A senseid reinstall re-prompts the Keychain "Always Allow" (#283).

## External Dependencies
- GitHub (issues, releases, Actions), Supabase (dōjō), Cloudflare (website/worker), Apple notarization,
  Homebrew tap; trackers (Jira, Azure DevOps, GitHub, GitLab) for ticket sync.
