-- What a dōjō is (#307 D11, user 2026-10-21): the tenant — the owner — and only
-- ever an organization's or a person's. `organization/<login>` is provisioned
-- from a GitHub org the person belongs to; `personal/<login>` from the person.
-- There is no hand-made dōjō: the `team` origin, `team/<slug>` keys and the
-- dojo.create_dojo function that made them are gone.
--
-- WHAT BREAKS THIS TEST: any tenant origin besides personal and organization;
-- dojo.create_dojo existing; a self_hosted flag on a tenant (D17); the retired spellings ('org', 'team', an `org`
-- column) accepted again; a person and an org of one name colliding.

begin;

do $$
declare got text;
begin
    select string_agg(e.enumlabel, ',' order by e.enumsortorder) into got
      from pg_enum e join pg_type t on t.oid = e.enumtypid
      join pg_namespace n on n.oid = t.typnamespace
     where n.nspname = 'dojo' and t.typname = 'tenant_origin';
    if got is distinct from 'personal,organization' then
        raise exception 'dojo.tenant_origin is [%], expected [personal,organization]', got;
    end if;

    -- D17: no self-hosting product, so a tenant carries no self-hosted flag.
    if exists (select 1 from information_schema.columns
                where table_schema = 'dojo' and table_name = 'tenants' and column_name = 'self_hosted') then
        raise exception 'dojo.tenants still has self_hosted (D17)';
    end if;

    if exists (select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace
                where n.nspname = 'dojo' and p.proname = 'create_dojo') then
        raise exception 'dojo.create_dojo still exists — there is no hand-made dōjō';
    end if;
end $$;

-- ── the retired spellings stay rejected ─────────────────────────────────────
do $$
declare retired text;
begin
    foreach retired in array array['org', 'team'] loop
        begin
            execute format($q$insert into dojo.tenants (key, origin, slug, name, dojo_url)
                              values (%L, %L, 'ztest-legacy', 'Legacy', 'u')$q$, retired || '/ztest-legacy', retired);
            raise exception '% is still an accepted tenant_origin', retired;
        exception when invalid_text_representation then
            null;
        end;
    end loop;
    begin
        execute $q$insert into dojo.tenants (key, origin, org, name, dojo_url)
                   values ('organization/ztest-legacy', 'organization', 'ztest-legacy', 'Legacy', 'u')$q$;
        raise exception 'dojo.tenants still accepts an "org" column — the rename did not happen.';
    exception when undefined_column then
        null;
    end;
end $$;

-- ── a person and an organization of one name stay apart (§IV.7 / F1) ────────
-- The origin prefix is what keeps the person `acme` and the GitHub org `acme`
-- from colliding.
insert into dojo.tenants (key, origin, slug, name, dojo_url) values
  ('personal/ztest-acme', 'personal', 'ztest-acme', 'Acme''s Dōjō', 'dojo.sensei-hq.org/personal/ztest-acme'),
  ('organization/ztest-acme', 'organization', 'ztest-acme', 'Acme (GitHub)', 'dojo.sensei-hq.org/organization/ztest-acme');

do $$
declare n int;
begin
    select count(*) into n from dojo.tenants where slug = 'ztest-acme';
    if n <> 2 then
        raise exception 'personal/ and organization/ztest-acme should coexist as 2 rows, found %.', n;
    end if;
end $$;

rollback;
