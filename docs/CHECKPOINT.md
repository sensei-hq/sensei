# Checkpoint — 2026-11-17 (#447 corrections by intent — build)

**Slice:** label user turns by intent (qwen3.8, two stages) instead of key phrases.
Branch `lane/447-correction-intent` (worktree `lanes/correction-intent`).

**Done:**
- Measurement + decisions in openspec `update-correction-detection-intent` (`077b6ecb9`).
- Two-stage classifier in `prompt_classify.rs` behind an `IntentModel` seam: red, then green `ceec14cb5`.

**Remaining:**
1. Local-only `intent` chain seed (ollama `qwen3.8:latest`, no cloud leg) + `GatewayIntentModel`.
2. `activity.transcript_turns.intent` + `intent_source` (intent|phrase); ingest must not clobber.
   (Deviation from tasks.md 2.2: labels live on transcript turns — the corrections ground truth.)
3. Daily task: candidates (human, after an assistant reply, not machine text) → classify → write →
   re-derive affected sessions; phrase fallback marked.
4. analyze/corrections/correction-prone read the label; backfill all history.
5. Re-measure through the daemon path; `openspec validate --strict`; PR.

**Next command:** `rg -n "qwen3:8b" database/import/staging` (add the intent chain seed, red first)

**Open questions:** none for #447. User calls elsewhere: #420, #389, #455, #457, #458.

**Known broken:** #483 (flaky Settings e2e), #482 (flaky tracker test).
