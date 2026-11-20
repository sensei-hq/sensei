# Change: Relay in the dōjō over Supabase realtime (#427)

## Why
The desktop Relay screen is live (the run journey, #426, is green). The dōjō has no Relay queue at all: the `relay`
schema is deployed to the dōjō database (`database/design.yaml:118-134`, dōjō scope) but nothing writes it, its RLS
keys on the machine-local `relay.daemon_user` id that no dōjō JWT carries (`database/policies/relay/crews.sql:9`),
it is not in `supabase_realtime` (`database/apply/after/realtime_publication.sql:41` lists only `dojo.relay_sessions`,
`relay_segments`, `relay_inbox`), and nothing in `dojo/src` reads it. The dōjō's relay screens read the older
run/TodoWrite pipeline: one channel on `relay_sessions` and `relay_inbox` (`dojo/src/lib/relay-realtime.ts:46`) that
triggers a coalesced `invalidateAll()` (`relay-realtime.ts:75`). The daemon reaches
the dōjō only over HTTPS with the persona's session; a reply is consumed only by the gate's `await_reply`
(`crates/senseid/src/dojo/client.rs:568`), and nudges are logged and dropped
(`crates/senseid/src/tasks/handlers/publish_run.rs:208`).

The person must be able to watch and steer a run from the dōjō, on a phone, with no refresh, while the run stays on
their machine.

## What Changes
- **Mirror table (identity bridge).** A new dōjō table `dojo.relay_rows` holds the daemon's relay reads, one row per
  entity in the exact shape the desktop stream carries (`lane`, `run`, `decision`, `message`, `item`, plus `crew`,
  `run_task`, `task`, `task_view`, `summary`). Rows key on `membership_id`; RLS is `dojo.owns_membership`; select
  for `authenticated`; removal is a soft delete so RLS covers every change.
- **Publisher.** A `relay_sync` daemon task shapes rows with the daemon's own read code (no second shaper), runs the
  leak guard on every text leaf, never sends `report_md` unless the crew's new `share_reports` flag is on, and upserts
  through new Worker routes `/v1/t/{origin}/{org}/relay/queue/rows`. It runs on relay events (debounced) and on a
  sweep; `sensei.sync_state` records each push (the dōjō footer's freshness).
- **Publication.** `dojo.relay_rows` joins `supabase_realtime` in the existing after-script.
- **Screens.** The relay view components move from `app/src/lib/relay` to `packages/ui/relay` (`@sensei/ui/relay`);
  the dōjō (whose `@sensei/state` alias #414 adds) renders `/you/(inbox)/relay` and `/you/(inbox)/relay/[crew_id]/[task_id]`
  with `RelayState` over a supabase-js realtime transport (view ids `dojo.you.relay`, `dojo.you.relay.task`).
- **Answers back.** A dōjō answer, message or nudge writes a `dojo.relay_inbox` row bound to a relay crew
  (`relay_crew_id`, `surface: dojo`); the daemon polls a new queue inbox route with a persisted cursor and applies
  each row through the same code as `POST /relay/answers` and `POST /relay/messages`; the publisher echoes the result.
- **Verify.** A dōjō Playwright suite (`dojo/e2e`, local Supabase + wrangler + magic link) and a Tauri e2e project
  `relay-dojo` that drives the fake crew on the desktop and watches a dōjō page move with no reload at each step.

## Impact
- Specs: new capability `relay-dojo`.
- DDL: `dojo.relay_rows` (+ policies), `dojo.relay_inbox` (nullable `session_id`, `relay_crew_id`, policy),
  `relay.crews.share_reports`, `sensei.sync_entity` values `relay_sync` and `relay_inbox` (before-hook),
  `sensei.task_execution_kind` value `relay_sync` (before-hook; `TaskKind` is persisted through it),
  `realtime_publication.sql`.
- Daemon: `crates/senseid` relay read shaping split from receipts, new `tasks/handlers/relay_sync.rs`, queue inbox
  consumer, `dojo/client.rs` queue routes, `dojo-protocol` queue wire types.
- Worker / dōjō: `dojo/src/routes/v1/t/[origin]/[org]/relay/queue/*`, the existing `relay/inbox` GET excludes crew
  rows, new `/you/(inbox)/relay` routes, `dojo/svelte.config.js` aliases, `dojo/e2e`.
- Shared: `packages/state/relay` (`assembleNow`, realtime transport mapping), `packages/ui/relay` (moved components).
- Depends on #414 (`update-shared-state-sources`): the task source, `realtimeChannel` and the dōjō alias land there
  first; this change adds only the `relay_rows` mapper and transport on top.
- Not here: production deploy and prod verification (#314), structured response forms (#389), retiring the
  TodoWrite relay pages, dōjō-side run/backlog writes, a UI toggle for `share_reports`.
