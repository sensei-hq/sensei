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
