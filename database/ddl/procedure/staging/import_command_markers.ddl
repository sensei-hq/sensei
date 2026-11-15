set search_path to staging, sensei, extensions;

-- Seed the build/test output markers from staging.command_markers. Every column
-- is in the key, so a re-import only inserts what is new; modified_at moves only
-- when the datafile's row is at least as new.
create or replace procedure import_command_markers()
language plpgsql
set search_path = staging, sensei, extensions
as $$
begin
  insert into sensei.command_markers (toolchain, category, kind, marker, modified_at)
  select
      stg.toolchain
    , stg.category
    , stg.kind
    , stg.marker
    , coalesce(stg.modified_at, now())
    from staging.command_markers stg
   where stg.toolchain is not null
     and stg.category  is not null
     and stg.kind      is not null
     and stg.marker    is not null
  on conflict (toolchain, category, kind, marker) do update set
      modified_at = excluded.modified_at
   where excluded.modified_at >= sensei.command_markers.modified_at;
end;
$$;
