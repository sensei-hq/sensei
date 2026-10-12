-- Adopting rule packs as the signed-in person (#236 step 3).
--
-- A person adopts a library pack into their OWN user namespace and reads their
-- own adoptions. dojo.set_pack_adoption used to take the user as an ARGUMENT —
-- safe only while the service role alone could call it; called from a session it
-- would let anyone adopt or drop packs in anyone's namespace. It now derives the
-- user from the session (dojo.current_principal_id) and refuses without one.
-- dojo.my_adopted_pack_slugs() reads the caller's adoptions; the cross-namespace
-- dojo.pack_adoption view stays the service role's.
--
-- WHAT BREAKS THIS TEST: a set_pack_adoption that still accepts a user id;
-- one that adopts with no session; a my_adopted_pack_slugs without the
-- principal pin; a session grant on dojo.pack_adoption.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000141', 'a1000000-0000-4000-8000-000000000141', 'ola'),
  ('a0000000-0000-4000-8000-000000000142', 'a1000000-0000-4000-8000-000000000142', 'bob');

set local role authenticated;

do $$
declare
    got text;
begin
    if to_regprocedure('dojo.set_pack_adoption(text, text, text, boolean)') is not null then
        raise exception 'set_pack_adoption: the signature that takes a user id must be gone';
    end if;

    -- No session: nobody to adopt for.
    perform set_config('request.jwt.claims', '', true);
    begin
        perform dojo.set_pack_adoption('clean-code', 'nobody', true);
        raise exception 'set_pack_adoption: adopted with no session';
    exception when insufficient_privilege then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000141"}', true);
    if dojo.set_pack_adoption('clean-code', 'Ola', true) is distinct from true then
        raise exception 'set_pack_adoption: ola should adopt clean-code';
    end if;
    if dojo.set_pack_adoption('ztest-no-such-pack', 'Ola', true) is distinct from false then
        raise exception 'set_pack_adoption: an unknown pack must answer false';
    end if;
    select coalesce(string_agg(s, ',' order by s), '') into got from dojo.my_adopted_pack_slugs() s;
    if got <> 'clean-code' then raise exception 'my_adopted_pack_slugs: ola should see clean-code, saw [%]', got; end if;

    begin
        select count(*)::text into got from dojo.pack_adoption;
        raise exception 'pack_adoption: a session read every namespace''s adoptions';
    exception when insufficient_privilege then
        -- Refused AT THE VIEW: a refusal on a base table holds only until a
        -- session is granted that table.
        if sqlerrm not like '%view pack_adoption%' then
            raise exception 'pack_adoption: refused for the wrong reason: %', sqlerrm;
        end if;
    end;

    -- bob neither sees nor undoes ola's adoption.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000142"}', true);
    select coalesce(string_agg(s, ',' order by s), '') into got from dojo.my_adopted_pack_slugs() s;
    if got <> '' then raise exception 'my_adopted_pack_slugs: bob saw ola''s adoptions [%]', got; end if;
    perform dojo.set_pack_adoption('clean-code', 'Bob', false);

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000141"}', true);
    select coalesce(string_agg(s, ',' order by s), '') into got from dojo.my_adopted_pack_slugs() s;
    if got <> 'clean-code' then raise exception 'set_pack_adoption: bob''s un-adopt reached ola''s namespace [%]', got; end if;
end $$;

rollback;
