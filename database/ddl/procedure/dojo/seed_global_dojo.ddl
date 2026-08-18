set search_path to dojo, extensions;

-- Seeds the special-case global-dojo tenant — the public collective everyone
-- may join, modelled as an ordinary Dōjō at scope=global (there is no separate
-- "Collective" concept). Idempotent: ON CONFLICT DO NOTHING never overwrites an
-- existing row, so it is safe to run on every dojo-scope deploy. Called after
-- the dojo tables are applied (the dojo service will invoke it on boot).
create or replace procedure dojo.seed_global_dojo()
language plpgsql
set search_path = dojo, extensions
as $$
begin
  insert into dojo.tenants (
      key, origin, org, dojo, scope, name, dojo_url, self_hosted
  )
  values (
      'org/global-dojo'
    , 'org'
    , 'global-dojo'
    , null
    , 'global'
    , 'Global Dōjō'
    , 'dojo.sensei-hq.org/org/global-dojo'
    , false
  )
  on conflict (key) do nothing;
end;
$$;
