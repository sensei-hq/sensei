# Platform plane — distribution, web, CI

> Platform = P1–P3. How sensei reaches a machine, what the website claims, and
> what protects main. Status: [`../../../roadmap/05-platform.md`](../../../roadmap/05-platform.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| P1 | Distribution | One command installs sensei on macOS, Linux and Windows | [`README`](01-distribution/README.md) · [`distribution`](01-distribution/scenarios/distribution.feature) · [`distribution-gaps`](01-distribution/scenarios/distribution-gaps.feature) |
| P2 | Website | The site says only what is true, and sells the built product | [`README`](02-website/README.md) · [`website`](02-website/scenarios/website.feature) · [`website-gaps`](02-website/scenarios/website-gaps.feature) |
| P3 | Harness & CI | No change reaches main without tests, lint, fmt and e2e | [`README`](03-harness-ci/README.md) · [`harness-ci`](03-harness-ci/scenarios/harness-ci.feature) · [`harness-ci-gaps`](03-harness-ci/scenarios/harness-ci-gaps.feature) |

## What this plane owns

P1: the install story across **macOS (current target) and Linux + Windows
(required)**. Per the 2026-10-13 decision (reorg §10.3), every
platform-specific behaviour lives behind a platform adapter; the design is
finished first, then development distributes across the three targets. Today the
Mac path (Homebrew tap, `brew services`, codesign, launchd) and parts of the
Linux/Windows story (tarball matrix, `crates/bootstrap/health/platforms/windows.rs`)
exist — the service layer and data-directory assumptions do not.

P2: the marketing site as a **P2 module** — it must only claim what is built;
reorg §12 tracks the claims that already slipped (D-11, D-12).

P3: what CI must enforce before anything merges.

## Reading order

P1 → P3 → P2. P2 is written *after* the roadmap so its claims trace to real
statuses.

## Customer

Everyone who installs and everyone who evaluates from the website. P1's
acceptance is platform-truthful: the reorg's platform matrix
(`../../reorg.md` §10) is its spec input.

## Legacy sources

`architecture/{cli,app,website}`, `Makefile` (dev convenience, not the install
path), `homebrew/` (Formula + tap), `.github/` + `.qlty/` (CI), the existing
`website/` (SvelteKit marketing site).