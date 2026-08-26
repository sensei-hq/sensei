# session-report

A retrospective from GitHub Copilot CLI transcripts.

```
session-report --input <folder> [--name <label>] [--out <file.md>]
```

`<folder>` holds one directory per session, each containing `events.jsonl`.
Pointing it straight at a single session directory also works.

```bash
cargo build --release
./target/release/session-report -i ~/transcripts/manoj -n "Manoj" -o manoj.md
```

## What it is for

Reading someone's sessions back to them: how the work went, where it snagged,
what to try. It is not a scorecard — there is no ranking and no target.

## Deliberate constraints

- **Detached from the workspace.** Its `Cargo.toml` carries its own empty
  `[workspace]`, so a build at the repo root never picks it up. It links no
  daemon code and opens no database — these are other people's transcripts and
  must not reach ours.
- **Nothing is estimated.** Every number comes from an event in the file. Where
  the data cannot answer, the report says so rather than showing a zero.
- **Every observation carries a reference** — session id and timestamp, often the
  event id — so it can be checked before being shown to anyone.
- **Findings have thresholds.** A section with nothing to say prints nothing;
  padding a retrospective with "no issues found" teaches people to skim it.

## Why it reads events directly

The ingestion adapter produces `ParsedTranscript`, which keeps prose turns and
synthesised events but drops tool success, turn boundaries and shutdown totals —
the signals most of these metrics rest on. `SynthEvent` has no field for whether
a tool call succeeded. Until it does, the richer read has to happen here. See
`docs/2026-08-26-copilot-adapter-review.md`.

## Measuring time

Session span is first-to-last event, which counts a session left open overnight
as many hours of "work". The report uses **active time** instead: the sum of gaps
between consecutive events, discarding anything over ten minutes. On the sample
that is the difference between 3,364 hours and 43.

## The qualitative half — `--facets`

The mechanical figures cannot say what someone was trying to do, what got in the
way, or whether it worked. Those need reading the session, so `--facets` makes
one model call per session and writes a fixed-shape record to
`facets/<name>/<session-id>.json`:

```
underlying_goal   one sentence, what they were trying to achieve
goal_categories   from a closed list (feature_implementation, testing, …)
outcome           completed | mostly_achieved | partial | blocked | abandoned | unclear
friction          from a closed list (repeated_tool_failures, lost_context, …)
friction_detail   one sentence
primary_success   what went best
brief_summary     two sentences
evidence          a verbatim quote from the transcript
```

The report's qualitative sections are **group-bys over those records**, not
generated prose, and the remedies are a lookup table keyed by friction kind. A
model asked to "write a retrospective" produces plausible advice nobody can
check; a group-by over grounded records produces advice that names the session it
came from.

Two rules keep it honest, both borrowed from sensei's process-quality analyzer:

- a record whose `evidence` is not found in the transcript is **dropped**, not
  stored — whitespace is normalised, so a correctly copied quote that got
  re-wrapped still counts, but a reworded one does not;
- values outside the closed vocabulary are discarded, so a model inventing a
  category cannot break a group-by.

```bash
# Local by default — nothing leaves the machine.
session-report -i ./alex -n alex -o reports/alex.md --facets

# An explicit endpoint, if you mean it.
session-report -i ./alex -n alex -o reports/alex.md \
  --facets http://host/api/generate --facet-model qwen3:14b
```

Records are cached: re-running only derives the sessions that are missing, and
regenerating a report needs no model at all. Sessions that yield no record are
**named** in the run output rather than counted anonymously, because a coverage
gap that reads as "nothing to report" is worse than one you can go and look at.

On the sample this covers 131 of 149 sessions.
