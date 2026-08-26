set search_path to staging, extensions;
create or replace procedure import_intake_guide()
language plpgsql as $$
begin
  -- Was insert-only (`where not exists`), which never clobbered but also never
  -- APPLIED: editing a prompt in the datafile did nothing to an existing row, so
  -- the file silently stopped being the source of truth. Now a guarded upsert.
  insert into sensei.intake_guide (kind, axis, prompt, help, enabled, source, modified_at)
  select kind, nullif(axis,''), prompt, help, coalesce(enabled,true),
         coalesce(source,'builtin'), coalesce(modified_at, now())
    from staging.intake_guide stg
  on conflict (kind, axis) do update
     set prompt      = excluded.prompt
       , help        = excluded.help
       , enabled     = excluded.enabled
       , modified_at = excluded.modified_at
   where sensei.intake_guide.source = 'builtin'
     and excluded.modified_at >= sensei.intake_guide.modified_at;
end;
$$;
