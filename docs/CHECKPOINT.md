# Checkpoint — 2026-11-17 (relay UI finish-up; PR to develop)

**Slice:** finish the relay UI, then a full e2e run that the screen updates as tasks run.
Branch `lane/relay-ui-verify` (from develop `1285c3d2d`).

**Done:**
- Task view listens through its state's source (#414): red `482100cda`, green `af29814f7`.
- #381 ticket links → system browser: green `d457a560a` (`$lib/open-external.ts`).
- #382: already built in the Crew view (Add a member, crew-named line); e2e pin added.
- e2e journey: task page live + scroll (#380) + join line, `f700439ff`.
- Full e2e: 135 passed, 1 failed (settings-editable flake, passes alone → #483), 21 skipped.
  `relay-run.spec.ts` green (2.8m), incl. `4-task-page-live`.
- #447 floors recorded on `lane/447-correction-intent` `8344ca279` (worktree
  `lanes/correction-intent`). Prompt v2 + scorer ready in `~/.sensei/eval/correction-intent-2026-11-17/`.

**Remaining:**
1. PR lane/relay-ui-verify → develop; on merge close by hand #378 #379 #380 #381 #382 #383
   #384 #385 #386 #388.
2. #447: `python3 -I run_variant.py prompt_v2.txt prompt v2` in the eval dir; iterate to floors;
   then build tasks 2.1–3.1 red-first in `lanes/correction-intent`.

**Next command:** `gh pr create --base develop --head lane/relay-ui-verify`

**Open questions (user):** #420 phone header bands (~1100px); #457; #458.

**Known broken:** #483 (flaky Settings e2e, unrelated).
