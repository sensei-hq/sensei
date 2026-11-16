# Checkpoint — 2026-11-14 (forge comment write-back: W1–W7 built on feature/forge-comment-writeback)

**Slice:** comment write-back for GitLab and GitHub (spec
`docs/spec/daemon/tracker-forge-comment-writeback.md`, plan
`docs/plans/2026-11-14-forge-comment-writeback.md`, issues #436–#443, D93). Built unattended
(user, 2026-11-14).

**Done (pushed):** W1 `7305d36c3`, W3 `cb0d3dfb7`, W4 `fcad91d78`, W2+W5+W6 `867687b79`,
W7 `de85312c6`. Each feature had red-first or mutation-checked tests, fmt and clippy
(`-D warnings`) clean, and an adversarial review whose CRITICAL/HIGH/MEDIUM findings were
fixed. Full senseid suite: 4902 passed (before the W5 review fixes; tracker and handler
tests 409 after). App: 172 unit tests, svelte-check clean. CLI: 100 tests.

**Amended at build (all recorded in the spec and `.sensei/slice-state.json`):**
- `Patch` and W2 moved into W5; the forge visibility reads moved into W4 (clippy dead code).
- The visibility check runs after `writeback_target` returns Go.
- A failed visibility read is final only on 404.
- A rate-limit 403 is `rate_limited` on reads too; 410 → `NotFound`.
- `issue_exists` checks identity.
- The sanitiser defuses line-start blocks.
- GitLab keys need a namespace.
- `github::is_repo_name` refuses `.` and `..`.

**Next command:** open a PR `feature/forge-comment-writeback` → `develop`. After merge, W8
(#443, the user's): the live check X2, X3, X5, X6 on gitlab.com and github.com test projects.
Then `/sensei:complete`, which closes #436–#443.

**Open questions (user):** none.


---

# Checkpoint — 2026-11-16 (OpenSpec drafts for every open item, on lane/relay-followups)

**Slice:** spec-first follow-ups after the relay rebuild merged (#452). User (2026-11-16): walk the open items
and build OpenSpec specs before continuing. OpenSpec 0.17.2 initialized (`openspec/`, `/openspec:*` commands).

**Done:** develop's hook green again after #453 (two guards; `960e9b5ce`). Nine changes drafted + `--strict` valid:
`update-phone-layout` (#420, from a ~40-finding phone survey), `add-relay-run-grouped-lanes` (#422),
`update-relay-task-view-attribution` (#425/#395), `fix-relay-read-merge` (#450), `fix-app-api-error-reason`
(#451), `add-relay-journey-ticket-item` (#449), `add-relay-lane-host` (#396), `update-sqlx-09-sqlite-cve` (#446),
`update-correction-detection-intent` (#447). Decided: crew→run→tasks, agent = lane, implementer + reviewer
(#422); phone layout phased — guard, rokkit shell, sidebar drawer, Relay first (#420).

**#420 phase 1 done (red→green):** phone-layout guard (24 files to fix, 3 fine); rokkit 1.15.7 (#186: shell
mobile-first, labels wrap); ShellScreen info band closed on a phone; Rail drawer below md (menu + backdrop/Esc/
navigation close); LaneRow name 8rem floor, facts wrap. Relay body at 430px starts ~390px down (was ~1100px).
#396 narrowed to the lane host (classification → #455); #447 daily + backfill (floors after the baseline).

**Built 2026-11-16 (red→green, pushed):** develop merged in (dup allow row dropped). #450 a landing read re-applies
stream rows from while it was out (no blink, no second read). #451 `failureFrom` in `@sensei/state`: every app API
helper + the relay client report `message` ?? `error` ?? status text, body kept. #425 task view: lanes by codename
(`nameOf` replaces `codeOf`), handoff `by_lane`/`to_lane` (daemon) → "Handed off by X · 10:42 → Y". Full desktop
e2e 134 passed; its 2 failures were #420's (title-bar selector, button audit read `hover:bg-`) — fixed, recheck 35/35.
**Next command:** #422 `add-relay-run-grouped-lanes`, then #449, #396, #446, #447; after #422/#396 one
`make install-service` (Keychain re-prompt, #283) and run `relay-timeline.spec.ts` (#425 4.1). #420 phase 2 between.

**Open questions (user):** when to release develop → main (255 commits, incl. the forge write-back whose W8 live
check is still the user's); #447 floors after the baseline.

---

# Checkpoint — 2026-11-16 (relay screen rebuild MERGED to develop: #452, `231cbc660`)

**Slice:** Relay screen rebuild to mockup r1 (#418–#429). Design `docs/design/2026-11-12-relay-screen-rebuild.md`;
catalogue spec `docs/spec/2026-11-14-shared-component-catalogue.md` (approved; dōjō `/preview` too, Rule 6).
User ruling 2026-11-14: one shell — fix rokkit, never patch per screen; reuse rokkit components over local copies.

**Done:**
- The run journey `app/e2e/tests/relay-run.spec.ts` (fake crew `app/e2e/lib/relay-crew.ts`) is green end to end:
  Queue → draft → crew live → Start → planner ask/answer → plan review → claims/steps → reviews → gate merges →
  Past runs, all from the stream (3.2 min; relay e2e 9/9). It found 7 bugs, each fixed red-first.
- #424 runs UI (draft Start/Cancel, Past runs, past-run view), #419 wrap-up (guidance v2, docs, DDL, install),
  rokkit 1.15.4–1.15.6 (Timeline, Pill title, List `selectable={false}` + `variant="card"`), StatCard fixtures.
- PR #452 merged on green (`test`, `lint`, all GitHub checks; DeepSource JS/Rust/Shell/coverage/Secrets red —
  advisory here, findings only on app.deepsource.com, as on #417/#435).
- Closed: #418 #419 #421 #423 #424 #426 #387 #429. Daemon installed (release, with the now fixes).

**Remaining (open issues):** #420 (phone header ~1100px — the user's call), #422 (lanes by run, needs the lane's
run), #425 (codename in task facts #395; who handed off), #449 (seeded ticket in the journey), #450 (roster
blink), #451 (app API client shows the code), #396 (editor tag), #446 (sqlx 0.9 / SQLite CVE), #447. Then
catalogue Order 2–5 (guards, dōjō `/preview`, component families).

**Next command:** pick the next item with the user; the lane branch is merged (new work: a fresh lane from develop).

**Open questions (user):** #420's phone header height.

**Known broken:** e2e screenshots blank in this harness (#284). CONTRACT §15 auto-nudge for non-review waits not
built (`task.waiting`). Each senseid reinstall re-prompts the Keychain "Always Allow" (#283).
