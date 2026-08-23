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
      -- DEPRECATED, retained only so the design matches the live type. Postgres
      -- cannot drop an enum value in place — it needs a full type recreation that
      -- rewrites every dependent column — and dbd detects the orphan but does not
      -- execute that rewrite. No metric uses it (all four moved to `usage`); it is
      -- an unused label, cheaper to carry than a live column rewrite. Remove it in
      -- a release that can afford the recreation.
      , 'cost'
      , 'usage'
      , 'velocity'
      , 'quality'
      , 'autonomy'
      , 'knowledge'
      , 'tool'
      , 'composite'
    );
