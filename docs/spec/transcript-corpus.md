---
name: Transcript corpus
description: Where assistant transcripts live for unit, integration and live testing — and why fixtures are generated rather than copied
date: 2026-09-28
status: current
---

# Transcript corpus

Six adapters parse assistant transcripts — `claude`, `cursor`, `zed`,
`opencode`, `copilot_cli`, `vscode` — and every format they read belongs to
somebody else and changes without notice. This states where test data for them
comes from.

## The measurement that motivates it

93 tests across those six adapters, 109 inline string literals, **zero file
fixtures**. That is not an oversight: `check-no-leaks.sh` blocks every
transcript-shaped path in the repo, so an inline literal was the only legal
option.

Inline literals are safe and they are also a hand-written guess at a real
format. A tool ships a schema change and all 93 stay green while ingestion
breaks in production. Nothing currently ties a test to what any tool actually
emits.

## Three tiers

| tier | lives | used by | committed |
|---|---|---|---|
| 1 — inline literals | in the `#[cfg(test)]` module beside the adapter | parser edge cases, error paths | yes |
| 2 — anonymised fixtures | `crates/senseid/tests/fixtures/transcripts/<family>/` | shape conformance against a real capture | yes |
| 3 — raw corpus | outside the repo, `$SENSEI_TRANSCRIPT_CORPUS` | integration runs over volume and variety | **never** |

### Tier 1 — keep it

Inline literals stay for everything they already cover. They are safe *by
construction*: you cannot paste a live session into a Rust string literal
without noticing you are doing it. Do not migrate these to fixtures.

### Tier 2 — generated, never written

A fixture is produced by `scripts/anonymize-transcript.py` from a real capture
and is committed. It is never hand-written and never hand-edited.

```
python3 scripts/anonymize-transcript.py <real-transcript> \
    --family claude --case tool-use --tool-version claude-code-2.0.14
```

The script **preserves shape and replaces content**:

- kept: every JSON key that is an identifier, every number, booleans and nulls,
  array order, nesting depth, discriminants (`role`, `type`, tool names, model
  names), and the relative timing between events;
- replaced: filesystem paths, emails, URLs, uuids, and all free prose and code.

Free text is *replaced*, not scrubbed. A prompt can contain anything, and no
denylist can be trusted to find all of it; scrubbing free prose is a bet that
you enumerated every secret in it, and replacing it is not a bet. A parser test
does not care what the words were.

Replacement is deterministic, so a repo appearing in forty turns appears as one
fake repo in forty turns. Without that a test asserting "these turns share a
cwd" would pass on the real transcript and fail on the fixture.

Two things guard it, and neither is the honour system:

- the script **verifies its own output** and refuses to write when a real home
  directory, an unlisted mailbox or a denylisted name survives. It fails closed.
- `transcript_fixtures_carry_provenance` requires a sibling `.meta.json` naming
  the tool, its version and the anonymiser run, and checks the fixture's
  **sha256 against that meta** — so a hand-edit fails the build.

`check-no-leaks.sh` carves this one directory out of its transcript-path rule.
Those two tests are the other half of that carve-out; the content rules still
run over the files.

### Tier 3 — raw, outside the repo

Real captures from experimenting with different tools live outside the
repository, exactly as `SENSEI_CORPUS` already works for languages this repo has
no source in:

```
~/.sensei/corpus/transcripts/<family>/…        # or $SENSEI_TRANSCRIPT_CORPUS
```

Tests that read it are `#[ignore]`d and print a skip line when the variable is
unset, so a fresh clone still runs green. Nothing from Tier 3 is ever committed;
it is the input to the anonymiser, not a test input itself.

## Capturing, while experimenting

Capture **diversity of shape, not volume**. Per tool, one small session each
for: a plain turn, a tool use, an error or refusal, a subagent or sub-thread, a
compaction, a resume, a session reporting cost and tokens, and one that changes
working directory mid-session. Many small sessions beat one long one — a fixture
is read by people.

Record the tool version at capture time. A fixture that does not say which
version of which tool produced it cannot be distinguished from a stale one, and
`--tool-version` is required for that reason.

## Known gap — two adapters emit a family the enum rejects

Found while writing this, and verified against the running database rather than
inferred.

`sensei.assistant_family` holds `claude cursor zed continue codex aider opencode
kiro`. `copilot_cli.rs` returns `family() = "copilot"` and `vscode.rs` returns
`"vscode"`, and **neither is in the enum**:

```
select 'copilot'::sensei.assistant_family;
ERROR:  invalid input value for enum sensei.assistant_family: "copilot"
```

The consequence is split, which is why it has gone unnoticed:

- `activity.transcript_turns.family` is **`text`**, so prose turns from those two
  adapters would land fine;
- `activity.assistant_events.family` **is the enum**, and `insert_assistant_event`
  casts to it — so the synthesized-event path would fail outright.

It is LATENT, not live: `assistant_events` currently holds `claude` 493,483,
`zed` 12,794, `opencode` 4,207 and `cursor` 18, and no rows at all from those
two adapters. Nothing has exercised the path yet.

Fix it before capturing a Copilot or VS Code session for Tier 2, or the fixture
will be the first thing to trip it. Either add both values to the enum or map
`copilot`/`vscode` onto existing ones — the first is honest, the second loses
the distinction between a harness and its host editor.
