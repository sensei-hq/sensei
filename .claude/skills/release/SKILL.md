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
5. Commit "chore(release): vX.Y.Z", tag, push, open/merge PR to main.
6. Install the built artifact and re-run the original bug repro. Report the exit code.
7. Confirm CI green on main before declaring done.
