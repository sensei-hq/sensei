# sensei — monorepo

## Structure

| Directory     | Language          | Purpose                                       |
|---------------|-------------------|-----------------------------------------------|
| `app/`        | SvelteKit + Tauri | Desktop app (UI + native shell)               |
| `crates/`     | Rust              | All Rust crates — single unified workspace    |
| ├ `senseid`   |                   | HTTP daemon (API server)                      |
| ├ `cli`       |                   | sensei CLI (binary: sensei)                   |
| ├ `mcp`       |                   | MCP server                                    |
| ├ `bootstrap` |                   | Installer / prereq checker                    |
| └ `logger`    |                   | Structured logging crate                      |
| `dojo/`       | SvelteKit         | Dōjō web app (developer/maintainer/admin/lead consoles; kavach+Supabase) |
| `website/`    | SvelteKit         | Marketing website                             |
| `docs/`       | Markdown          | Design docs, mockups, DDL, backlog            |
| `homebrew/`   | Ruby              | Homebrew tap (subtree → sensei-hq/homebrew-tap) |
| `marketplace/`| JSON/Markdown     | Skills & plugins (subtree → sensei-hq/marketplace) |

**Sibling repos** (pulled in as dependencies, not in this tree):

- **[sensei-hq/gateway](https://github.com/sensei-hq/gateway)** — LLM routing library. `crates/senseid` consumes it as `gateway-embedded` git dep. Was previously `crates/gateway/`; moved out so it can release to crates.io independently.

## Version

`VERSION` at the repo root is the single source of truth.
Run `make bump v=patch|minor|major|X.Y.Z` to update all manifests, commit, tag, push, and sync subtrees.

## Common commands

```bash
# Build Rust crates
make crates              # cargo build --release for senseid + sensei-cli + sensei-mcp
make crates-debug        # debug variant (faster compile, same code path)
make crates-all          # full-coverage build: root workspace + sidecar workspace

# Install (one umbrella + two sub-parts)
make install             # full install: service binaries + desktop .app
make install-service     # overlay senseid/sensei/sensei-mcp into brew prefix + codesign
make install-app         # build desktop .app + cp to /Applications/
make install-debug       # service overlay with debug binaries (fast iteration)

# Run desktop app (dev — tauri dev with Vite HMR)
make app-dev

# Run all tests
make test
make test-fast           # no DB required (pre-commit hook)

# Bump version across all manifests + tag + push
make bump v=patch        # 0.2.13 → 0.2.14
make bump v=minor        # 0.2.13 → 0.3.0
make bump v=0.5.0        # explicit version
```

## Database

See `database/` for DDL. The daemon connects to `sensei` on port 7744 with
data dir `~/.sensei/`. There's no dev/prod mode any more — one binary, one
DB. For iterating on DDL without publishing a release tag, set
`SENSEI_DDL_DIR=/abs/path/to/database` before launching the daemon and
`SenseiConfig::db_schema_source()` will resolve to that local directory.

## Rules

- Always start with `docs/backlog.md`
- **Code navigation** — prefer the sensei MCP tools (`search`, `get_callers`/`get_callees`, `get_patterns`, `get_rules`, `get_project_conventions`, `get_project_summary`) over `grep`/`glob` for finding symbols and understanding structure: they use the indexed code graph and return richer, ranked results. Fall back to grep/glob only when a tool errors or returns empty, and say so.
- Create a todo list for complex tasks
- TDD — always use zero-errors-policy before starting work
- Commit and push when a logical chunk is complete
- Work in `develop` branch; merge to `main` when a feature is complete
- `homebrew/` and `marketplace/` are git subtrees — edit in-repo, sync with `make tap-push` / `make marketplace-push`

## Hard rules — never break without explicit user approval

**Don't repeat yourself (DRY). Reuse over duplication.**
Before writing any function, type, constant, or list: search the codebase for an existing implementation. If one exists in a shared crate or module, use it. Never duplicate logic that is already owned by a shared location. Three near-identical lines are a sign to refactor — not a reason to add a fourth.

**No intentional avoidance of shared code.**
If you think a shared crate, function, or pattern should not be used in a given place, stop and ask the user before writing code that works around it. Never document avoidance inline in source code comments (e.g. `// does not depend on X to avoid Y`, `// keep in sync with Z`). If the user approves an avoidance, record the decision in `docs/backlog.md` or an ADR, not in a code comment.

**No silent workarounds for architectural decisions.**
Every deliberate deviation from the established architecture (e.g. not using `sensei-bootstrap` from a crate that depends on it, duplicating a type that already exists in a shared crate, maintaining a separate hardcoded list that the daemon should own) must be raised with the user and documented before implementation.

**Write modular, reusable, fully testable code.**
Extract shared logic into functions or crates. Prefer pure functions with clear inputs/outputs over side-effecting code. Design for testability: avoid hard-coding paths, environment globals, or network calls that cannot be injected or mocked. If a function cannot be unit-tested, ask whether it should be refactored before writing it.

**Never fabricate data on a failure path. Return an error, not a plausible default.**
When a fallible read/lookup/parse/RPC fails, the service MUST surface the failure — propagate the error (a 4xx/5xx, an `Err`, or an explicit error state the UI renders) — and MUST NOT substitute a fabricated, dummy, placeholder, or fixture value that a caller can't distinguish from a real result. Forbidden on a failure/miss path:
- Minting a synthetic identity (a fake project/folder/session id like `format!("p-{name}")`, a made-up uuid, a placeholder record) when a lookup misses. A miss is `None`/404, never an invented row.
- `.unwrap_or_default()` / `.unwrap_or(<value>)` / `.ok()` / `catch { return fixture }` on a fallible DB/network call in a **read handler or loader**, which turns a failure into an empty-or-zero "success" (200 with `[]`/`0`) — indistinguishable from a genuinely-empty result. The daemon idiom is `.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?`; frontends surface an error state.
- Persisting a plausible-but-wrong value (a defaulted family/model/timestamp) when the real one is absent, instead of skipping or erroring.
- Returning a fixture/mock from a shipped code path on error (esp. money-, identity-, or governance-facing screens).
Honest-empty (`None`, `[]`, `0`) is correct ONLY when the data genuinely is empty — never as a mask for a failure. `#[cfg(test)]`/fixtures used *inside tests* are fine; fixtures reachable in production are not. Fail closed on safety/identity (see the #109 audit + `crates/senseid/src/resolution.rs`). Any deliberate exception must be raised with the user and documented in `docs/backlog.md`, never hidden in a fallback.
