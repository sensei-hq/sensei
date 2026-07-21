---
type: design
---

# Setup & config — module

Behind-the-scenes design for the [Setup](../features/01-setup.md) entry gate and
the [Configuration](../features/02-config.md) surface. The feature docs say what
the user sees and does; this says how it works and where the code lives.

## Health gate (bootstrap)

- Crate: `crates/bootstrap` (`health/` module).
- Probes six gates, in order: Homebrew (package manager), PostgreSQL, Ollama,
  sensei components (cli · mcp · daemon), the database, the daemon.
- States: `probing` → `auto-fixing` → `all-green`; or `manual` when Homebrew
  itself is missing (the one hand-run step).
- Auto-fix runs `brew bundle` against the manifest from the `sensei-hq` Homebrew
  tap. No input needed. Offline/proxy: honor `HTTPS_PROXY` or use the offline
  bundle.
- Runs on every launch; passes straight through when all gates are green.
- Mockup: [`bootstrap-splash.jsx`](../mockups/Sensei/lib/setup/bootstrap-splash.jsx).

## Installation

- Components installed from the Homebrew tap: cli, MCP server, daemon.
- `make install-service` overlays + code-signs the binaries into the brew prefix.
- Database: create the schema + pgvector index. The daemon connects to `sensei`
  on `:7744` with data dir `~/.sensei/`.
- Daemon started in the background (restarts with the machine).

## Folder scan

- App routes: `app/src/routes/(config)/setup/{roots,scan}`; client state
  `scanState`.
- Roots are recursive. The daemon walks each root, finds git repos, and extracts
  the code graph (files · symbols · docs).
- Progress streams over SSE (`/api/scan/events`); the same channel carries
  assistant-part registration events.
- The watcher keeps the graph current — incremental scan + reconcile.
- Mockup: [`setup-wizard.jsx`](../mockups/Sensei/lib/setup/setup-wizard.jsx)
  (folder + scan stages).

## Dōjō auto-discover (not built)

- Inspect each scanned repo's GitHub remote → classify personal vs org-owned →
  match the org against known dōjōs → surface a prompt.

## Configuration surface

- App: `app/src/routes/(observatory)/settings/*`, rail from `settings-nav.ts`
  (You · Sources · Reasoning · Extensions). Dōjō under
  `app/src/routes/(observatory)/dojo/{sharing,connections}`.
- Thin route pages delegate to shared components (e.g. `ProjectsSection`).
- Assignments are folded into Inference (`InferenceAssignmentsPanel`, a live
  role → model editor) — no separate screen. Providers hold cloud API keys,
  Keychain-backed.
- Mockups: [`setup-wizard.jsx`](../mockups/Sensei/lib/setup/setup-wizard.jsx),
  [`wiz-inference.jsx`](../mockups/Sensei/lib/setup/wiz-inference.jsx),
  [`inference-settings.jsx`](../mockups/Sensei/lib/setup/inference-settings.jsx),
  [`wiz-assignments.jsx`](../mockups/Sensei/lib/setup/wiz-assignments.jsx),
  [`collective-settings.jsx`](../mockups/Sensei/lib/observatory/collective-settings.jsx).
</content>
