---
name: release
description: Cut a patch/minor release end to end
---

Follow the **Release Checklist** in `~/.claude/CLAUDE.md` — these steps are its
concrete form for this repo. If the two ever disagree, the checklist wins.

1. Confirm working tree clean; abort if not.
2. Run: cargo test --all && cargo clippy --all-targets -- -D warnings && cargo fmt --check
3. Bump version in Cargo.toml, update CHANGELOG.md from commits since last tag.
4. Sync docs/ and .claude/skills/ with current source behavior; flag any drift.
5. On develop: `make bump v=patch|minor` (commits the version, pushes develop). Open the develop → main PR;
   merge it when green. Then on `main`: `make release` — tags main's head, pushes only the tag (main is
   protected), syncs the marketplace, installs locally, merges main back into develop. The tag's
   `release.yml` (`on-main` first) builds, releases and deploys. `scripts/release-guard.sh` refuses elsewhere.
6. Install the built artifact and re-run the original bug repro. Report the exit code.
7. Confirm CI green on main before declaring done.
