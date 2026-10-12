-- Who issues and reads invites (#236 step 3; docs/spec/dojo/access-matrix.md
-- "Tenancy, identity, billing": invites R/W tenant for an admin).
--
-- A tenant admin issues invites in their own name and reads the tenant's. An
-- invite is accepted by the system (the joining person has no membership yet,
-- and the token is the authority), so no session updates one.
--
-- The token is a bearer: no session reads it, the issuing admin included (they
-- are handed the token the server minted).
--
-- WHAT BREAKS THIS TEST: a policy below admin or without the tenant; an
-- invited_by that need not be the caller; any UPDATE grant; a SELECT grant on
-- the token.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-0000000000c1', 'a1000000-0000-4000-8000-0000000000c1', 'ada'),
  ('a0000000-0000-4000-8000-0000000000c2', 'a1000000-0000-4000-8000-0000000000c2', 'lea'),
  ('a0000000-0000-4000-8000-0000000000c3', 'a1000000-0000-4000-8000-0000000000c3', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000000ca', 'organization/ztest-inv-t', 'organization',
   'ztest-inv-t', 'invites fixture T', 'dojo.sensei-hq.org/organization/ztest-inv-t'),
  ('b0000000-0000-4000-8000-0000000000cb', 'organization/ztest-inv-u', 'organization',
   'ztest-inv-u', 'invites fixture U', 'dojo.sensei-hq.org/organization/ztest-inv-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-0000000000ca', 'a0000000-0000-4000-8000-0000000000c1', 'admin', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-0000000000ca', 'a0000000-0000-4000-8000-0000000000c2', 'lead',  'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-0000000000cb', 'a0000000-0000-4000-8000-0000000000c3', 'admin', 'employer', 'github_oauth');

insert into dojo.invites (id, tenant_id, email, kind, token, invited_by, expires_at) values
  ('dc000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000ca', 'ztest-1@example.com',
   'employer', 'ztest-tok-1', 'a0000000-0000-4000-8000-0000000000c1', now() + interval '7 days'),
  ('dc000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000cb', 'ztest-2@example.com',
   'employer', 'ztest-tok-2', 'a0000000-0000-4000-8000-0000000000c3', now() + interval '7 days');

set local role authenticated;

do $$
declare
    n int;
    got text;
    t   constant uuid := 'b0000000-0000-4000-8000-0000000000ca';
    u   constant uuid := 'b0000000-0000-4000-8000-0000000000cb';
    ada constant uuid := 'a0000000-0000-4000-8000-0000000000c1';
    lea constant uuid := 'a0000000-0000-4000-8000-0000000000c2';
begin
    -- lea (lead): no invites, read or issued.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000c2"}', true);
    select count(*) into n from dojo.invites where email like 'ztest-%';
    if n <> 0 then raise exception 'invites: a lead saw % invite(s)', n; end if;
    begin
        insert into dojo.invites (tenant_id, email, role, kind, token, invited_by, expires_at)
        values (t, 'x@example.com', 'contributor', 'employer', 'ztest-tok-lea', lea, now() + interval '1 day');
        raise exception 'invites: a lead issued an invite';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'lead invite refused for the wrong reason: %', sqlerrm; end if;
    end;

    -- ada (admin of T): T's invites; issues in her own name, in T only.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000c1"}', true);
    select coalesce(string_agg(split_part(email, '@', 1), ',' order by email), '') into got
      from dojo.invites where email like 'ztest-%';
    if got <> 'ztest-1' then raise exception 'invites: ada should see T''s only, saw [%]', got; end if;
    begin
        select token into got from dojo.invites where email = 'ztest-1@example.com';
        raise exception 'invites: an admin read an invite''s token';
    exception when insufficient_privilege then null;
    end;

    insert into dojo.invites (tenant_id, email, role, kind, token, invited_by, expires_at)
    values (t, 'new@example.com', 'contributor', 'employer', 'ztest-tok-ada', ada, now() + interval '7 days');
    begin
        insert into dojo.invites (tenant_id, email, role, kind, token, invited_by, expires_at)
        values (t, 'x@example.com', 'contributor', 'employer', 'ztest-tok-forged', lea, now() + interval '7 days');
        raise exception 'invites: ada issued an invite in lea''s name';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'forged inviter refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.invites (tenant_id, email, role, kind, token, invited_by, expires_at)
        values (u, 'x@example.com', 'contributor', 'employer', 'ztest-tok-u', ada, now() + interval '7 days');
        raise exception 'invites: ada issued an invite into another tenant';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'cross-tenant invite refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        update dojo.invites set accepted_at = now() where email = 'ztest-1@example.com';
        raise exception 'invites: an admin accepted an invite from their session';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'invites: update must be refused by the grant'; end if;
    end;
end $$;

rollback;
