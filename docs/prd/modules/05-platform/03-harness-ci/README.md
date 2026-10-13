# Harness & CI · P3

**Plane:** platform · **Status:** [roadmap/05-platform.md](../../../../roadmap/05-platform.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator))

## Purpose

Harness & CI is **what must pass before anything merges to main** — the
contract that keeps main green. It covers the Rust test suite, the app unit
tests, the pre-commit hooks (fmt, clippy, SQL layer checks, grant targets, DDL
comments, brew tokens, dbd pins, JSON wire shapes, deploy/upgrade tests), the
e2e suite (known reds from T-3 block green-main claims), and the release
automation. The job is **no change reaches main without tests, lint, fmt, and
e2e**. The user feels this as *main is always shippable — I trust the gate.*

> **Status honesty.** The pre-commit suite and Rust/app unit tests run on every
> PR and are **green** (1,962 tests). The e2e suite has **known reds (T-3)**
> that block green-main claims. The release automation (bump, publish, tag) is
> designed but not fully automated. The `.github/` workflows + `.qlty/` config
> exist.

## In scope / out of scope

- In: the pre-commit hook chain (fmt → clippy → SQL layer → grant targets →
  DDL comments → brew tokens → dbd pins → JSON wire shapes → deploy/upgrade
  tests); the Rust test suite (`cargo test --workspace`); the app unit tests
  (`bun run test:unit` — 1,764 tests); the e2e suite (Playwright, known reds
  T-3); the release automation (version bump, crate publish, Homebrew formula
  update, git tag, GitHub Release); the qlty config (`.qlty/qlty.toml` with
  `exclude_patterns` for mockups, grammars, minified files).
- Out: the install implementation (P1), the website claims (P2), the Dōjō
  deployment (T1).

## Surfaces

- **CI** — `.github/workflows/*.yml` (PR checks, nightly, release); `.qlty/`
  (qlty config); `Makefile` (dev convenience, not the install path).
- **Local** — pre-commit hook (runs the fast unit test suite); `cargo fmt --check`,
  `cargo clippy -- -D warnings`; `bun run lint`, `bun run fmt:check`.
- **Release** — `cargo release` (or manual: version bump → `cargo publish` →
  Homebrew formula PR → git tag → GitHub Release).

## Data invariants

- **Lead.** **No change merges without the full pre-commit suite passing.**
- **Lead.** The e2e suite has **known reds (T-3)** — these are documented and
  do not block PR merges but **do block "main is green" claims**.
- **Lead.** The release order is canonical: version bump → crate publish →
  Homebrew formula update → git tag → GitHub Release.
- **Lead.** The qlty `exclude_patterns` includes `docs/mockups/**` and
  `crates/senseid/grammars/**` — these are prototype/generated and not our
  code quality signal.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Pre-commit status | all hooks passed | `✅ fmt · ✅ clippy · ✅ sql-layer · ✅ unit-tests` |
| E2E status | passing / known-red / flaky | `T-3 known red · 47/51 passing` |
| Release status | drafted / published / failed | `v0.12.0 · published` |
| Qlty score | code quality grade | `A-` |

## Done gate

- A PR that passes all pre-commit hooks can be merged to main.
- The e2e suite's known reds (T-3) are documented in the workflow; they don't
  fail PR checks but are visible in the run summary.
- A release candidate tag triggers the canonical release automation.
- The Homebrew formula PR is opened automatically with the new version/sha256.

```bash
# Local pre-commit (fast)
git commit -m "..."  # runs fmt, clippy, sql-layer, grant-targets, ddl-comments, brew-tokens, dbd-pins, json-wire, deploy-upgrade

# CI full suite
cargo test --workspace
cd app && bun run test:unit
cd website && bun run test
npx playwright test  # e2e
```

## Wrong gate

- **A PR merges with a failing pre-commit hook** — hook bypass regressed.
- **An e2e known-red (T-3) starts failing in a new way** — flakiness regressed.
- **A release publishes crates but the Homebrew formula isn't updated** —
  release order regressed.
- **The qlty config stops excluding mockups/grammars** — noise regressed.
- **A fmt/clippy error slips through** — lint gate regressed.

## Scenarios

- [`scenarios/harness-ci.feature`](scenarios/harness-ci.feature) — the
  pre-commit chain, Rust/app unit tests, e2e known reds, release automation
  (pinned for pre-commit/unit tests, `NO TEST` / `# gap: T-3` for e2e reds).
- [`scenarios/harness-ci-gaps.feature`](scenarios/harness-ci-gaps.feature) —
  the e2e known reds and release automation gaps (`NO TEST` / `# gap: T-3`).

## Open gaps

- [T-3](../../../../roadmap/06-gaps.md) — e2e known reds block green-main
  claims (P1).
- [D-12](../../../../roadmap/06-gaps.md) — dependency drift: `@rokkit/*`,
  `@kavach/*` held back by lockfiles (P3).
- [D-13](../../../../roadmap/06-gaps.md) — 28 medium OSV advisories
  (transitive; none code-fixable) (P3).

## Related

- [.github/](../../../../../.github/) — workflows.
- [.qlty/](../../../../../.qlty/) — qlty config (exclude_patterns).
- [Makefile](../../../../../Makefile) — dev convenience.
- [prd/03-personas.md §1,2,3](../../../03-personas.md).
- Crates: `crates/bootstrap/` (the pre-commit test suite lives here).