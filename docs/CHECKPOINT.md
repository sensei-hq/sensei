# Checkpoint — 2026-10-21

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
- #293 step 1 DONE for both apps (app c290f38e … bae2126f; dōjō 9f11df08, 785279a6, a0b8aaf4, 3d443b1b — one
  run view, activity shown). #305 closed (bf16a437, live on the local dōjō; production after the next release). Step 2 (rokkit) + step 3 dead code remain.
- Dōjō expansion planned: epic #307, docs/roadmap/09-dojo-expansion.md (D1–D23, V1–V12, 2026-10-21); PRD T5 releases,
  DORA & release gates (5564841d). Wave A: A1–A3 done; Wave R (D11–D13) before A4.
- #302 closed live (keychain migrated); #301/#299 verified on the LOCAL dōjō — production needs the daemon
  enrolled with dojo.sensei-hq.com (user step: Settings → Dōjō → Connection). Local dōjō verify: a minted magic link for the local layout-verify test account. Step 2 after. #304: 26 writes still swallow failures (fix per screen).
- #294, #297, #298.
- Open: #292, #149, #278, dōjō (#132, #106, #191), code health (#199, #201, #139). Wave 5 #190; wave 9 re-index;
  wave 10 #275/#276. #187 needs `make test-app-e2e-cold`. #194 waits on a live failure.

**Next command:** gaps first, by priority (user: "document and address the gaps"): #309 (ede5da58) and #311 (f607bf0a) done — next #312 relay gates fail closed, then #318 code lags decisions. #314/#315 (production verification, release rehearsal) need the user's production access. Then #307 R2b (page URLs and words, D17), R3. Done today: R2a (65eede4d), #274 merged, archive pruned, D1–D27, gap register #309–#321.

**Open questions:**
- Answered 2026-10-21: D24 (open source; benefits of the paid tier), D25 (governance dōjō → workspace → repository), D26 (k-anonymity stays), D27 (production Relay/push untested → #314).
- #289: delete the 167,932 duplicate assistant events (after `make db-backup`)? Recommended.
- #149: communities — Leiden/Louvain (recommended) or hierarchy grouping?
- #133: project glyphs in icon mode, and the dōjō glyph default (kanji recommended for both).
- #282 needs an Apple Developer account. #199: headless Linux credential store or drop Linux daemon artifacts?
- #194: drop the 145k/day `task_started`/`task_completed` rows? Orphaned Cloudflare build trigger: delete it?

**Known-broken / pending:**
- Keychain approval needed after every daemon reinstall (#283 until #282). `sensei auth status` shows both identities
  need approval; joelthomas8847 never signed in here.
- #284: inspect captures blank until `make dev-signing-cert` + Screen Recording grant. `database/design.dbml` is stale.
