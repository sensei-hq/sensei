# 語 · Pipeline · Insight copy generation

**Owner file:** (proposed) `crates/senseid/src/analysis/narration_cache.rs`
**Called by:** every producer of user-facing insight text — [[pipeline/signals]], [[pipeline/insights]], [[pipeline/memory]] (adoption blurbs), koan authoring on Today, drift note copy, etc.

## Purpose

Human-readable insight text is the **mentor voice** of the product.
Static templates hit their ceiling immediately: `"40 tools dormant"`
reads as noise the second time you see it; `"{short}: dormant"` is
generic; every screen ends up sounding like the same auto-form-letter
generator.

This pipeline routes every human-facing insight string through a
small local model (embedded gemma4 via the sensei gateway) that
takes structured facts and returns copy that reads like a mentor
noticed something specific. Static text is the **fallback** for cold
starts, timeouts, or failure — never the default.

Kanji is 語 — *word / to speak*.

## Design principles

1. **Model owns voice, code owns action.** The model writes the
   sentence you read; the code owns the button label / route / verb.
   `action: "Edit tool: sensei.search"` never goes through the model.
2. **Fixed input schema per `kind`.** Each insight kind has a stable
   `facts` shape so the prompt template is small and cache-friendly.
3. **Deterministic fallback.** Every call site carries a static
   template. The model is optional; the screen renders even if
   ollama is down.
4. **Time-boxed.** The producer waits at most `timeout_ms` (default
   400ms) for the model. Miss it → fallback → warm-up in background.
5. **Persisted, not just cached.** Generated copy lives in
   `sensei.narration_cache` keyed on `(kind, facts_hash)`. Same facts →
   same copy indefinitely (until eviction). This turns a "call gemma4
   every request" into a "call gemma4 once when the facts change";
   the wire path never blocks on inference in steady state.
6. **Consistent voice.** All prompts share the sensei voice charter
   (below). No emoji, no marketing lift, no exclamations.
7. **Two generation modes.**
   - **Eager** — the `AggregateToolInsights` and other periodic
     tasks call the model at tick time, so wire responses read from
     the persisted row with no inference cost.
   - **Lazy** — surfaces with variable input (Today koan on a fresh
     boot, ad-hoc insights) call at request time, miss the cache,
     hit gemma4, and store on the way back.

   Both modes write to the same `narration_cache` table. A lazy write
   is indistinguishable from an eager one on read.

## Voice charter (fed to every prompt)

    You are Sensei, a quiet mentor watching a developer work with AI
    coding assistants. You notice things and say them once, plainly.
    You are specific about what you saw and what you would change.
    You never use marketing language, exclamations, emojis, or the
    word "leverage." You never call the user "you" more than once
    per message. Sentence case. Lowercase "sensei" and "ollama."
    Short — the reader is glancing, not reading.

## Public function shape

    /// Generate the visible `title` + `detail` for one insight card.
    ///
    /// - `kind` selects the prompt template + fallback template
    /// - `facts` is a strongly-typed JSON payload the caller
    ///   assembles from the DB (< 200 tokens after serialization)
    /// - `limits` caps the title (60 chars) and detail (180 chars)
    /// - `fallback` is the deterministic string set the caller must
    ///   provide — used when the model is unavailable, times out,
    ///   or returns copy that doesn't fit the limits
    ///
    /// Returns the model's output when available and valid,
    /// otherwise `fallback`.
    pub async fn generate_narration_cache(
        kind: InsightKind,
        facts: &InsightFacts,
        limits: CopyLimits,
        fallback: FallbackCopy,
    ) -> InsightCopy;

    pub enum InsightKind {
        // Health signals
        ToolWarn, ToolOpportunity, ToolDormant, ToolWorkhorse,
        ToolsDormantSummary, ToolsWorkhorseSummary,
        // Learnings triage
        MemoryProposedAdopt, MemoryProposedReview,
        PatternPromoted,
        DriftDetected,
        // Today koan
        HeroKoanEarly, HeroKoanMature,
        InsightRecurringPattern, InsightAdopted, InsightDrift,
        // Impact
        FtrLift, FtrRegression,
    }

## Data invariants

- The sensei gateway (`gateway-embedded`, `sensei-hq/gateway`) has an
  `narration-cache` chain configured with:
  - **primary:** ollama gemma4 (embedded, local)
  - **timeout:** 400ms
  - **temperature:** 0.3 (voice consistency > variety)
  - **max_tokens:** 120
  - **no** fallback to remote providers — offline should still work
- Model availability is detected once at daemon boot and re-probed
  on each cold-start after a 60s failure back-off. If unavailable,
  `generate_narration_cache` short-circuits to fallback without an
  attempt (no per-call timeout tax).

### DDL — `sensei.narration_cache`

Proposed shape (new table; add under `database/ddl/table/sensei/`):

    create table if not exists narration_cache (
      kind          text        not null,
      facts_hash    text        not null,
      title         text        not null,
      detail        text        not null,
      model_provider text,
      model_id      text,
      generated_at  timestamptz not null default now(),
      last_used_at  timestamptz not null default now(),
      primary key (kind, facts_hash)
    );

    create index if not exists narration_cache_last_used_idx
      on narration_cache(last_used_at);

**Semantics.**
- `facts_hash = sha256(kind + canonical_json(facts))` — deterministic
  and small (< 60 bytes). Any facts change makes a new key.
- Rows are **never deleted synchronously**. A daily maintenance task
  deletes rows with `last_used_at < now() - interval '30 days'`.
- On read, bump `last_used_at = now()` so hot copy stays warm.
- On write, `ON CONFLICT (kind, facts_hash) DO UPDATE SET title =
  EXCLUDED.title, detail = EXCLUDED.detail, generated_at = now(),
  last_used_at = now()` — the newer model output wins if a re-gen
  landed.

**Impact on existing tables.**
- `sensei.tool_insights.signal_title` / `signal_detail` become the
  **fallback text**, not the wire truth. The observatory read path
  first checks `narration_cache(kind, facts_hash)`; on miss it falls
  back to the tool_insights static columns.
- No wire-shape change. `GET /api/observatory/tool-signals` still
  returns `{ signals: [{ tool_name, variant, title, detail, action? }] }`
  — the `title` and `detail` come from `narration_cache` when present,
  from the static fallback otherwise.

### API impact

- **No new endpoint.** Consumers keep calling the same URLs; the
  daemon internally routes through `generate_narration_cache`.
- **Optional debug endpoint** (dev-only): `GET
  /api/observatory/narration-cache/{kind}/{facts_hash}` returns the raw
  cached row for spot-inspection. Not part of the public API.

## Prompt shape (one template per `kind`)

    <voice-charter>
    You are Sensei, … (charter as above)
    </voice-charter>

    <task>
    Kind: tool_dormant.
    Write a card that tells the developer one specific tool is
    dormant. Include the tool name. Suggest what to do.
    </task>

    <facts>
    { "short": "get-callers", "tool": "sensei.get-callers",
      "days_since_last_use": 42, "total_calls": 3 }
    </facts>

    <limits>
    title ≤ 60 chars, detail ≤ 180 chars
    </limits>

    <format>
    Return JSON: { "title": "...", "detail": "..." }
    No prose. No preamble.
    </format>

The producer parses that JSON, validates the char limits, checks for
`sensei` / `ollama` casing, strips exclamations, and either uses it
or falls back.

## Done gate

- `generate_narration_cache` returns in ≤ 400ms P95 on a warm daemon
  when ollama is available.
- Fallback path (ollama absent) has zero measurable latency (short-
  circuit, no attempt).
- Cache hit rate ≥ 60% in steady state on Jerry's data (the same 40
  dormants regenerate at most once per 24h).
- Titles and details always fit their char limits — no ellipses in
  the rendered card.
- Voice check: 100 sampled outputs contain no emojis, no
  exclamations, no marketing terms (leverage / seamlessly /
  effortless / robust / powerful), no "Claude" (the word "sensei" is
  the mentor, not the assistant).
- Same `(kind, facts_hash)` returns the cached copy on the second
  call within 24h.

Optional check:
```
curl -sN 'http://localhost:7744/api/observatory/tool-signals?nocache=1' \
  | jq '.signals[0]'
# expected: title + detail differ from the static fallback template
# expected: source: "derived" and no exclamation in the strings
```

## Wrong gate

- **Every insight reads identically across all 40 dormants.** Caching
  is keyed on `kind` alone, not `(kind, facts_hash)`.
- **The Health tab shows the fallback template even when ollama is
  running.** Timeout too tight OR the JSON parse is over-strict.
- **A card that says "Leverage your dormant tools!"** — voice
  charter drift; add a regression test with a banned-words list.
- **Model output leaks into the `action` string.** Action is
  code-owned; a producer wired action through the LLM is the bug.
- **Cache-poisoning: same tool, wrong copy.** Facts hash omitted a
  field the copy depends on (e.g. days_since_last_use rolled over
  but hash reused).
- **Latency spike on a busy tab.** A UI batch of 40 insights all
  called the model — should have been curated first (5 calls max
  per screen).

## Related

- [[pipeline/signals]] — primary consumer today
- [[pipeline/insights]] — Today koan + Learnings-triage copy
- [[pipeline/memory]] — adoption blurbs
- [[pipeline/impact]] — before/after FTR sentences

## Open questions

- Do we want the model to also suggest the `action` verb, or is
  keeping actions deterministic (as specified) the right call
  long-term? Bias: keep deterministic — the mentor voice writes the
  observation, the product decides the affordance.
- Streaming the response for larger surfaces (Today koan body) vs
  await-full for cards. Bias: await-full for cards, stream for the
  koan hero body only.
- Do we want a "sensei's take" second layer that runs a bigger
  model (remote allowed) for the Weekly digest surface? Deferred —
  local-first is the day-one contract.
