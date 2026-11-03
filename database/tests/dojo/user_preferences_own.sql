-- A person's preferences (#133: the glyph mode, synced between the app and the
-- dōjō) are theirs alone: read and written as the signed-in person, never
-- another's. The daemon writes them through the same route as its person.
--
-- WHAT BREAKS THIS TEST: a read policy wider than own; a missing with-check (a
-- row re-pointed at another person); a glyph mode outside kanji/icon.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000061', 'a1000000-0000-4000-8000-000000000061', 'alice'),
  ('a0000000-0000-4000-8000-000000000062', 'a1000000-0000-4000-8000-000000000062', 'bob');
insert into dojo.user_preferences (user_id, glyphs, glyphs_set_at) values
  ('a0000000-0000-4000-8000-000000000062', 'icon', now());

set local role authenticated;

do $$
declare
    n int;
    alice constant uuid := 'a0000000-0000-4000-8000-000000000061';
    bob   constant uuid := 'a0000000-0000-4000-8000-000000000062';
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000061"}', true);

    select count(*) into n from dojo.user_preferences;
    if n <> 0 then raise exception 'prefs: alice saw % row(s) of someone else''s', n; end if;

    -- The route's upsert, as PostgREST sends it.
    insert into dojo.user_preferences (user_id, glyphs, glyphs_set_at, updated_at)
    values (alice, 'icon', now(), now())
    on conflict (user_id) do update set user_id = excluded.user_id, glyphs = excluded.glyphs,
        glyphs_set_at = excluded.glyphs_set_at, updated_at = excluded.updated_at;
    if (select glyphs from dojo.user_preferences where user_id = alice) <> 'icon' then
        raise exception 'prefs: alice''s write did not land';
    end if;

    begin
        insert into dojo.user_preferences (user_id, glyphs) values (bob, 'kanji');
        raise exception 'prefs: alice wrote prefs for bob';
    exception
        when insufficient_privilege then
            if sqlerrm not like '%row-level security%' then raise exception 'refused for the wrong reason: %', sqlerrm; end if;
        when unique_violation then raise exception 'prefs: alice reached bob''s row';
    end;

    begin
        update dojo.user_preferences set user_id = bob where user_id = alice;
        raise exception 'prefs: alice handed her row to bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 're-point refused for the wrong reason: %', sqlerrm; end if;
    end;

    begin
        update dojo.user_preferences set glyphs = 'emoji' where user_id = alice;
        raise exception 'prefs: a glyph mode outside kanji/icon was stored';
    exception when invalid_text_representation then null;
    end;
end $$;

rollback;
