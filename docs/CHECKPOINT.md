# Checkpoint — 2026-11-15 (lanes/relay-screen on lane/relay-screen-rebuild)

**Slice:** Relay screen rebuild to mockup r1, each component benched against the mockup (desktop + phone ×
light/dark × kanji/icon). Design `docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = rig).
User ruling 2026-11-14: one shell — fix rokkit, never patch per screen; reuse rokkit components over local copies.
Catalogue spec `docs/spec/2026-11-14-shared-component-catalogue.md` approved (dōjō `/preview` too, Rule 6).

**Done (red-first):**
- Earlier: rokkit 1.15.4 (stat cards, toolbar kind/count, fonts); nudge end to end; `/preview` + rig; scroll test;
  relay toolbar/Pill on rokkit; TaskHistory on `<Timeline marker="dot">`; MemberCard host tag (`editorTag`, 1.15.5).
- 2026-11-15: rokkit #184 (List `selectable={false}` + `variant="card"`) + #185 (ScreenInfo spec — rokkit
  coverage job had been red since 1.15.5) → **rokkit 1.15.6 published**. LaneList + CompletedList on display-only
  List cards: red `a09b0797b`, green `382022f51`; app + dōjō `^1.15.6`. Catalogue Order step 1 done.
- sensei v0.22.1 released (notarized, tap, dōjō deployed); shipped issues closed (memory: close on merge).

**Remaining (in order):** StatCard fixtures · #419 wrap-up (guidance v2, live DDL incl. now.ddl `last_nudge`,
editor field, install, e2e) → #426. Then catalogue Order 2–5 (guards, dōjō `/preview`, families; QueueSection and
dōjō ListSection need their row controls inside display-only rows). Queued: #446 (sqlx 0.9 / SQLite CVE), #447.

**Next command:** StatCard fixtures in `/preview` (red-first).

**Open questions (user):** none.

**Known broken (pre-existing, not from the List change — verified by a capture without it):**
- Phone `/preview/all`: the fixture index overlaps the samples (preview chrome).
- Phone LaneList: a long state pill ("approved, not merged") wraps under the name since rokkit 1.15.5's pill.
- CONTRACT §15 auto-nudge for non-review waits not built (`task.waiting` gap). Dev Vite binds IPv6 `localhost`
  only. Lane test DB: re-apply a changed view by hand (self-apply skips).
