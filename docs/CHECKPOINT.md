# Checkpoint — 2026-11-20 (qlty security sweep — 26 findings triaged)

**Slice:** security follow-up to v0.25.0 (`qlty check --all`: 162 issues, 26 security).
**Branch:** `lane/security-qlty` (worktree `lanes/security-qlty`), from origin/develop @ 39df4c0d4.

## Done
- zizmor ×3 fixed: per-job permissions (coverage.yml), persist-credentials
  (deploy-dojo), no-cache (deploy-worker setup-bun). Verified with zizmor 1.29.
- Patched: tokio 1.54.1, rustix 1.1.6, aws-smithy-json 0.62.7 (CVE-2026-18140),
  block-buffer 0.12.1, async-executor 1.14.1. cargo audit ×3 green.
- Excepted per bar with proof: number_prefix + paste (dead ort feature),
  glib + proc-macro-error (Linux-GTK-only). quick-xml was already excepted.
- Accepted risk in #120: chrono, pin-utils, unsafe-libyaml (no fix exists);
  zbus 5→6 migration BLOCKED (secret-service 6 is RC; keyring pins ss5).
- Residual qlty security set is exactly the 12 accepted + 7 excepted; zero fixed
  items linger. qlty has no advisory-level suppression, so they keep listing.

## Remaining
- Push lane, PR to develop, CI green, merge.

## Next command
`git push -u origin lane/security-qlty` (from `lanes/security-qlty`), then `gh pr create`.

## Open questions
- None.

## Known-broken
- None.
