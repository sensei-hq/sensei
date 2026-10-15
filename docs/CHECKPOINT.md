# Checkpoint — 2026-10-15

**Slice:** #268, the dōjō diagram screens (next). #225 is live-verified; #238 is built.

**Done** (pushed to develop, through 54e60083):
- #238 steps 2–5: the usage, routing, gate and drive mods. Each was live-verified.
  The policy calls are in `docs/decisions.md` 2026-10-15; PRD M3 §Usage & cost capture.
- #225 live verification on the local stack: all 5 done-gate lines pass.
  Defects found and fixed:
  - `dojo.graph_schema_indexes.name` is now nullable.
  - A deleted file leaves the index (`repos_in_scope`; `prune_vanished` reads `files` rows).
  - `database/tests/run.sh` routes each directory to its scope's database.
  - The app states skip reasons in words.

**Remaining:** #225 decisions (below) → #268 dōjō diagram screens → #254 JPA 8b.
#238 stays open until `make bump` ships the mods (C-13 install path).

**Next command:** `/sensei:session refocus`, then #268. Its fixture is the local Supabase tenant
`personal/graphdev`, holding this repository's develop graph. Start the stack with
`cd dojo && bun run dev -- --host 127.0.0.1`.

**Open questions:**
- #225: `pg_store/tests.rs` exceeds the 3 MB slice cap. Raise the cap, chunk the slice, or split the file?
- #225: should one over-cap file be a partial success rather than error/never-agreed?
- Does `--resume` keep the session id and reset /cost? (`GREATEST` would under-count)
- Should `check_hooks` WARN when the mods module is missing?

**Known-broken / pending:**
- This repository's graph row reads `error` until the over-cap decision is made.
- The mods and the new columns reach installs only at the next release.
- The committed `database/design.dbml` is stale against `dbd dbml` (pre-existing).
