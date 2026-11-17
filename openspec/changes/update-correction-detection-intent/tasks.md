## 1. Measurement first
- [x] 1.1 A labelled prompt set (real turns; kept local — the repo is public) + a precision/recall harness; the
  phrase list's baseline recorded in design.md (P 0.75, R ~0.05)
## 2. Classification (red first)
- [ ] 2.1 Candidates = user turns after an assistant turn; batch the classifier on the local model
- [ ] 2.2 Label + source on `activity.transcript_turns` (see design.md); phrase fallback marked; relabel when the model returns
## 3. Metrics
- [ ] 3.1 Per-turn flag, FTR, outcomes, correction-prone, recurring corrections read the label; backfill
## 4. Verification
- [ ] 4.1 Floors met on the labelled set; live run on real sessions compared with the phrase baseline; `openspec validate update-correction-detection-intent --strict`
