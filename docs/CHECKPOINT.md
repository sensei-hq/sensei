# Checkpoint — 2026-11-26 (#538 public install: PR mergeable, release pending)

**Slice:** #538 — brew + first-run schema 404 for users outside the org.
**Branch:** `fix/public-install` (off develop, pushed; not merged).

## Done
- `65d29e974` schema embedded in the binary (build.rs, git-tracked
  `database/` only — never untracked capture); deploy writes it to
  `~/.sensei/schema/v{ver}/`; `SENSEI_DDL_DIR` still overrides. Fresh
  install from the embedded schema verified on Postgres.
- `4e621174f` binaries publish to public `sensei-hq/sensei-releases`;
  update-tap downloads anonymously before rendering; renderer refuses
  non-public URLs (`make check-tap-render`, in CI); formula drops SSH
  `--HEAD` + rust/make build deps; Report issue → sensei-releases.
- `2a2a2e8b0` merged develop (#511 Windows lifecycle) into the branch.
- `925b12d3b` Windows missing-binaries remedy → sensei-releases
  (`RELEASES_LATEST_URL`); it pointed at the private repo's releases.
- Owner added `RELEASES_GITHUB_TOKEN` (confirmed via `gh secret list`).

## Remain
1. CI green on #539.
2. Merge `fix/public-install` → develop → main; cut the release.
3. Verify: anonymous `brew install sensei-hq/tap/sensei` on a clean user,
   then `sensei start` creates the DB with no network.
4. Carried: #513/#512/#444/#393/#268/#531/#532/#430 close on the next main
   merge; #292 stale vs #536; candidates #522, #533, #301/#283.

## Next command
`gh pr checks 539 --watch`

## Open questions
- Licence swap + third-party notices: separate slices
  (website `docs/plan/site-split.md` §5).
- 6 process.rs fmt spots (local vs CI toolchain) — untouched.

## Known-broken
- Until the next release the tap points at v0.27.1 on the private repo,
  so `brew install` 404s for users.
