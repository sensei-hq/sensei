# Distribution · P1

**Plane:** platform · **Status:** [roadmap/05-platform.md](../../../../roadmap/05-platform.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Distribution is **one command that installs sensei on macOS, Linux, and
Windows** — the platform-specific behaviour lives behind a platform adapter; the
design is finished first, then development distributes across the three targets.
The Mac path (Homebrew tap, `brew services`, codesign, launchd) and parts of
the Linux/Windows story (tarball matrix, `windows.rs`) exist — the service
layer and data-directory assumptions do not. The job is **platform-truthful
install**: the reorg's platform matrix (§10) is its spec input; the acceptance
is that the installed binary runs, the daemon starts, and health checks pass on
all three targets. The user feels this as *I run one command and sensei works —
I don't debug platform quirks.*

> **Status honesty.** The macOS Homebrew formula + tap exist and are maintained
> (`homebrew/` subtree → `sensei-hq/homebrew-tap`). The bootstrap crate has
> macOS and Windows platform checkers (`health/platforms/{macos,windows}.rs`).
> **The Linux service layer (systemd/user), data-directory conventions, shell
> integration, and the unified install script are unbuilt** (gap P-5, P1). The
> design-first decision (reorg §10.3) means the platform matrix must be
> authored before the implementation distributes.

## In scope / out of scope

- In: the platform matrix (reorg §10) — per-target service manager, data dir,
  binary layout, shell integration, codesign/notarization, auto-update; the
  unified install script (`curl ... | sh`); the Homebrew formula + tap (macOS);
  the tarball + checksum matrix (Linux/Windows); the bootstrap health checkers
  per platform; the `sensei-bootstrap` crate (provisioning, engines, health,
  upgrade).
- Out: the marketing website claims (P2), CI/harness (P3), the Dōjō SaaS
  deployment (T1).

## Surfaces

- **CLI** — `sensei install` (unified); `sensei doctor` (health); `sensei upgrade`;
  `sensei self-update`; `brew install sensei-hq/tap/sensei` (macOS).
- **Daemon** — `bootstrap.rs` (provisioning, engine installs, health graph,
  upgrade orchestration); `health/platforms/{macos,linux,windows}.rs`
  (checkers + resolvers); `engines.rs` (engine install/pin/verify);
  `database.rs` (DB setup/migrate); `upgrade.rs` (canonical step order).
- **Artifacts** — Homebrew formula (`homebrew/Formula/sensei.rb`); tarball
  releases (`sensei-<version>-<target>.tar.gz` + `.sha256`); Windows installer
  (MSI or signed zip — TBD).
- **Data** — `~/.sensei/` (data dir — platform-specific default); daemon PID/
  socket; engine installations (`~/.sensei/engines/`).

## Data invariants

- **Lead.** **Design first, then distribute** (reorg §10.3 decision 7). The
  platform matrix is authored and reviewed before any implementation.
- **Lead.** The service layer is **platform-native**: macOS → `launchd` (via
  `brew services`); Linux → `systemd --user`; Windows → scheduled task or
  native service.
- **Lead.** The data directory follows **platform conventions**: macOS →
  `~/Library/Application Support/sensei`; Linux → `~/.local/share/sensei`
  (XDG); Windows → `%LOCALAPPDATA%\sensei`.
- **Lead.** The binary layout is **identical across targets** — single
  `sensei` binary with subcommands; platform adapters are compile-time.
- **Lead.** Codesign/notarization is **macOS-only**; Windows → Authenticode
  signature; Linux → unsigned tarball + checksum (cosign/Reproducible builds
  later).
- **Lead.** Auto-update uses the **same channel** on all targets; the bootstrap
  crate orchestrates the upgrade steps in canonical order.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Install command | one-liner for all targets | `curl -fsSL https://sensei.dev/install.sh \| sh` |
| Health check | all-green on fresh machine | `sensei doctor` → 5/5 ready |
| Service status | daemon running via native manager | `brew services list \| grep sensei` |
| Data dir | platform-conventional | `~/Library/Application Support/sensei` |
| Engine status | rustup, ollama, postgres ready | `sensei doctor --engines` |
| Upgrade | canonical step order | `sensei upgrade` → db → engines → daemon |

## Done gate

- `curl -fsSL https://sensei.dev/install.sh | sh` installs the latest release,
  starts the daemon via the native service manager, and `sensei doctor` returns
  all-green on a clean macOS, Linux, and Windows machine.
- The Homebrew formula installs, links, and starts the service on macOS.
- The Linux tarball extracts to a platform-conventional location, the systemd
  user unit starts the daemon, and `systemctl --user status sensei` shows
  active.
- The Windows artifact installs, the service starts, and `Get-Service sensei`
  shows Running.
- `sensei upgrade` runs the canonical step order (db → engines → daemon) on
  all targets.

```bash
sensei doctor --json | jq '.components[] | select(.status != "ready")'
```

## Wrong gate

- **The install script works on macOS but fails on Linux/Windows** — platform
  matrix not authored.
- **The daemon starts via `launchd` on Linux** — service layer regressed.
- **The data dir is `~/.sensei` on macOS** — platform convention regressed.
- **`sensei doctor` passes but the daemon isn't actually running** — health
  check regressed.
- **Upgrade runs steps out of order (daemon before db)** — canonical order
  regressed.
- **Windows installer is unsigned** — Authenticode regressed.

## Scenarios

- [`scenarios/distribution.feature`](scenarios/distribution.feature) — the
  platform matrix, install script, service layer, data dir, health, upgrade
  (pinned for macOS/Homebrew/bootstrap, `NO TEST` / `# gap: P-5` for
  Linux/Windows service layer).
- [`scenarios/distribution-gaps.feature`](scenarios/distribution-gaps.feature) —
  the unbuilt Linux/Windows service layer, unified install script, data-dir
  conventions (`NO TEST` / `# gap: P-5`).

## Open gaps

- [P-5](../../../../roadmap/06-gaps.md) — **Makefile/Homebrew are macOS-only;
  Linux service/data-dir/shell assumptions unresolved — design first, then
  distribute** (decision 7, reorg §10.3) (P1).
- [D-13](../../../../roadmap/06-gaps.md) — 28 medium OSV advisories (transitive;
  none code-fixable) (P3).
- [D-12](../../../../roadmap/06-gaps.md) — dependency drift: `@rokkit/*`,
  `@kavach/*` held back by lockfiles (P3).

## Related

- [reorg.md §10](../../../../reorg.md) — platform matrix, decision 7.
- [architecture/cli.md](../../../../architecture/cli.md) — CLI structure.
- [homebrew/](../../../../../homebrew/) — Formula + tap (subtree).
- [prd/03-personas.md §1,2](../../../03-personas.md).
- Crates: `crates/bootstrap/src/{bootstrap.rs, health/platforms/{macos,linux,windows}.rs,
  engines.rs, database.rs, upgrade.rs, config.rs}`.