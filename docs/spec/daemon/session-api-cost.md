---
name: Session API cost (per model, per message)
status: draft
date: 2026-10-23
parent: docs/spec/daemon/tracker-jira-linkage.md (R7 usage) · crates/senseid/src/cost.rs (subscription cost)
---

# Session API cost — what the work would cost at Anthropic's API list prices

## Purpose

Price every AI session's tokens at **its model's published API rates**, per message, and
report the total as **API cost** wherever usage is shown (ticket, sprint, backlog — app
and CLI). It answers "what did this ticket's AI work cost?" in a figure that is comparable
across tickets and is the actual bill for an API user.

It is not the subscription cost. `crates/senseid/src/cost.rs` keeps pricing a Pro/Max plan
from its flat fee (`cost_per_result`); a subscription's marginal token costs nothing, so
the screens label this figure **API cost**, never "spent" or "billed".

## Findings this rests on (verified 2026-10-23)

- **Tokens are double-counted today.** Claude Code writes one JSONL line per content block
  of a response, each repeating the response's `message.usage`. `merge_facts`
  (`transcript/mod.rs`) and `claude_tokens_of` (`transcript/claude.rs`) add every line. On
  one real transcript: output ×2.1, cache reads ×1.9, cache writes ×1.9. Across 30
  transcripts, 1748 of 2264 messages repeat, and **no repeat differs** from its first line.
- **Thinking is inside output.** `usage.output_tokens` includes thinking, reported as
  `usage.output_tokens_details.thinking_tokens` (e.g. 313 output, 60 thinking); thinking is
  billed at the output rate. Nothing reads the breakdown today.
- **Each message carries what its price needs:** `message.model`, `message.id`,
  `usage.cache_creation.{ephemeral_5m_input_tokens, ephemeral_1h_input_tokens}`,
  `usage.speed` (`standard`|`fast`), `usage.inference_geo` (`global`|`us`|``),
  `usage.service_tier`. None is read today.
- `transcript_turns.model` is the session's dominant model copied to every turn, not the
  turn's.
- Hook `TurnComplete` usage (`activity.turn_usage`) has four counts and a model: no cache
  split, no thinking, no speed. Subagent transcripts (`<session>/subagents/agent-*.jsonl`)
  are not ingested (#73), so their turns are known only from the hook.

## Resolved design

### C1 · Count each message once

The Claude parser groups assistant lines by `message.id` and takes the **last** line's
`usage` per id (identical in every observed case; last is the final one if a client ever
streams). A usage line with no `message.id` counts once as its own message. Turn sums
(`transcript_turns.tokens_*`), the session total (`claude_tokens_of`) and therefore
`sessions.tokens_*` (via `turn_usage`) are built from the de-duplicated messages.
`merge_facts`' other callers (adapters reading Claude-style JSON) get the same rule.

### C2 · One row per API message — `activity.transcript_messages`

| Column | Source |
|---|---|
| `source`, `session_id`, `message_id` (PK together) | adapter source, transcript session id, `message.id` (or `line:<n>` for a usage line without one) |
| `turn_index` (null before the first human prompt) | the turn the line falls in |
| `sent_at timestamptz` | the line's `timestamp` |
| `model text` | `message.model` (`<synthetic>` and model-less records are not written) |
| `speed text`, `inference_geo text`, `service_tier text` | `usage.speed`, `usage.inference_geo`, `usage.service_tier` (null when absent) |
| `input`, `cache_read`, `output` bigint | `input_tokens`, `cache_read_input_tokens`, `output_tokens` |
| `cache_write_5m`, `cache_write_1h` bigint | `usage.cache_creation.ephemeral_5m/1h_input_tokens`; when the split is absent, `cache_creation_input_tokens` goes to `cache_write_5m` **and `cache_split_known = false`** |
| `thinking` bigint null | `output_tokens_details.thinking_tokens` — a breakdown of `output`, never added to it |

Written at transcript ingest beside `transcript_turns` (the session's rows replaced in one
transaction). Rows are facts, not prices: changing a price never needs a re-read.

### C3 · Prices — `sensei.model_prices`, dated

One row per `(model, speed, prompt_over, effective_from)`:

| Column | Meaning |
|---|---|
| `model text` | the API model id without a date suffix (`claude-opus-5-5`, `claude-sonnet-4-5`); a message's model matches after stripping `-YYYYMMDD` |
| `speed text default 'standard'` | `standard` or `fast` (fast rows exist only for models the list prices fast) |
| `prompt_over int default 0` | the row applies to a request whose prompt (input + cache reads + cache writes) is **over** this many tokens; the highest matching row wins (Haiku 5.5: rows at 0 and 100000) |
| `effective_from date null`, `effective_to date null` | the period the price applied (`effective_from` null = since before any known change; the 2026-10-23 seed has no start dates because the page gives none); a message is priced by the row whose period holds its `sent_at` (UTC day) |
| `input`, `cache_write_5m`, `cache_write_1h`, `cache_read`, `output` numeric | USD per million tokens |
| `us_multiplier numeric default 1` | applied to every category when `inference_geo = 'us'` (1.1 for Claude 4.6 and later) |
| `source text`, `retrieved_on date` | where the row came from (the pricing page URL) and when |

Seeded through `staging.model_prices` + its import procedure + `database/import/staging/model_prices.jsonl`
(the `reason_codes` pattern), from Anthropic's pricing page retrieved 2026-10-23, every
model in its table including retired ones (old sessions used them). Fast rows carry the
caching multipliers on top of the fast input price, as the page states.

### C4 · Cost at read — `activity.message_api_cost` (view)

Per message: the matching price row (model, speed, prompt band, date), then
`(input·in + cache_write_5m·w5 + cache_write_1h·w1 + cache_read·r + output·out) / 1e6 ·
(us_multiplier if geo = 'us')`. `cost` is **null** when no row matches (unknown model,
a fast message on a model with no fast row, a date before any row).

Per session (`activity.session_api_cost`, view): `api_cost` = Σ priced messages,
`priced` / `unpriced` message counts, and `hook_only_turns` = the session's
`turn_usage` rows with `seen_by_transcript = false` (subagents), which have no messages to
price. A session with no messages has no row.

### C5 · Where it is shown

- R7 usage gains `api_cost` (Σ over sessions with a row), `api_cost_n` (sessions summed),
  and `api_cost_partial` (sessions in the set with `unpriced > 0` or `hook_only_turns > 0`).
- The Tickets screen's cost column and the ticket detail show `api_cost` as "$1.24",
  headed **API cost**; with `api_cost_partial > 0` the cell reads "$1.24+" and the screen
  notes "Some AI work could not be priced (subagents or a model with no price), so this
  is a lower bound." `metered_cost` (OpenCode's own figure) stays in the payload, unshown.
- The CLI `usage` table's `cost` column becomes `api cost`, same rules.

### C6 · Bringing history in line

The fix changes stored token counts, so existing sessions are re-read once: the ingest's
watermark for Claude transcripts is cleared by a one-time repair keyed on a new
`sensei.config` flag (`transcripts.messages_v1`), so every Claude transcript is re-parsed,
its turns and messages rewritten, and `refresh_session_tokens` re-run. Daily token
metrics recompute from the corrected columns on their next run.

## Out of scope

Pricing subagent work exactly (needs #73: ingest `subagents/agent-*.jsonl`); batch
pricing (Claude Code does not batch); non-Anthropic models (OpenCode keeps its own
`metered_cost`); currency conversion; a UI to edit prices (a row in the seed file, then
`dbd import`); the `tools/session-report` script's own double count.

## Done gate

1. A transcript whose message spans three lines contributes its usage once to the turn,
   the session and `transcript_messages`.
2. A message with `cache_creation` `{5m: 0, 1h: 725}` stores `cache_write_1h = 725`; a
   message without the split stores it in `cache_write_5m` with `cache_split_known = false`.
3. `thinking` holds `output_tokens_details.thinking_tokens` and `output` is unchanged.
4. An Opus 5.5 message of 2 input / 33826 1h-writes / 26784 reads / 313 output costs
   `(2·4 + 33826·8 + 26784·0.20 + 313·20) / 1e6` = $0.282.
5. A Haiku 5.5 message with a 120k-token prompt uses the over-100k row; one with 90k the
   base row. A `us` message on a 4.6+ model costs 1.1×.
6. A message on a model with no price row has `cost` null; its session's usage shows the
   priced part with `api_cost_partial = 1`, never $0.
7. Changing a price row changes the reported cost of old messages in that period with no
   re-ingest; a message dated before a price change keeps the old price.
8. After the one-time repair, `sessions.tokens_out` for a re-read session equals the sum
   of its messages' `output`.

## Wrong gate

- Thinking tokens added to output (double charge).
- A missing price, or an unknown model, shown as $0.00.
- Every message priced at today's rate regardless of when it ran.
- Cache writes priced at the 5-minute rate when the transcript says 1-hour.
- "Spent", "billed" or "cost to you" wording for a subscription user.
