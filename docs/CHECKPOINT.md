# Checkpoint — 2026-11-17 (#447 shipped in v0.24.0; placement floor fixed)

**Slice:** corrections by intent (#447) → release 0.24.0 → live run; then the failing placement floor.

**Done:**
- #447 merged (#486), closed; released in v0.24.0 (#487, tag CI green incl. dōjō-prod). Installed;
  daemon on 0.24.0 keeps the schema (dbd diff clean).
- Ollama router timeout 120 s → 600 s (#489, merged) — the first live intent run timed out on a 27B
  cold load. Live re-run: 20 turns labelled by the model (`intent_pass_runs`, ignored test).
- Placement floor (failing nightly since 2026-10-27): `packages/tsconfig.json` (#490, merged c25d1a930);
  CI dispatch on the branch green; nightly turns green once it reaches main (next release).

**Remaining / follow-ups:** #485 recall tuning (4th fresh draw); #488 index (session_id, prompt_id);
gateway#96 per-request timeout; dbd#121 enum ADD VALUE clash. Backfill of ~5,000 turns drains via the
daily ClassifyTurnIntent runs (watch `select intent_source, count(*) from activity.transcript_turns ...`).

**Next command:** `psql -d sensei -Atc "select intent_source, intent, count(*) from activity.transcript_turns where intent is not null group by 1,2"`

**Open questions (user):** #420 phone header bands (~1100px); #457; #458; #389; #455.

**Known broken:** #483, #482 (flaky, unrelated); Keychain-approval tests fail in non-interactive shells.
