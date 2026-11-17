# Checkpoint — 2026-11-17 (#447 corrections by intent — build done, measuring)

**Slice:** label user turns by intent (qwen3.8, two stages) instead of key phrases.
Branch `lane/447-correction-intent` (worktree `lanes/correction-intent`).

**Done (each red → green):**
- Two-stage classifier + `GatewayIntentModel` (intent chain, temp 0, no fallback) — `prompt_classify.rs`.
- Seeded `intent` chain: qwen3.8:27b on ollama only (local alias: `ollama cp qwen3.8:latest qwen3.8:27b`).
- `transcript_turns.intent/intent_source/intent_at` (+ enums), guarded writes, clear on changed prompt.
- Analysis reads the label: corrections, FTR, outcome, hook-turn flags, evidence; label newer than
  analysis ⇒ session due.
- `ClassifyTurnIntent` task: daily window, batches of 10, self-chaining backfill, phrase fallback marked.
- Correction-prone + recurring corrections read the label; old gemma `classify_batch` deleted.
- Indexer dangling ratchet back at 1,197 (= base). Clippy clean. openspec validate --strict OK.
- Ignored harness `intent_labels_a_set` (daemon path over a local set).

**In flight:** daemon-path runs on held-out, fresh (150, κ 0.98, gold 40/2/108) and set 1;
score with `python3 -I ~/.sensei/eval/score_daemon.py <dir>=<labels.json> ...`.

**Remaining:** record scores in design.md; tasks.md ticks; PR to develop; install + live run.

**Next command:** `python3 -I ~/.sensei/eval/score_daemon.py ~/.sensei/eval/correction-intent-holdout-2026-11-17=…/h_daemon.json`

**Lane DB:** `TEST_DATABASE_URL=postgresql://localhost:5432/sensei_test_447_tpl` (restamp after DDL commit).

**Open questions:** none for #447. Elsewhere: #420, #389, #455, #457, #458.

**Known broken:** #483, #482 (flaky, unrelated); Keychain-approval tests fail in this shell (env).
