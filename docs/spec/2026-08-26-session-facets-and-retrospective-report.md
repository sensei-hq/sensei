# Session facets and the developer retrospective

Status: design agreed; the offline half is BUILT and running against the sample.
The sensei-side tables (D2/D3) are not built.
Supersedes: nothing. Extends `2026-08-20-transcript-process-quality-analyzer.md`.

## What this is for

The retrospective at `~/.claude/usage-data/report-*.html` answers questions our
`tools/session-report` output cannot: *what do you actually work on*, *what goes
wrong*, *what should you change*. We want that in sensei, across every ACP.

The gap is not more counters. It is a different KIND of record.

## The two halves

Reading the reference implementation's backing store makes the split explicit.
It keeps two files per session:

| | `session-meta/<id>.json` | `facets/<id>.json` |
|---|---|---|
| produced by | parsing the transcript | one LLM call over the transcript |
| `project_path`, `duration_minutes` | ✓ | |
| `tool_counts`, `languages` | ✓ | |
| `git_commits`, `git_pushes` | ✓ | |
| `input_tokens`, `output_tokens` | ✓ | |
| `user_response_times`, `user_interruptions` | ✓ | |
| `tool_errors`, `tool_error_categories` | ✓ | |
| `underlying_goal`, `goal_categories` | | ✓ |
| `outcome`, `user_satisfaction_counts` | | ✓ |
| `friction_counts`, `friction_detail` | | ✓ |
| `primary_success`, `session_type` | | ✓ |
| `brief_summary` | | ✓ |

Every qualitative section of that report is an aggregation over the second
column. "Where Things Go Wrong" is a group-by on `friction_counts`. "What You
Work On" is a group-by on `goal_categories`. The mechanical half only supplies
the supporting numbers.

**So the thing to capture is a per-session facet record.** Not a metric.

## What sensei already has

Most of this exists. `activity.sessions` already carries the mechanical half —
`turns`, `corrections`, `tokens_fresh`/`cache_read`/`cache_write`, `metered_cost`,
gap-aware `duration`, `model`, `provider`, `acp_id`, `module` — and two facets
outright:

- `outcome` (`session_outcome`: empty | incomplete | completed | corrected |
  blocked | partial | abandoned)
- `summary` — the `brief_summary` equivalent

More importantly the *production model* already exists. The process-quality
analyzer (spec 2026-08-20) is exactly this shape: a daily incremental LLM pass,
gated by `sessions.process_analyzed_at` so a session is judged at most once until
its transcript changes, writing judgments to `props.process` and grounding each
one in `activity.session_process_evidence` — one row per cited turn, holding the
verbatim quote. Its rule D5 (*a judgment the model cannot ground in a quote is
dropped, never stored*) is already the guarantee we want: every statement in the
report is checkable against a real turn.

What is missing is only the facet VOCABULARY. The analyzer currently emits four
process signals — `spec_depth`, `spec_deviation`, `refuted_findings`,
`incomplete_analysis_llm` — which answer "did they follow the spec", not "what
were they doing and what got in the way".

## Design

### D1 — Extend the existing analyzer; do not add a second pass

The facets come out of the SAME LLM call that already reads the transcript, not
a new one. A second pass would double the per-session cost and add a second
incremental gate to keep in step with the first.

`sessions.process_analyzed_at` stays the single gate.

### D2 — One 1:1 table for the scalar facets

```
activity.session_facets
  session_id       text primary key   -- sessions.client_session_id
  underlying_goal  text not null      -- free text, one sentence
  session_type     session_type       -- single_task | multi_task | exploratory | interrupted
  satisfaction     satisfaction_level -- likely_satisfied | neutral | likely_frustrated
  helpfulness      helpfulness_level  -- very_helpful | helpful | mixed | unhelpful
  primary_success  success_kind       -- multi_file_changes | debugging | research | ...
  friction_detail  text not null default ''
  analyzed_at      timestamptz not null default now()
```

`outcome` and `brief_summary` are deliberately ABSENT: they already live on
`activity.sessions`. Duplicating them is how two tables for one thing start to
diverge.

Enums, not text+CHECK — house rule, and these are closed vocabularies that the
rollups group by.

### D3 — One child table for BOTH multisets

`goal_categories` and `friction_counts` are both `{name: count}` maps per
session. Two near-identical tables would be the same divergence risk, so:

```
activity.session_facet_tags
  session_id  text not null
  kind        facet_tag_kind  -- goal | friction
  value       text not null
  weight      integer not null default 1
  primary key (session_id, kind, value)
```

`value` stays text rather than an enum: the goal and friction vocabularies will
grow as we see more ACPs, and a new value should not need a migration. The
`kind` discriminator is closed and IS an enum.

### D4 — Reuse the evidence table, do not add a second one

`activity.session_process_evidence.signal` is already a free-text discriminator
documented as an open list. Facet evidence goes in the same table with
`signal in ('underlying_goal', 'friction:<value>', 'primary_success')`. Same D5
rule applies unchanged: a facet the model cannot cite is dropped.

This is what makes the report's claims checkable — the user's standing
requirement that every observation carry a reference.

### D5 — Report sections are aggregations, not prose generation

| Section | Aggregation |
|---|---|
| What you work on | `session_facet_tags` where kind=goal, by weight; joined to projects and languages |
| How you use it | mechanical only — tool mix, delegation, model mix, turn length |
| What went well | facets where `sessions.outcome='completed'` and helpfulness high, with cited quotes |
| Where things go wrong | kind=friction, by weight; joined to tool failure runs and `corrections` |
| What to change | rules over the friction tags → concrete recommendations |

Only the last needs judgment beyond a group-by, and it is a rules table mapping
a friction tag to a recommendation, not free generation.

## Gaps in the mechanical half — CLOSED

`languages`, `git_commits`/`git_pushes` and `user_response_times` are now derived
in `tools/session-report` for all three ACPs, from tool ARGUMENTS that were
already in the transcripts and unread. None needed a model. `user_interruptions`
remains implied by `corrections` and is not separately derived.

sensei does not yet store any of them: `activity.sessions` has no `languages`,
`git_commits` or reply-time column. That is the remaining mechanical work, and it
is a column addition rather than a new capture path — the daemon's adapter reads
the same tool arguments.

## Producing this for the five sample users — DONE

The offline pipeline is built and has run: 131 of 149 sessions carry a facet
record, and each of the five reports now ends with a "What the sessions say"
section built from them. Coverage per person: chandra 33/33, rajkumar 43/45,
dipti 30/40, Balaji 17/22, manoj 8/9.

It runs against a LOCAL ollama (`gemma4`), the same model family sensei's process
analyzer uses, so none of the transcript text left the machine. Records are
cached, so regenerating a report costs nothing.

What the implementation added beyond this design:

- **Grounding normalises whitespace.** Byte-exact comparison rejected 15 records
  whose quote was copied correctly but re-wrapped. It still rejects a reworded
  quote.
- **Dropped sessions are named, not counted.** A coverage gap that reads as
  "nothing to report" is worse than one you can go and look at.
- **VS Code needs both transcripts.** Some event streams hold no `user.message`
  at all while the journal for the same id has every prompt, so the facet pass
  falls back to the sibling journal.

The original note below still holds for the sensei-side work.

The mechanical half is already done — `reports/*.md` covers 149
sessions across the five (manoj 9, rajkumar 45, Balaji 22, chandra 33, dipti 40).

The facet half needs one LLM call per session, so 149 calls. The constraint is
that `tools/session-report` is deliberately offline — no database, no network,
these being other people's transcripts. That property should not be quietly
dropped, so:

- facets are produced only under an explicit `--facets <endpoint>` flag,
- written to a local `facets/` folder beside the reports, never to our database,
- and the folder is the input to the retrospective renderer.

That keeps the isolation guarantee intact while letting the same renderer serve
both the sample users and, later, sensei's own sessions reading from D2/D3
instead of from JSON files.

## Open questions

1. Does the goal vocabulary need to be per-ACP? A Copilot CLI session and a
   Claude Code session may not categorise the same way.
2. `user_satisfaction_counts` is a map in the reference (multiple readings per
   session) but D2 stores one value. If per-turn satisfaction turns out to
   matter, it moves to `session_facet_tags` as a third `kind`.
