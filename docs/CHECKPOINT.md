# Checkpoint — 2026-10-20

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 done; wave 3 in progress.

**Done:**
- **v0.15.0 released** (tag c7803b82; main c79d69c0; fix 9948ac80 on develop). Release workflow green; dōjō Worker
  serves 0.15.0; dōjō production reshaped (reconcile --allow-destructive → policies → `dbd diff` clean, run 39496401704).
- Carries #293 step 0 (shared `Screen`), #299 S1/S2 (shared projects), #301 (personal connection, auto-elect),
  #300 (system-browser sign-in + account guard), #296 (closed, live), #295 (closed), display-name prefill.

**Remaining:**
- Live proof of #301/#299/#300: user runs `sensei auth approve` (daemon reinstall revoked keychain, #283), signs in to
  dojo.sensei-hq.com once (backfills the personal connection), then a sync pass → check `sensei-hq/sensei` is filed
  under organization/sensei-hq and personal repos under the personal dōjō.
- #299 S3 done on develop (99cddc18): the relay payload names a project by slug + constitution only.
- #302 done on develop (17302c40), debug daemon installed: one keychain item per identity. Needs one approval
  (two prompts this once) → verify `credentials.jerrythomas` present, old items gone; then close.
- #293 step 1 done for the app (c290f38e … bae2126f, 7 screens; sessions now one screen). Next: dōjō step 1 —
  `you/[section]`, `org/[slug]/[section]`, `runs/[run_id]`. Step 2 after. #304: 26 writes still swallow failures (fix per screen).
- #294, #297, #298.
- Open: #292, #149, #278, dōjō (#132, #106, #191), code health (#199, #201, #139). Wave 5 #190; wave 9 re-index;
  wave 10 #275/#276. #187 needs `make test-app-e2e-cold`. #194 waits on a live failure.

**Next command:** after `sensei auth approve`: check the keychain migration (#302) and the sync (#301/#299); then #293 step 1 in the dōjō (`you/[section]` first).

**Open questions:**
- #289: delete the 167,932 duplicate assistant events (after `make db-backup`)? Recommended.
- #149: communities — Leiden/Louvain (recommended) or hierarchy grouping?
- #133: project glyphs in icon mode, and the dōjō glyph default (kanji recommended for both).
- #282 needs an Apple Developer account. #199: headless Linux credential store or drop Linux daemon artifacts?
- #194: drop the 145k/day `task_started`/`task_completed` rows? Orphaned Cloudflare build trigger: delete it?

**Known-broken / pending:**
- Keychain approval needed after every daemon reinstall (#283 until #282). `sensei auth status` shows both identities
  need approval; joelthomas8847 never signed in here.
- #284: inspect captures blank until `make dev-signing-cert` + Screen Recording grant. `database/design.dbml` is stale.
