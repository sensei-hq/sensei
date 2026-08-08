# AI-delivery metrics — what the research supports, and why our metric set is the right one

_2026-08-05 · independent deep-research review of the AI-augmented-delivery metrics landscape.
Companion to [`2026-08-04-metrics.md`](2026-08-04-metrics.md),
[`2026-08-04-metrics-catalog.md`](2026-08-04-metrics-catalog.md), and
[`2026-08-05-indexer-capability-coverage.md`](2026-08-05-indexer-capability-coverage.md)._

## Purpose

We surveyed the metrics the industry is circulating for "AI's impact on software delivery" —
throughput vs stability, code rework/turnover, suggestion acceptance, vulnerability density,
governance gaps — to decide which, if any, we should adopt. **Conclusion up front: we adopt
none of the circulating benchmark *numbers* as our metrics.** They fail as measurements for the
reasons documented below. Instead we keep the metric set we had already planned (first-time-right,
rework ratio, cost-of-rework, code turnover/duplication, change-failure/regression, cost-per-
feature), because it is *measured rather than believed, first-party rather than borrowed, and
paired velocity-with-quality by construction*. This doc records what the research found and the
reasons behind that decision.

> **Verification status (honest).** The research ran a 5-angle web search + 20-source fetch and
> extracted ~95 claims; the automated 3-vote adversarial-verification step then **failed on an
> API rate-limit**. Grades below are therefore assigned by **source rigor** (RCT / peer-reviewed
> vs disclosed-methodology vendor vs perception survey vs uncited re-citation) and by the
> **citation-chain tracing** the search step performed (quoted). They are not automated
> "confirmed" verdicts; a re-run can upgrade them.

---

## 1. What the credible research actually establishes

Grades: **A** = RCT / peer-reviewed / large longitudinal dataset · **B** = disclosed-methodology
vendor study · **C** = perception survey · **D** = uncited number / re-citation.

- **Productivity gains are not established, and self-reports are unreliable (A).** The **METR
  2025 RCT** (16 experienced developers, 246 real tasks in mature repos, 143 h of labelled
  recordings) found modern AI tooling made them **19% slower**, while they *believed* it made
  them ~20% faster — a ~39-point perception-vs-reality gap. METR's own 2026 follow-up does not
  reproduce the effect and it disavows any single number. **Lesson: no headline AI-productivity
  figure — in either direction — is settled, and survey/perception metrics systematically
  overstate reality.**
- **The throughput↔stability trade-off is real (A).** **DORA 2024** (Google, large-scale)
  models **+25% AI adoption → delivery stability −7.2%, throughput −1.5%** — a second
  consecutive year AI *worsened* delivery performance even as individual output rose. This is the
  defensible backbone of "speed up, stability down."
- **"Code churn / turnover" is a real, git-derivable metric (B).** **GitClear** (211 M changed
  lines, 2020–2024) gives the load-bearing operational definition: **churn = % of authored lines
  reverted or rewritten within ~2 weeks**; it observed cloned lines rising 8.3%→12.3% and
  refactored share falling 25%→<10% as AI adoption grew. (Its marketed "**4× clones**" headline
  is inflated — the measured proportion moved ~1.5×.)
- **AI code carries real security risk (A/B).** **Veracode 2025** — 45% of AI-generated samples
  failed security tests / introduced OWASP Top-10 issues, and larger/newer models did **not**
  improve security. Peer-reviewed corroboration: the classic Copilot study (~40% of 1,689
  programs vulnerable) and a 2025 result that **iterative LLM "self-improvement" raised critical
  vulnerabilities +37.6% after 5 passes** — agentic fix-loops can make security worse.
- **Balanced-scorecard practice has converged (B).** DORA, **DX Core 4** (Speed / Effectiveness /
  Quality / Business-Impact), DX's AI framework (Utilization / Impact / Cost), and Faros' "rework
  rate as the 5th DORA metric" all say the same thing: **never report a velocity number without
  its paired quality/rework counterpart.**

**Net:** the research supports the *phenomena* (speed↔stability tension, real churn, real
security risk) and the *methodologies* (churn definition, rework rate, paired scorecards) — but
almost none of the specific circulating *numbers* survive scrutiny as things to adopt.

---

## 2. Why we adopt none of the circulating benchmark numbers

Each class of widely-quoted figure, and the reason it does not become one of our metrics:

| Circulating claim (representative) | Why we don't adopt it |
|---|---|
| "PRs +20%, incidents/PR +23.5%, change-failures +30%" | **Re-citation of a platform-vendor benchmark (Grade C/D).** Traces to one vendor's 2026 benchmark, re-quoted downstream. Not reproducible, and magnitude is platform-specific — another vendor's telemetry shows incidents/PR up **242%** for the "same" phenomenon. A number that swings 10× between sources is not a metric; it's an anecdote. |
| "78% more productive / 65% new vulns / 52% no governance / 81% no visibility" | **Perception survey of ~400 security leaders (Grade C).** Measures *belief*, and METR shows belief about AI productivity is badly miscalibrated. Useful as a *gap signal*, worthless as a delivery *metric*. |
| "AI turnover 12–18%@30d, 1.8–2.5× human; 35–40% rework" | **Uncited vendor "industry averages" (Grade D).** No primary study; the one real dataset (GitClear) shows ~1.5×, not 4×/2.5×. Borrowed benchmarks aren't your metric — your own measured turnover is. |
| "41% generated, 27% accepted" (acceptance rate) | **Single-dataset, and acceptance is a weak/invertible signal (Grade C/D).** High acceptance of low-quality code is *worse*, not better. Acceptance without a retention/quality pair misleads. |
| "40–45% of AI code contains a vulnerability" | **Credible as a finding (A/B) — but it is a property of *unguarded AI code in general*, not a measurement of *our* delivery.** We don't quote it as our metric; we measure our own vulnerability density with a scanner in the loop. |
| "$0.9B→$15.7B TAM; 92% of devs use AI" | **Market slideware, not a delivery metric (Grade C/D).** Directionally fine for context; not something to instrument. |

The unifying reasons — the "why" behind rejecting all of them:
1. **Borrowed, not measured.** They are other organizations' numbers (often re-cited in loops), not computed from our own delivery. A metric you can't reproduce on your own data can't be improved or defended.
2. **Belief, not outcome.** The productivity/governance clusters are perception surveys; the one RCT proves perception ≠ reality.
3. **Single scary numbers, unpaired.** They report a velocity or a risk in isolation, which every credible framework (DORA, DX Core 4) says is meaningless without its counterpart.
4. **Inflated headlines over disclosed data.** Where a real dataset exists (GitClear), the marketed multiplier overstates it.

---

## 3. Why the metric set we planned is the better choice

We keep the metrics already defined in the [metrics-catalog](2026-08-04-metrics-catalog.md) and
[code-graph spec](../spec/pipeline/code-graph.md). Each one replaces a circulating benchmark with
a *measured, first-party, paired* equivalent — and each pairing is a documented reason it is
better:

| Our planned metric | Replaces the circulating claim | Why ours is better (documented) |
|---|---|---|
| **First-time-right (FTR)** — shipped without a correction | "% productive" perception surveys | Outcome, not belief; instrumented per session (currently 67.6%). Directly answers "was the work right the first time" — the thing surveys only guess at. |
| **Rework Ratio** + **cost-of-rework** | "AI rework 35–40% / 1.8–2.5× human" | Computed from our own sessions (0.76 of tool-calls come from corrected sessions; non-FTR sessions cost 4.7×). Uses the *concept* the vendors sell, with *our* number — reproducible and improvable. |
| **Code turnover / churn** (reverted-or-rewritten <14–30d) | GitClear's "4× clones" headline | Adopts GitClear's *definition* (credible) but computes our real value from the code graph — no inflated multiplier, our actual churn. |
| **Duplication ratio** (new symbol == existing) | "declining DRY / code reuse" narratives | Measured from the code graph at write-time, not inferred from a proxy; catches the actual regression as it happens. |
| **Change-failure / regression rate** (5th-DORA-style) | "change-failures +30%" | DORA-native definition, computed on our deployments; paired with throughput by construction. |
| **Cost / tokens per session & per feature** | absent from the circulating set | The DX "cost" dimension; makes the rework story quantifiable in money (we already showed rework costs 4.7×). |
| **Vulnerability density** (OWASP/CWE per kloc of AI code) | "40–45% of AI code is vulnerable" | Turns a general finding into a scanner-measured property of *our* code, trendable over time. |
| **Perception-vs-reality delta** (measured vs self-reported) | the entire survey genre | Bakes the METR lesson in: we track the gap rather than trusting the self-report. |

The five structural reasons our set is superior, once and clearly:
1. **Measured, not believed** — every metric is instrumented from real sessions / git / the code
   graph, so the METR perception trap doesn't apply to us.
2. **First-party, not borrowed** — they are *our* numbers on *our* delivery, reproducible and
   therefore improvable and defensible to a client; borrowed benchmarks are neither.
3. **Paired by construction** — every velocity metric is defined alongside its quality/rework
   counterpart (the DORA / DX Core 4 rule), so we can't tell the flattering half of the story.
4. **Grounded in the credible methodologies, not the inflated headlines** — we take GitClear's
   churn definition and DORA's rework-rate definition, and drop the "4×"/"2.5×" marketing.
5. **Outcome-anchored (FTR), not output-anchored (LOC / acceptance / PR count)** — we measure
   whether the work was *right*, not merely how much was produced (raw output is, per DORA/METR,
   often negative-value).

---

## 4. Positioning implication (metrics lens)

The evidence supports selling **provable, measured quality** over raw capacity: the credible
research says unguarded AI output is frequently slower, churny, and insecure, and that everyone
quoting productivity gains is quoting each other's surveys. The differentiator is therefore not a
better benchmark to cite — it is the ability to **measure and prove first-time-right and low
rework on real delivery data.** What an organization must be able to prove to make that claim
credibly is exactly the §3 set: its own FTR, its own rework rate, its own churn and cost — which
is what our instrumentation produces. This validates the existing roadmap: the metrics-catalog P0
items (token/model/cost capture) and the code-graph capability roadmap (churn/duplication from the
graph) are the precise pieces that complete this scorecard.

---

## Sources by rigor

**A (RCT / peer-reviewed / large longitudinal):** METR RCT (arXiv 2507.09089; metr.org 2025-07 +
2026-02 disavowal); DORA 2024 (dora.dev); "Asleep at the Keyboard" (arXiv 2108.09293);
iterative-refinement security (arXiv 2506.11022); GitClear 211M-line dataset.
**B (disclosed-methodology vendor):** Veracode 2025 GenAI Code Security Report; Faros.ai
(rework-rate as 5th DORA metric); GetDX (DX Core 4, AI Measurement Framework).
**C (perception survey):** Cycode 2025 State of ASPM (the 78/65/52/81 cluster); broad
adoption surveys.
**D (uncited / re-citation — not adopted):** platform-vendor throughput/stability trio (via a
2026 benchmark re-cite); vendor turnover multipliers (larridin / exceeds.ai); single-dataset
acceptance split; "322% more vulns" chains.

> **Re-verification available.** The deep-research workflow can replay from cache and run the
> 3-vote adversarial verification on the extracted claims once the API limit resets, upgrading the
> source-rigor grades to automated CONFIRMED/REFUTED verdicts.
