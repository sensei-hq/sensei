# Checkpoint — 2026-11-27 (#538 public install: develop merged in, CI pending)

**Slice:** #538 — the website, brew and DMG downloads 404 for users outside
the org (binaries live only on the private `sensei` repo's releases).
**Branch:** `fix/public-install` (worktree `lanes/public-install`), PR #539.

## Done
- `65d29e974` schema embedded in the binary; deploy writes it to
  `~/.sensei/schema/v{ver}/`; `SENSEI_DDL_DIR` still overrides.
- `4e621174f` binaries publish to public `sensei-hq/sensei-releases`;
  update-tap downloads anonymously before rendering; renderer refuses
  non-public URLs (`make check-tap-render`, in CI).
- `925b12d3b` Windows missing-binaries remedy → sensei-releases.
- Develop (0.28.1 + #543 janitor + daemon serve-while-booting) merged in.
  The earlier CI `test` red was `relay_host_disk_exhausted` on the runner,
  which develop's relay-health seam fixes.

## Remain
1. CI green on #539 (repo public for the run, private after).
2. Merge → develop; bump 0.28.2 → main; `make release`.
3. Verify logged-out: website Download link, DMG, tarball, fresh
   `brew install sensei-hq/tap/sensei`.

## Next command
`gh pr checks 539 --watch`

## Open questions
- `sensei-releases` has zero releases today: 0.28.2 is the first mirrored one.
