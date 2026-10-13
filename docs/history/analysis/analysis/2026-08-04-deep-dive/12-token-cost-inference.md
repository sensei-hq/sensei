## Token & Cost Inference — we are token-blind while the exact numbers sit on disk
_sensei stores `sessions.tokens_in/out` but writes NULL to all 69; meanwhile Claude Code's own transcripts — which the daemon already parses — carry exact per-message `usage`. This is a capture bug, not an inference problem._

**Context.** The prompt: "We are token/cost blind (`sessions.tokens_in/out = 0` for all 69). Idea: INFER token/cost from limit-reset messages and usage signals in the event stream." I chased the inference angle and found something better first: the real usage is not missing, it is un-ingested. I still design the limit-reset windowing (P2) and a char proxy (P1) for the gaps, but the headline is that P0 is cheap and exact.

**What the data shows.**

- **Every session is token-blind at the DB layer.** `tokens_in`, `tokens_out`, `model`, and `provider` are NULL on all 69 sessions. The columns exist; nothing populates them.
  ```sql
  SELECT count(*) total, count(*) FILTER (WHERE tokens_in IS NULL) null_tok_in,
         count(*) FILTER (WHERE model IS NULL) null_model, count(client_session_id) with_client_id
  FROM activity.sessions;
  -- total=69  null_tok_in=69  null_model=69  with_client_id=67
  ```

- **The event stream carries NO token/usage/cost fields.** Sampling `jsonb_object_keys(payload)` for the plausible carriers (`Stop`, `SubagentStop`, `PreCompact`, `Notification`, `SessionEnd`, `SessionStart`) returns keys like `last_assistant_message`, `transcript_path`, `permission_mode`, `effort`, `notification_type`, `model` — but not one `*_tokens` / `usage` / `cost` key.
  ```sql
  SELECT event_type, jsonb_object_keys(payload) k, count(*)
  FROM activity.assistant_events
  WHERE event_type IN ('Stop','SubagentStop','PreCompact','Notification','SessionEnd','SessionStart')
  GROUP BY 1,2 ORDER BY 1,3 DESC;
  -- 64 distinct (event_type,key) rows; none is a token/usage/cost field.
  ```
  `Notification.message` is only operational chatter, never a usage number:
  ```sql
  SELECT payload->>'message' msg, count(*) FROM activity.assistant_events
  WHERE event_type='Notification' GROUP BY 1 ORDER BY 2 DESC;
  -- 'Claude is waiting for your input' 1136 | 'Claude needs your permission' 276 | 'Claude Code login successful' 15
  ```

- **`gateway.*` is a static catalog — zero per-call usage logging.** The 6 gateway tables (`models`, `providers`, `routers`, `models_in_router`, `fallback_chains`, `fallback_chain_models`) hold config only. `models` has `context_window` / `max_output_tokens` (spec, not usage) and an empty `props`:
  ```sql
  SELECT count(*) FROM gateway.models WHERE props::text <> '{}';  -- 0  (no pricing/usage props anywhere)
  ```
  There is no cost table in the entire DB. Any $ figure needs an external price constant.

- **The exact tokens are on disk, and the daemon already opens these files.** Every `Stop`/`SessionStart` payload has a `transcript_path` pointing at `~/.claude/projects/<enc>/<session_id>.jsonl`. Those files carry a `usage` block on every assistant message:
  ```
  "usage":{"input_tokens":29188,"cache_creation_input_tokens":9576,
           "cache_read_input_tokens":18262,"output_tokens":910,
           "server_tool_use":{"web_search_requests":0,"web_fetch_requests":0}}
  ```
  ```bash
  find ~/.claude/projects -name '*.jsonl' | wc -l   # 1328 transcript files
  du -sh ~/.claude/projects                         # 1.4G on disk
  ```

- **The join key is clean and already stored.** `activity.sessions.client_session_id` == transcript filename UUID == `assistant_events.session_id`. Parsing the on-disk `usage` for the 67 linked sessions found **58 on disk (86.6%)**; the 9 misses are short/abandoned 2026-07-31 sessions (subagent-only or rotated) plus one 08-03 row.
  ```bash
  # for each client_session_id: glob ~/.claude/projects/*/<sid>.jsonl, sum message.usage.*
  # linked=67  found_on_disk=58  missing=9
  ```

- **Real, exact usage across those 58 sessions is enormous — dominated by cache reads.**

  | Metric | Real tokens (58 sessions) |
  |---|---:|
  | input_tokens | 14,483,292 |
  | output_tokens | 115,215,747 |
  | cache_creation_input_tokens | 603,439,922 |
  | cache_read_input_tokens | 29,766,267,910 |
  | **grand total (incl cache)** | **30,499,419,259** |
  | billable non-cache (in+out+cache_write) | 733,129,595 |

  Cache-read is 97.6% of all tokens — expected for agentic coding that re-streams a large context every turn.

- **Equivalent-API cost (published Opus 4.x rates: in $15 / out $75 / cache-write $18.75 / cache-read $1.50 per MTok — NOT in the DB) = $64,823 for 58 sessions.** Actual spend is a flat Max subscription; this is the *value-delivered* metric.

  | Cost component | USD | % of cost |
  |---|---:|---:|
  | cache_read | 44,649 | 69% |
  | cache_write | 11,315 | 17% |
  | output | 8,641 | 13% |
  | input | 217 | <1% |
  | **total** | **64,823** | 100% |

  Per-session: mean **$1,118**, median **$484**, max **$6,021**, min ~$0.

- **Rework is the cost driver — non-FTR sessions cost 4.7× more each.** Joining real per-session tokens to `sessions.ftr`/`outcome`:

  | Cohort | n | total tokens | equiv cost | $/session |
  |---|---:|---:|---:|---:|
  | FTR=true / completed | 37 | 8,430,385,063 | $18,945 | **$512** |
  | FTR=false / corrected | 19 | 22,049,609,421 | $45,804 | **$2,411** |

  19 corrected sessions burn 2.4× the *total* tokens of 37 completed ones. **9 of the 12 most-expensive sessions are `corrected`** (top session `1e9f95ca`: 80 turns, 3.03B cache-read tokens, **$6,021**). This is the tokens/FTR story sensei is built to tell — and currently can't.

  | session | outcome | ftr | turns | output_tok | cache_read | equiv_cost |
  |---|---|---|---:|---:|---:|---:|
  | 1e9f95ca | corrected | f | 80 | 8,354,134 | 3,034,287,845 | $6,021 |
  | 263eefa6 | corrected | f | 243 | 10,962,856 | 2,830,634,864 | $5,759 |
  | 1d25172c | corrected | f | 111 | 9,801,438 | 2,557,780,004 | $5,642 |
  | 4c8fbbff | corrected | f | 75 | 6,501,166 | 2,087,851,430 | $4,604 |
  | 3b2c3d6e | completed | t | 54 | 6,316,677 | 1,903,891,718 | $4,236 |

- **The char-count proxy (the brief's P1) is far weaker than hoped — off by 10²–10⁴×.** On 44 sessions that have *both* `transcript_turns.char_count` and on-disk usage: `sum(char_count)/4 = 1.16M` "tokens" vs real **output** 88.8M (100× low) and real **total** 23.5B (**20,279× low**).
  ```sql
  -- proxy side: SELECT session_id, sum(char_count) FROM activity.transcript_turns GROUP BY 1;
  -- 44 cal sessions: char/4 = 1,160,935 ; real_output = 88,799,946 ; real_total = 23,542,271,214
  ```
  Two reasons: (1) `transcript_turns` is **downsampled** — 1,517 rows over 113 sessions (~13 turns each) vs hundreds of assistant messages per real transcript; (2) cache tokens (97.6% of the real bill) leave no proportional text footprint. A char proxy can rank sessions but cannot size them.

- **The char-proxy pipeline is itself silently regressed** — same failure shape as the community write-back. `transcript_turns` was last populated 2026-07-31 (partial: 19 of 26 sessions), and Aug 1–4 (15 sessions) have **zero** turns:
  ```sql
  SELECT to_char(s.started_at,'YYYY-MM-DD') d, count(*) sess,
         count(*) FILTER (WHERE tt.n>0) sess_with_turns
  FROM activity.sessions s
  LEFT JOIN (SELECT session_id, count(*) n FROM activity.transcript_turns GROUP BY 1) tt
         ON tt.session_id::text = s.client_session_id
  GROUP BY 1 ORDER BY 1;
  -- 2026-07-31: 26/19 ; 2026-08-01: 1/0 ; 08-02: 3/0 ; 08-03: 5/0 ; 08-04: 6/0
  ```

- **Both limit-window signals exist and are extractable — from transcripts, not events.** Anthropic's UI strings show two hard windows:
  ```bash
  grep -rhoE "hit your (weekly|session) limit · resets" ~/.claude/projects/*/*.jsonl | wc -l
  # session(5-hour) hits: 172   |   weekly hits: 14
  ```
  Weekly anchors are a fixed Saturday-11am-CT cadence — perfect calibration boundaries:
  ```
  hit your weekly limit · resets Jun 21 at 11am (America/Chicago)
  hit your weekly limit · resets Jun 28 at 11am (America/Chicago)
  hit your weekly limit · resets Jul 26 at 11am (America/Chicago)
  hit your weekly limit · resets Aug  2 at 11am (America/Chicago)
  ```
  The 5-hour session bucket produced 20+ distinct reset stamps ("resets 2:40pm (America/Chicago)", …). `transcript_turns` captured a subset (46 hard + 35 soft limit lines) but the full record is in the JSONL.

- **Limit hits are not wired to anything.** `runs.paused_until` / `runs.pause_reason` exist, but `pause_reason` holds only one `_verify_ pause smoke test` value (8 NULL). No run was ever paused because of a usage limit — the 172 session-limit hits are invisible to the daemon.
  ```sql
  SELECT pause_reason, count(*) FROM activity.runs GROUP BY 1;
  -- (null) 8  |  '_verify_ pause smoke test' 1
  ```

- **Model IS knowable, just not persisted.** `SessionStart.model = claude-opus-4-8[1m]` on 98/118 events, and the transcript adapter already captures a dominant model per session — yet `sessions.model` is NULL 69/69.
  ```sql
  SELECT payload->>'model', count(*) FROM activity.assistant_events
  WHERE event_type='SessionStart' GROUP BY 1;  -- claude-opus-4-8[1m] 98 | '' 20
  ```

**Root cause / interpretation.**

This is a capture gap, not a missing-data problem. The daemon's transcript ingestion (`crates/senseid/src/transcript/{mod,claude}.rs`, landed with the cold-start analyzer #75 and `feat(analyzer): session-level model capture`, 2026-06-25) **streams the exact files that contain per-message `usage`** and extracts everything except the tokens. The `TranscriptTurn` struct it builds is `{ turn_index, user_text, assistant_text, started_at }` plus an optional `(provider, model)` from `model_for(content)`. There is no field for `usage`, so `input_tokens` / `output_tokens` / `cache_creation_input_tokens` / `cache_read_input_tokens` are read off disk into a JSON value and dropped on the floor. The parser is a few lines away from exact cost accounting and walks past it every run.

The write path corroborates this. `complete_session(... tokens_in, tokens_out)` in `pg_store.rs` does `tokens_in = COALESCE($7, tokens_in)` — the plumbing is real — but its only feeders are the MCP `record_outcome` / session-complete handlers, which read `params["tokensIn"]` / `body["tokensIn"]`. Those are values an *assistant* would have to self-report, and no assistant knows its own billed token counts, so the argument is always absent and the COALESCE always keeps NULL. sensei built a slot for tokens and then only offered to fill it from a source that can never supply them. `model`/`provider` have the same shape: a backfill `UPDATE activity.sessions SET provider=$2, model=$3 WHERE client_session_id=$1` exists, and SessionStart/transcript both carry the model, but nothing runs it over the existing rows — wired but never backfilled. So the token/cost blindness is not "hard to compute"; it is "the last mile of an existing pipeline was never connected."

Because the true bill is 97.6% cache-read tokens, the brief's char-proxy idea (P1) can't be the primary estimator: text volume tracks the ~2.4% of tokens you can *see* (input+output prose) and is blind to the context re-streaming that dominates cost. My calibration shows the `transcript_turns`-derived proxy landing 20,279× low against real totals, worsened by that table being downsampled. The char proxy is still useful for the ~13% of sessions with no on-disk transcript and for non-Claude assistants whose logs omit usage — but as a *ranker*, calibrated against the on-disk truth, not as an absolute meter.

The limit-reset windowing (P2) is real and worth building, but as an **account-level calibration layer**, not a per-session estimator. The weekly boundary is a clean, fixed anchor (Sat 11am CT); between two weekly resets the account consumed ≈ its weekly budget, which lets you convert the (subscription-flat) plan into an *equivalent-cost* denominator and sanity-check the transcript sum. The 172 five-hour hits are the more actionable operational signal: each is a forced stall that today no table records, so runs silently wait instead of pausing/rescheduling. Wire limit detection into `run_events` + `runs.pause_reason` and you get both a cost calibrator and a scheduling input for free.

**Recommendations.**

1. **(P0) Ingest real `usage` from the transcript JSONL — exact, 87% coverage, ~1 day.** In `crates/senseid/src/transcript/claude.rs`, while already streaming each line, accumulate `message.usage.{input_tokens,output_tokens,cache_creation_input_tokens,cache_read_input_tokens,server_tool_use}` per session (and per subagent sidechain — see #4). Add `tokens_in`, `tokens_out`, `cache_write_tokens`, `cache_read_tokens` (extend `activity.sessions`; `tokens_in/out` already exist) and write them from the backfill + live path. This replaces inference with truth for 58/67 sessions today and all future ones.

2. **(P0) Add an equivalent-cost computation with a real price table.** Create `gateway.model_prices(model, input_per_mtok, output_per_mtok, cache_write_per_mtok, cache_read_per_mtok, effective_from)` (or populate `gateway.models.props`) — do NOT hardcode rates in Rust, and do NOT default a price on a model miss (fail closed, per the money-facing rule). Cost = `Σ tokens_x/1e6 * price_x`. Surface `equiv_cost_usd` per session/run/feature.

3. **(P0) Backfill `sessions.model`/`provider` from SessionStart + transcript dominant-model.** The capture already exists; run the existing `SET provider,model` update over the 69 NULL rows and wire it into session-complete so it never regresses. Without model you cannot pick the right price row in #2.

4. **(P1) Include subagent sidechains in the token sum.** My totals parsed only top-level `<sid>.jsonl` and thus **undercount** — there are 4,916 `SubagentStop` events, and sidechains live at `~/.claude/projects/<enc>/<sid>/subagents/agent-*.jsonl`. The adapter already knows this layout; fold their `usage` into the parent session.

5. **(P1) Ship a calibrated char proxy for the gap only.** For sessions with no on-disk transcript / non-usage-logging assistants: `est_output_tokens ≈ Σ char_count / 4`, then scale by a fitted factor from the sessions that have both signals (recompute per assistant family). Store as `tokens_estimated` with a `source` enum (`transcript_usage` | `char_proxy` | `unknown`) so estimates are never confused with measured values. First, fix the `transcript_turns` ingestion regression (Aug 1–4 = 0 turns) — the proxy has no input until then.

6. **(P2) Detect limit-reset events and calibrate weekly windows.** Parse `hit your (weekly|session) limit · resets …` from transcripts (and any future event), write a `limit_reset` row into `activity.run_events`, set `runs.pause_reason='usage_limit'` + `paused_until=<parsed reset ts>` so runs reschedule instead of stalling. Use the fixed weekly boundaries (Sat 11am CT) to derive an account-level equivalent-budget and cross-check the transcript token sum.

7. **(P2) Expose cost in the app.** A per-session and per-feature "equivalent cost" + "tokens/FTR" tile turns the rework finding (corrected = 4.7× cost) into a visible KPI on the sessions/observatory screen.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| tokens_in / tokens_out (measured) | Σ `message.usage.input_tokens` / `output_tokens` over session transcript | `activity.sessions.tokens_in/out` ← transcript `usage` | per session (live + backfill) | NULL 69/69; parser drops `usage` |
| cache_write / cache_read tokens | Σ `cache_creation_input_tokens` / `cache_read_input_tokens` | new `activity.sessions.cache_write_tokens`/`cache_read_tokens` | per session | columns don't exist; 97.6% of real tokens uncounted |
| equiv_cost_usd | `Σ tokens_x/1e6 * price_x` | `activity.sessions.*` × `gateway.model_prices` | per session/run/feature | no price table (`models.props` empty); no cost column |
| cost_per_feature | `Σ equiv_cost_usd` grouped by run feature | sessions × `activity.run_events.feature` | per run | no cost + features not joined to sessions |
| tokens_per_FTR | `Σ tokens / count(FTR=true)`; and $/session by outcome | `sessions.tokens_*`, `sessions.ftr`, `sessions.outcome` | weekly | needs #1; today corrected=$2,411 vs completed=$512 unreportable |
| session_model / provider | dominant transcript model / `SessionStart.model` | `sessions.model/provider` | per session | NULL 69/69; capture exists, backfill unrun |
| char_proxy_coverage_% | `sessions with transcript_usage / all sessions` (+ char_proxy fallback %) | `sessions.tokens_source` | weekly | no `source` field; can't tell measured vs estimated |
| transcript_usage_coverage_% | `sessions with on-disk usage parsed / linked sessions` | parser `BackfillReport.files_ingested` | per backfill | 58/67 (86.6%) today, uninstrumented |
| limit_hits (5h / weekly) | count of parsed `hit your … limit · resets` markers | `activity.run_events` (kind=`limit_reset`) | daily | not detected; 172 / 14 on disk, 0 in DB |
| runs_paused_on_limit | runs with `pause_reason='usage_limit'` | `activity.runs.pause_reason/paused_until` | per run | never set; runs stall silently |
