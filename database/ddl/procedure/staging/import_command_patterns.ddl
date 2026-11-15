set search_path to staging, sensei, extensions;

-- Seed the build/test command catalogue from staging.command_patterns.
--
-- Same timestamp guard as import_reason_codes: deploy order is apply → import,
-- so the seed otherwise has the last word and a hand-edited row would be reverted
-- on the next deploy with nothing to show why.
create or replace procedure import_command_patterns()
language plpgsql
set search_path = staging, sensei, extensions
as $$
begin
  insert into sensei.command_patterns (
      toolchain, pattern, category, wrapper, compiles, owns, priority, modified_at
  )
  select
      stg.toolchain
    , stg.pattern
    , nullif(stg.category, '')
    , coalesce(stg.wrapper, false)
    , coalesce(stg.compiles, false)
    , stg.owns
    , coalesce(stg.priority, 0)
    , coalesce(stg.modified_at, now())
    from staging.command_patterns stg
   where stg.toolchain is not null
     and stg.pattern   is not null
  on conflict (toolchain, pattern) do update set
      category    = excluded.category
    , wrapper     = excluded.wrapper
    , compiles    = excluded.compiles
    , owns        = excluded.owns
    , priority    = excluded.priority
    , modified_at = excluded.modified_at
   where excluded.modified_at >= sensei.command_patterns.modified_at;
end;
$$;
