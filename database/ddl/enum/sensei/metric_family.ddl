set search_path to sensei, extensions;

-- Family a metric belongs to (the machine field the UI groups/colours from, never a
-- hardcoded name). See features/metrics/catalog.md for the per-metric assignment.
--
-- `usage`, not `cost`: the token metrics measure how much CONTEXT the work
-- consumed, which is not what it cost. Under a subscription the marginal cost of a
-- token is zero, so pricing them was wrong in magnitude (~8x, since ~98% of the
-- input total is cache reads) and in direction (the number RISES as caching
-- improves). Real cost comes from the user's configured plan — see crate::cost.
create type metric_family
    as enum (
        'outcome'
      -- Real money, from the user's configured subscription — NOT tokens. The four
      -- token metrics moved to `usage` because under a flat fee the marginal cost
      -- of a token is zero; `cost` now means what it says.
      , 'cost'
      , 'usage'
      , 'velocity'
      , 'quality'
      , 'autonomy'
      , 'knowledge'
      , 'tool'
      , 'composite'
      -- STRUCTURE, not hygiene. `quality` measures how the code is written —
      -- duplication, smells, churn, coverage. Nothing measured how it is SHAPED,
      -- even though the graph holds 4,078,536 edges and 1,182,758 nodes and, as of
      -- 2026-09-28, exactly zero metrics read either (#156).
      --
      -- Appended so no existing ordinal moves; `metric_family` is a sort key.
      , 'architecture'
    );
