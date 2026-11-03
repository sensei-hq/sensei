# Roadmap — Platform plane (P1–P3)

Distribution (P1), website (P2), harness/CI (P3). Status legend:
[`README.md`](README.md).

> **Status populated 2026-11-03.** The cells follow the readiness table in
> [`../review/2026-11-03-orientation.md`](../review/2026-11-03-orientation.md) §2 and the capability
> audit in [`../review/2026-10-27-roadmap-vs-state.md`](../review/2026-10-27-roadmap-vs-state.md) §1.5.
> The last column names what proves each cell; a status is a snapshot, so move it when the module moves.

| ID | Module | One-line "done" | Status | Evidenced by |
|---|---|---|---|---|
| P1 | Distribution | One command installs sensei on macOS, Linux and Windows | 🟡 **Partial** | The Homebrew formula and cask, the `.dmg` and the Linux x86_64/aarch64 tarballs ship from `release.yml`; there is no systemd unit, Windows target or `install.sh` (P-5), and releases are ad-hoc signed, so Keychain is revoked on every upgrade (#282) |
| P2 | Website | The site says only what is true, and sells the built product | 🟡 **Partial** | The site is built in the sibling repository [`sensei-hq/website`](https://github.com/sensei-hq/website) (moved out of this repo in #346; routes `+page`, `sensei`, `katachi`, `torii-seiki`, `privacy`, `terms`); D-11 stands — the `/sensei` page offers a Windows `.exe` and an AppImage that no release builds, and lists Codex and Aider, which have no capture adapter |
| P3 | Harness & CI | No change reaches main without tests, lint, fmt and e2e | 🟡 **Partial** | `rust.yml` runs fmt, clippy, audit and tests and is required on main, and `release.yml` cuts from `v*` tags behind `scripts/release-guard.sh`; the app and dōjō unit tests run only in `coverage.yml`, which is disabled, and no workflow runs Playwright — e2e is local (`test:inspect`) |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| D-11 | website presents unbuilt as shipped (one claim already dropped) | P2 |
| ~~D-12~~ | ~~dependency drift: `@rokkit/*`, `@kavach/*` held back by lockfiles~~ — **closed 2026-11-03:** `app/package.json` asks `^1.14.0` and `app/bun.lock` resolves `@rokkit/ui` 1.14.0; `dojo/bun.lock` resolves `@kavach/ui` 1.3.0 to match `dojo/package.json`. #191 still open but stale | — |
| D-13 | 28 medium osv advisories (transitive; none code-fixable) | P3 |
| P-5 | Makefile/Homebrew are macOS-only; Linux service/data-dir/shell assumptions unresolved — design first, then distribute (decision 7) | P1 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

P1's three-target design is written before the implementation distributes across
macOS / Linux / Windows (decision 7, reorg §10.3). P2 is written after the
roadmap statuses so its claims trace to evidence. P3's e2e reds were triaged and
baselined in #134 (closed 2026-10-18); a green-main claim still rests on that baseline.
