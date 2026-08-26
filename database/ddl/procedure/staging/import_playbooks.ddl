set search_path to staging, extensions;
create or replace procedure import_playbooks()
language plpgsql as $$
begin
  insert into sensei.playbooks (name, title, when_to_use, opening_tone, method_ref, enabled, source, modified_at)
  select name, title, when_to_use, opening_tone, method_ref,
         coalesce(enabled, true), coalesce(source,'builtin')::sensei.source_kind,
         coalesce(modified_at, now())
    from staging.playbooks
  on conflict (name) do update
     set title=excluded.title, when_to_use=excluded.when_to_use,
         opening_tone=excluded.opening_tone, method_ref=excluded.method_ref,
         modified_at=excluded.modified_at
   -- Two guards, and both are needed. source='builtin' keeps org/learned
   -- playbooks out of the seed's reach entirely; the timestamp then keeps an
   -- in-place edit to a BUILTIN one, which the source check alone let the next
   -- deploy overwrite.
   where sensei.playbooks.source = 'builtin'
     and excluded.modified_at >= sensei.playbooks.modified_at;
end;
$$;
