-- Fixture view. S1 applies under view/, function/ and procedure/.
create or replace view metric_view as
select
  coalesce(m.value, 0) as value, -- expect: S1
  coalesce(stg.family, 'claude') as family, -- expect: S1
  coalesce( -- expect: S1
    m.props,
    '{}'::jsonb
  ) as props,
  coalesce(m.a, m.b) as either,
  coalesce(stg.ts, extract(epoch from now())::bigint * 1000) as ts, -- expect: S1
  -- coalesce(m.hidden, 0) in a comment
  'coalesce(x, 0)' as label
from metrics m;
