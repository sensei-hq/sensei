-- Fixture table: a coalesce outside view/function/procedure is not S1.
create table if not exists metrics (
  value numeric generated always as (coalesce(raw, 0)) stored
);
