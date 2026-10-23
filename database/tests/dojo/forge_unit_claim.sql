-- Claiming an organization's dōjō (#308 R5a, D20).
--
-- An org dōjō is created by whoever signs in first, who may be a plain member, so
-- its existence proves nothing about who owns the org. A GitHub org ADMIN claims
-- it: the server proves the admin with the person's own token, then calls
-- dojo.claim_forge_unit, which records the claim on the tenant (claimed_at,
-- claimed_by — the claim gates billing and private sync for the whole dōjō, so it
-- lives on the tenant, not on one of its connections), re-proves the connection
-- (verified_at) and writes the audit row in the same transaction.
--
-- WHAT BREAKS THIS TEST: a signed-in caller claiming directly; a second claim
-- rewriting claimed_by; a claim with no audit row, or an audit row per re-proof;
-- verified_at not set; a claim on an org nobody connected, or on a personal dōjō.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000501', 'a1000000-0000-4000-8000-000000000501', 'first-admin'),
  ('a0000000-0000-4000-8000-000000000502', 'a1000000-0000-4000-8000-000000000502', 'second-admin');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000050a', 'organization/ztest-claim-org', 'organization', 'ztest-claim-org', 'Org', 'x'),
  ('b0000000-0000-4000-8000-00000000050b', 'personal/ztest-claim-me', 'personal', 'ztest-claim-me', 'Me', 'x');
insert into dojo.tenant_connections (tenant_id, provider, external_id, external_slug, connected_by) values
  ('b0000000-0000-4000-8000-00000000050a', 'github', 'ztest-gh-501', 'ztest-claim-org', 'a0000000-0000-4000-8000-000000000501'),
  ('b0000000-0000-4000-8000-00000000050b', 'github', 'ztest-gh-502', 'ztest-claim-me', 'a0000000-0000-4000-8000-000000000501');

-- A signed-in caller cannot claim: the admin proof is the server's, made against
-- the forge with the person's own token.
set local role authenticated;
do $$
begin
    perform dojo.claim_forge_unit('a0000000-0000-4000-8000-000000000501', 'github', 'ztest-gh-501');
    raise exception 'a signed-in caller claimed a dōjō';
exception when insufficient_privilege then null;
end $$;
reset role;

set local role service_role;
do $$
declare
    r record;
begin
    select * into r from dojo.claim_forge_unit('a0000000-0000-4000-8000-000000000501', 'github', 'ztest-gh-501');
    if r.tenant_id <> 'b0000000-0000-4000-8000-00000000050a' or r.claimed_by <> 'a0000000-0000-4000-8000-000000000501'
       or r.claimed_at is null or not r.claimed_now then
        raise exception 'claim: the first admin should claim the org dōjō, got %', r;
    end if;

    -- First claim wins: a second admin re-proves the connection but does not
    -- rewrite who established the claim.
    select * into r from dojo.claim_forge_unit('a0000000-0000-4000-8000-000000000502', 'github', 'ztest-gh-501');
    if r.claimed_by <> 'a0000000-0000-4000-8000-000000000501' or r.claimed_now then
        raise exception 'claim: a second claim rewrote the first, got %', r;
    end if;

    begin
        perform dojo.claim_forge_unit('a0000000-0000-4000-8000-000000000501', 'github', 'ztest-gh-none');
        raise exception 'claim: an org nobody connected was claimed';
    exception when no_data_found then null;
    end;

    begin
        perform dojo.claim_forge_unit('a0000000-0000-4000-8000-000000000501', 'github', 'ztest-gh-502');
        raise exception 'claim: a personal dōjō was claimed';
    exception when invalid_parameter_value then null;
    end;
end $$;
reset role;

do $$
begin
    if (select verified_at from dojo.tenant_connections where external_id = 'ztest-gh-501') is null then
        raise exception 'claim: the connection was not re-proved (verified_at)';
    end if;
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-00000000050a' and action = 'dojo_claimed'
           and actor_id = 'a0000000-0000-4000-8000-000000000501'
           and detail ->> 'external_slug' = 'ztest-claim-org') <> 1 then
        raise exception 'claim: expected exactly one dojo_claimed audit row by the first admin, found %',
            (select count(*) from dojo.audit_events where tenant_id = 'b0000000-0000-4000-8000-00000000050a');
    end if;
    if (select count(*) from dojo.audit_events where tenant_id = 'b0000000-0000-4000-8000-00000000050a') <> 1 then
        raise exception 'claim: a re-proof wrote an audit row';
    end if;
    if (select claimed_at from dojo.tenants where id = 'b0000000-0000-4000-8000-00000000050b') is not null then
        raise exception 'claim: the personal dōjō was marked claimed';
    end if;
end $$;

rollback;
