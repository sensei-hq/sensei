-- Every SECURITY DEFINER function a signed-in person can call answers ONLY for
-- the caller (user, 2026-10-27; the Supabase advisor warns on each of them,
-- 46 in production, one of them a stale copy since dropped, by design: they are the dōjō's API and the helpers its
-- RLS policies call, and each must gate on the caller itself).
--
-- Held here: bob — signed in, a member of nothing — calls each one with ALICE's
-- dōjō, workspace, repository, team, membership and keys. Every predicate
-- answers false; every read returns nothing of hers; every write is refused or
-- changes nothing (a fingerprint of every dojo table, taken as the owner before
-- and after, is unchanged). The list of functions is complete: one added
-- without a line here fails the first check, so a new one cannot go unchecked.
--
-- WHAT BREAKS THIS TEST: a definer function that trusts its arguments instead
-- of the caller; a new definer function granted to `authenticated` and not
-- listed below.

begin;

-- ── Alice's dōjō, with something behind every function ─────────────────────
insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-0000000000a1', 'a1000000-0000-4000-8000-0000000000a1', 'alice'),
  ('a0000000-0000-4000-8000-0000000000b1', 'a1000000-0000-4000-8000-0000000000b1', 'bob'),
  ('a0000000-0000-4000-8000-0000000000c1', 'a1000000-0000-4000-8000-0000000000c1', 'carol');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000000a1', 'organization/ztest-definer', 'organization',
   'ztest-definer', 'definer fixture', 'dojo.sensei-hq.org/organization/ztest-definer');

insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via) values
  ('e0000000-0000-4000-8000-0000000000a1', 'b0000000-0000-4000-8000-0000000000a1',
   'a0000000-0000-4000-8000-0000000000a1', 'admin', 'employer', 'github_oauth');

insert into dojo.workspaces (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-0000000000a1', 'b0000000-0000-4000-8000-0000000000a1', 'ztest-definer-w', 'W');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-0000000000a1', 'b0000000-0000-4000-8000-0000000000a1',
   'github.com/ztest-definer/r', 'r', 'github');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-0000000000a1', '90000000-0000-4000-8000-0000000000a1',
   'b0000000-0000-4000-8000-0000000000a1');
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-0000000000a1', 'a0000000-0000-4000-8000-0000000000a1', 'admin', 'admin');
insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-0000000000a1', 'b0000000-0000-4000-8000-0000000000a1', 'T');
insert into dojo.team_members (team_id, principal_id) values
  ('d0000000-0000-4000-8000-0000000000a1', 'a0000000-0000-4000-8000-0000000000a1');
insert into dojo.workspace_viewers (workspace_id, principal_id) values
  ('c0000000-0000-4000-8000-0000000000a1', 'a0000000-0000-4000-8000-0000000000c1');
insert into dojo.dojo_viewers (tenant_id, principal_id) values
  ('b0000000-0000-4000-8000-0000000000a1', 'a0000000-0000-4000-8000-0000000000c1');
insert into dojo.relay_sessions (membership_id, run_id, workspace_id, status, started_at) values
  ('e0000000-0000-4000-8000-0000000000a1', gen_random_uuid(),
   'c0000000-0000-4000-8000-0000000000a1', 'running', now());
insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value)
select 'b0000000-0000-4000-8000-0000000000a1', '90000000-0000-4000-8000-0000000000a1',
       m.id, 'repo', current_date, 0.5
  from sensei.metrics m order by m.key limit 1;

-- ── The list is complete ──────────────────────────────────────────────────
do $$
declare
    listed text[] := array[
      'add_team_member','administers_repository','administers_team','administers_workspace',
      'attach_run_to_workspace','can_read_repository','can_read_repository_metric','can_read_workspace',
      'clear_repository_role_override','create_team','current_principal_id','delete_team',
      'file_graph_slices','file_metric_rows','file_repository_workspace','grant_workspace_viewer',
      'grant_workspace_viewer_to_team','graph_file_hashes','is_dojo_viewer','is_member',
      'is_workspace_viewer','leads_team_member','my_adopted_pack_slugs','my_graph_branches',
      'my_repositories','owns_membership','peer_run_status','register_repository',
      'remove_dojo_viewer','remove_team_member','rename_team','repo_role_at_least',
      'revoke_workspace_viewer','set_graph_branch','set_metric_activation','set_pack_adoption',
      'set_repository_election','set_repository_role_override','team_metric_totals',
      'team_tenant_member','tenant_live_session_count','tenant_pack_rules','tenant_role_at_least',
      'tenant_shared_rules','workspace_in_my_dojo_or_none'];
    unlisted text;
    gone text;
begin
    select string_agg(distinct p.proname, ', ') into unlisted
      from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     where n.nspname = 'dojo' and p.prosecdef
       and has_function_privilege('authenticated', p.oid, 'execute')
       and p.proname <> all (listed);
    if unlisted is not null then
        raise exception 'definer functions a signed-in person can call, not checked here: %', unlisted;
    end if;
    select string_agg(l, ', ') into gone from unnest(listed) l
     where not exists (select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace
                        where n.nspname = 'dojo' and p.proname = l);
    if gone is not null then raise exception 'listed but no longer defined: %', gone; end if;
end
$$;

-- ── A fingerprint of every dojo table, as the owner ───────────────────────
create temp table definer_snap (phase text, tbl text, h text);
create or replace function pg_temp.snap(p_phase text) returns void language plpgsql as $$
declare t record; h text;
begin
    for t in select c.relname from pg_class c join pg_namespace n on n.oid = c.relnamespace
              where n.nspname = 'dojo' and c.relkind = 'r' loop
        execute format('select md5(coalesce(string_agg(x::text, ''|'' order by x::text), '''')) from dojo.%I x', t.relname)
           into h;
        insert into definer_snap values (p_phase, t.relname, h);
    end loop;
end $$;
select pg_temp.snap('before');

create temp table definer_leaks (fn text, what text);
grant all on definer_leaks to authenticated;

-- ── Bob, signed in, a member of nothing ───────────────────────────────────
set local role authenticated;
select set_config('request.jwt.claims',
                  '{"sub":"a1000000-0000-4000-8000-0000000000b1","role":"authenticated"}', true);

do $$
declare
    t constant text := '''b0000000-0000-4000-8000-0000000000a1''::uuid';
    w constant text := '''c0000000-0000-4000-8000-0000000000a1''::uuid';
    r constant text := '''90000000-0000-4000-8000-0000000000a1''::uuid';
    tm constant text := '''d0000000-0000-4000-8000-0000000000a1''::uuid';
    alice constant text := '''a0000000-0000-4000-8000-0000000000a1''::uuid';
    bob constant text := '''a0000000-0000-4000-8000-0000000000b1''::uuid';
    carol constant text := '''a0000000-0000-4000-8000-0000000000c1''::uuid';
    mid constant text := '''e0000000-0000-4000-8000-0000000000a1''::uuid';
    rkey constant text := '''github.com/ztest-definer/r''';
    rows constant text := format('''[{"repository_id":"90000000-0000-4000-8000-0000000000a1","metric_id":"%s","computed_on":"2026-01-02","value":9}]''::jsonb',
                                 (select id from sensei.metrics order by key limit 1));
    call text;
    b boolean;
    n bigint;
    u uuid;
begin
    -- Predicates: false for everything of alice's.
    foreach call in array array[
        format('dojo.administers_repository(%s)', r),
        format('dojo.administers_team(%s)', tm),
        format('dojo.administers_workspace(%s)', w),
        format('dojo.can_read_repository(%s, %s)', r, t),
        format('dojo.can_read_repository_metric(%s, ''repo'', %s, %s)', r, alice, t),
        format('dojo.can_read_workspace(%s)', w),
        format('dojo.is_dojo_viewer(%s)', t),
        format('dojo.is_member(%s)', t),
        format('dojo.is_workspace_viewer(%s)', w),
        format('dojo.leads_team_member(%s, %s)', alice, t),
        format('dojo.owns_membership(%s)', mid),
        format('dojo.repo_role_at_least(%s, ''contributor'')', r),
        format('dojo.team_tenant_member(%s)', tm),
        format('dojo.tenant_role_at_least(%s, ''contributor'')', t),
        format('dojo.workspace_in_my_dojo_or_none(%s)', w)] loop
        begin
            execute 'select ' || call into b;
            if b then insert into definer_leaks values (call, 'answered true'); end if;
        exception when others then null; -- a refusal answers nothing
        end;
    end loop;

    -- Reads: nothing of alice's.
    foreach call in array array[
        format('dojo.graph_file_hashes(%s, ''main'')', r),
        'dojo.my_adopted_pack_slugs()',
        'dojo.my_graph_branches()',
        'dojo.my_repositories()',
        format('dojo.peer_run_status(%s)', t),
        format('dojo.team_metric_totals(%s, array(select key from sensei.metrics), ''2000-01-01'')', t),
        format('dojo.tenant_pack_rules(%s, null)', t),
        format('dojo.tenant_shared_rules(%s, 0)', t)] loop
        begin
            execute format('select count(*) from %s', call) into n;
            if n > 0 then insert into definer_leaks values (call, n || ' row(s)'); end if;
        exception when others then null;
        end;
    end loop;
    begin
        execute format('select dojo.tenant_live_session_count(%s, ''2000-01-01'')', t) into n;
        if n > 0 then insert into definer_leaks values ('tenant_live_session_count', n || ' session(s)'); end if;
    exception when others then null;
    end;
    select dojo.current_principal_id() into u;
    if u is distinct from 'a0000000-0000-4000-8000-0000000000b1' then
        insert into definer_leaks values ('current_principal_id', 'answered ' || coalesce(u::text, 'null'));
    end if;

    -- Writes at alice's things: refused, or a no-op (the fingerprint says which).
    foreach call in array array[
        format('dojo.add_team_member(%s, %s)', tm, bob),
        format('dojo.attach_run_to_workspace(%s, ''ztest-definer-w'', ''{}''::jsonb)', t),
        format('dojo.clear_repository_role_override(%s, %s)', r, alice),
        format('dojo.create_team(%s, ''bob''''s'')', t),
        format('dojo.delete_team(%s)', tm),
        format('dojo.file_graph_slices(%s, ''{}''::jsonb, ''[]''::jsonb, ''{}''::jsonb)', r),
        format('dojo.file_metric_rows(%s)', rows),
        format('dojo.file_repository_workspace(%s, ''bobs'', ''bobs'')', r),
        format('dojo.grant_workspace_viewer(%s, %s)', w, bob),
        format('dojo.grant_workspace_viewer_to_team(%s, %s)', w, tm),
        format('dojo.register_repository(%s, ''github'', ''ztest-definer'', ''r'', ''https://github.com/ztest-definer/r'', null)', rkey),
        format('dojo.remove_dojo_viewer(%s, %s)', t, carol),
        format('dojo.remove_team_member(%s, %s)', tm, alice),
        format('dojo.rename_team(%s, ''bob was here'')', tm),
        format('dojo.revoke_workspace_viewer(%s, %s)', w, carol),
        format('dojo.set_graph_branch(%s, ''main'', true, null)', rkey),
        format('dojo.set_metric_activation(%s, (select key from sensei.metrics order by key limit 1), false)', rkey),
        format('dojo.set_pack_adoption(''ztest-no-such-pack'', ''bob'', true)'),
        format('dojo.set_repository_election(%s, false)', rkey),
        format('dojo.set_repository_role_override(%s, %s, ''admin'')', r, bob)] loop
        begin
            execute 'select * from ' || call;
        exception when others then null; -- refused: its effects are rolled back
        end;
    end loop;
end
$$;

reset role;
select pg_temp.snap('after');

-- ── Positive controls: the same questions CAN answer yes ───────────────────
-- Bob's "no" means something only if the fixture gives a "yes" to someone: a
-- predicate that answers false to everyone would pass above and prove nothing.
-- Alice (admin of the dōjō) and carol (its viewer, and the workspace's).
set local role authenticated;
do $$
declare
    who record;
    call text;
    b boolean;
    n bigint;
    t constant text := '''b0000000-0000-4000-8000-0000000000a1''::uuid';
    w constant text := '''c0000000-0000-4000-8000-0000000000a1''::uuid';
    r constant text := '''90000000-0000-4000-8000-0000000000a1''::uuid';
    tm constant text := '''d0000000-0000-4000-8000-0000000000a1''::uuid';
    alice constant text := '''a0000000-0000-4000-8000-0000000000a1''::uuid';
    mid constant text := '''e0000000-0000-4000-8000-0000000000a1''::uuid';
begin
    for who in select * from (values
        ('a1000000-0000-4000-8000-0000000000a1', array[
            format('dojo.administers_repository(%s)', r), format('dojo.administers_team(%s)', tm),
            format('dojo.administers_workspace(%s)', w), format('dojo.can_read_repository(%s, %s)', r, t),
            format('dojo.can_read_repository_metric(%s, ''repo'', %s, %s)', r, alice, t),
            format('dojo.can_read_workspace(%s)', w), format('dojo.is_member(%s)', t),
            format('dojo.leads_team_member(%s, %s)', alice, t), format('dojo.owns_membership(%s)', mid),
            format('dojo.repo_role_at_least(%s, ''contributor'')', r), format('dojo.team_tenant_member(%s)', tm),
            format('dojo.tenant_role_at_least(%s, ''contributor'')', t),
            format('dojo.workspace_in_my_dojo_or_none(%s)', w)],
         array[format('dojo.peer_run_status(%s)', t), 'dojo.my_repositories()']),
        ('a1000000-0000-4000-8000-0000000000c1', array[
            format('dojo.is_dojo_viewer(%s)', t), format('dojo.is_workspace_viewer(%s)', w)],
         array[]::text[])) as v(sub, yes, rows) loop
        perform set_config('request.jwt.claims', format('{"sub":"%s","role":"authenticated"}', who.sub), true);
        foreach call in array who.yes loop
            execute 'select ' || call into b;
            if b is not true then raise exception 'control: % answers % for its owner — the check above proves nothing', call, b; end if;
        end loop;
        foreach call in array who.rows loop
            execute format('select count(*) from %s', call) into n;
            if n = 0 then raise exception 'control: % returns nothing for its owner — the check above proves nothing', call; end if;
        end loop;
    end loop;
end
$$;
reset role;

do $$
declare
    leaks text;
    changed text;
begin
    select string_agg(fn || ': ' || what, '; ') into leaks from definer_leaks;
    if leaks is not null then raise exception 'a non-member reached alice''s dōjō: %', leaks; end if;
    select string_agg(a.tbl, ', ' order by a.tbl) into changed
      from definer_snap a join definer_snap b on b.tbl = a.tbl and b.phase = 'after'
     where a.phase = 'before' and a.h is distinct from b.h;
    if changed is not null then
        raise exception 'a non-member''s calls changed: %', changed;
    end if;
end
$$;

rollback;
