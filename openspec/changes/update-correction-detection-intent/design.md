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
