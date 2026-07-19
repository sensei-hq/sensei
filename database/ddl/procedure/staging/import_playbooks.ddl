set search_path to staging, extensions;
create or replace procedure import_playbooks()
language plpgsql as $$
begin
  insert into sensei.playbooks (name, title, when_to_use, opening_tone, method_ref, enabled, source, created_at)
  select name, title, when_to_use, opening_tone, method_ref,
         coalesce(enabled, true), coalesce(source,'builtin'), coalesce(modified_at, now())
    from staging.playbooks
  on conflict (name) do update
     set title=excluded.title, when_to_use=excluded.when_to_use,
         opening_tone=excluded.opening_tone, method_ref=excluded.method_ref
   where sensei.playbooks.source = 'builtin';  -- never clobber org/learned edits
end;
$$;
