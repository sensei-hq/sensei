-- The per-person forge mirror (#307 A2c, D5). A member's own token tells the dōjō
-- their permission on every repository they can see; provisioning passes those
-- for one dōjō: `[{"repo_key": "github.com/org/r", "permission": "write"}, …]`.
-- It sets this person's forge roles on the dōjō's repositories it lists and clears
-- their forge roles on the dōjō's repositories it no longer lists. It never
-- touches another person, another dōjō, or an admin override.
--
-- WHAT BREAKS THIS TEST: touching someone else's role; touching another dōjō's
-- repository; keeping a forge role the forge no longer reports; clearing an
-- override; a key that is not the dōjō's creating a role; a signed-in caller
-- running it.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000331', 'a1000000-0000-4000-8000-000000000331', 'mo'),
  ('a0000000-0000-4000-8000-000000000332', 'a1000000-0000-4000-8000-000000000332', 'kit');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000033a', 'organization/ztest-rm-t', 'organization', 'ztest-rm-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000033b', 'organization/ztest-rm-u', 'organization', 'ztest-rm-u', 'U', 'x');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-000000000331', 'b0000000-0000-4000-8000-00000000033a', 'github.com/ztest-rm/a', 'a', 'github'),
  ('90000000-0000-4000-8000-000000000332', 'b0000000-0000-4000-8000-00000000033a', 'github.com/ztest-rm/b', 'b', 'github'),
  ('90000000-0000-4000-8000-000000000333', 'b0000000-0000-4000-8000-00000000033a', 'github.com/ztest-rm/c', 'c', 'github'),
  ('90000000-0000-4000-8000-000000000334', 'b0000000-0000-4000-8000-00000000033b', 'github.com/ztest-rm/u-only', 'u-only', 'github');
-- Before: mo writes b and c (c overridden to maintainer by an admin); kit writes a;
-- mo also has a role on u-only, a repository in the OTHER dōjō U (which mo's
-- token, read for T, does not list).
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role, overridden, override_role) values
  ('90000000-0000-4000-8000-000000000332', 'a0000000-0000-4000-8000-000000000331', 'write', 'contributor', false, null),
  ('90000000-0000-4000-8000-000000000333', 'a0000000-0000-4000-8000-000000000331', 'write', 'contributor', true,  'maintainer'),
  ('90000000-0000-4000-8000-000000000331', 'a0000000-0000-4000-8000-000000000332', 'write', 'contributor', false, null),
  ('90000000-0000-4000-8000-000000000334', 'a0000000-0000-4000-8000-000000000331', 'read',  'contributor', false, null);

-- A signed-in caller cannot run it.
set local role authenticated;
do $$
begin
    perform dojo.mirror_principal_repository_roles('a0000000-0000-4000-8000-000000000331', 'b0000000-0000-4000-8000-00000000033a', '[]'::jsonb);
    raise exception 'a signed-in caller ran the per-person mirror';
exception when insufficient_privilege then null;
end $$;
reset role;

-- mo's token now says: maintain on a, nothing on b or c; and a key the dōjō
-- does not hold.
set local role service_role;
select dojo.mirror_principal_repository_roles(
  'a0000000-0000-4000-8000-000000000331', 'b0000000-0000-4000-8000-00000000033a',
  '[{"repo_key":"github.com/ztest-rm/a","permission":"maintain"},
    {"repo_key":"github.com/ztest-rm/elsewhere","permission":"admin"}]'::jsonb);
reset role;

do $$
declare
    mo  constant uuid := 'a0000000-0000-4000-8000-000000000331';
    kit constant uuid := 'a0000000-0000-4000-8000-000000000332';
    rr record;
    n int;
begin
    select * into rr from dojo.repository_roles where repository_id = '90000000-0000-4000-8000-000000000331' and principal_id = mo;
    if rr.forge_role is distinct from 'maintainer' or rr.forge_permission is distinct from 'maintain' then
        raise exception 'mo on a: expected forge maintain/maintainer, got %/%', rr.forge_permission, rr.forge_role;
    end if;
    select * into rr from dojo.repository_roles where repository_id = '90000000-0000-4000-8000-000000000332' and principal_id = mo;
    if rr.forge_role is not null then raise exception 'mo kept a forge role on b the forge no longer reports'; end if;
    select * into rr from dojo.repository_roles where repository_id = '90000000-0000-4000-8000-000000000333' and principal_id = mo;
    if not rr.overridden or rr.override_role is distinct from 'maintainer' then raise exception 'the mirror touched an override on c'; end if;
    select * into rr from dojo.repository_roles where repository_id = '90000000-0000-4000-8000-000000000331' and principal_id = kit;
    if rr.forge_role is distinct from 'contributor' then raise exception 'the mirror touched someone else (kit on a)'; end if;
    select * into rr from dojo.repository_roles where repository_id = '90000000-0000-4000-8000-000000000334' and principal_id = mo;
    if rr.forge_role is distinct from 'contributor' then raise exception 'the mirror touched another dōjō''s repository'; end if;
    select count(*) into n from dojo.repository_roles rr2 join dojo.repositories r on r.id = rr2.repository_id
     where r.repo_key = 'github.com/ztest-rm/elsewhere';
    if n <> 0 then raise exception 'a key the dōjō does not hold created a role'; end if;
end $$;

rollback;
