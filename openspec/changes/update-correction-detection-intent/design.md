## Decisions (user, 2026-11-16)
- Cost: the classifier runs **daily** over the day's candidate turns (as process quality does).
- Backfill: **yes** — relabel all history; phrase labels are replaced by intent labels with source `intent`.

## Baseline (measured 2026-11-17)
- Candidates: 5094 human-written user turns after an assistant turn (354 sessions; machine turns —
  command wrappers, hook output, interrupts, continuations — excluded). The phrase list fires on 60 (1.2%).
- Labelled set: 189 of them (all 59 phrase hits under a 2-per-session cap + 130 random non-hits, seed 447),
  each with the tail of the previous assistant reply. Two blind labellers agreed on 180 (95.2%,
  Cohen's κ 0.90); 9 adjudicated. Gold: 65 correction, 5 principle, 119 neither.
- The set is the user's real prompts and this repository is public: it is kept LOCAL
  (`~/.sensei/eval/correction-intent-2026-11-17/`), never committed.

| Detector (correction class, population-weighted) | Precision | Recall |
|---|---|---|
| Phrase list (today) | 0.75 | ~0.05 (0.04–0.08) |
| gemma4, today's `prompt_classify` prompt, prompt only | 0.64 | 0.81 |
| gemma4, same prompt, previous reply as context | 0.47 | 0.91 |

- Phrase list by family: "why did you…" 14/14, "that's wrong…/doesn't work…" 26/35, "revert" 4/7, "actually/wait…" 0/3.
- gemma4's false positives are mostly "fix X" requests ("fix the linting errors", "fix the RLS bug"):
  a correction targets the agent's own previous work, a fix of something else is a new request.

## Open Questions
- (none open)

## Decisions
- Q2 (user, 2026-11-17): floors **recall >= 0.80 and precision >= 0.75**, population-weighted on the
  labelled set. The labelled set stays local (`~/.sensei/eval/correction-intent-2026-11-17/`; the repo is
  public). Today's prompt meets recall but not precision (0.64); the build sharpens it (the agent's own
  previous work; "fix X" negatives) and re-measures. The non-hit stratum (130) drives the precision
  estimate's spread; growing it to ~300 would tighten both numbers.
- Compute (user, 2026-11-17): several model calls per batch and a second, larger local model are acceptable —
  the pass runs in the background analysis, not in the person's way.
- Classifier (2026-11-17, measured): **qwen3.8, two stages.** Stage 1 labels each batch of ten with the end
  of the agent's previous reply (600 chars) beside each message, under a definition that compares the
  message with that reply; stage 2 asks one strict yes/no per stage-1 correction ("does it push back on the
  agent's own work?"). Population-weighted, correction class:

  | Classifier | Set 1 (189; prompts tuned here) | Held-out (151, after tuning) | Pooled (340) |
  |---|---|---|---|
  | gemma4, today's prompt | 0.64 / 0.81 | 0.74 / 0.59 | 0.69 / 0.68 |
  | qwen3.8, strict prompt, no context | 0.80 / 0.54 | 0.90 / 0.56 | 0.86 / 0.56 |
  | qwen3.8, stage 1 only | 0.54 / 0.95 | 0.81 / 0.85 | 0.67 / 0.89 |
  | **qwen3.8, stage 1 + stage 2** | 0.66 / 0.90 | **0.91 / 0.80** | **0.78 / 0.84** |
  | vote: 2 of (strict, two-stage, gemma4) | 0.85 / 0.81 | 0.93 / 0.75 | 0.89 / 0.77 |

  (precision / recall.) The vote was chosen on set 1 and misses recall on the held-out set; the two-stage
  classifier, written after set 1, meets both floors on the held-out set and pooled. The held-out set: same
  pool and strata, seed 4471, none of set 1's turns (150 ordinary + 1 phrase hit — set 1 used 59 of the
  pool's 60); two blind labellers, 97.4% agreement, kappa 0.93; 4 adjudicated. Both sets stay local.
  The margin is modest (corrections are ~7% of ordinary turns; 55 in the two sets), so the build keeps the
  harness and re-measures on a fresh draw before shipping.
- Where labels live (build, 2026-11-17): on `activity.transcript_turns` (`intent`, `intent_source`,
  `intent_at`), not `activity.turns` as tasks.md first said. Session corrections are counted from the
  transcript, which holds the message and the reply it answers; `activity.turns` is rebuilt from hooks on
  every analysis and takes its flag from the transcript turn with the same prompt id. A re-ingest that
  changes a turn's text clears its label; a label is written only when it changes; a label newer than
  the session's analysis makes the session due again.
- Model access (build, 2026-11-17): a seeded `intent` chain with one leg, qwen3.8 on local Ollama, no
  fallback — another model is an unmeasured classifier, and the turns are the user's own prompts (the
  `classify` chain ends in cloud models). When it is unavailable, turns are labelled by the phrase list
  with source `phrase` and relabelled by a later run.
- The pass (build, 2026-11-17): `ClassifyTurnIntent` rides the analyzer's daily window, works in
  batches of ten until its time is up, and queues itself again while candidates remain — which is how the
  one-time backfill of history drains (several hours of local model time, once).
