set search_path to sensei, extensions;

-- The rule pack of a tech stack, made on first use (#318 R6, D29).
--
-- A stack is not a governance level: it is a tech stack's rules, packed and
-- applied at a level — the repositories where it is detected (D33). This is
-- that pack: `stack:<slug>`, area `tech_stack`, a library pack (no owner).
-- `scope` local, `origin` learned: sensei made it from what it detected on this
-- machine, and it never leaves it on its own. The `stack:` prefix is the
-- ladder's own `kind:slug` notation, and no curated library slug has a colon —
-- the curated `stack-templates` pack shows why a hyphen would not do — so a
-- stack's own rules (converted from the retired `technology` rung) never land
-- in a pack the seed import may replace.
create or replace function stack_pack(p_stack_slug text, p_stack_name text)
returns uuid
language plpgsql
volatile
as $$
declare
    v_name constant text := coalesce(nullif(p_stack_name, ''), p_stack_slug);
    v_id   uuid;
begin
    if coalesce(p_stack_slug, '') = '' then
        raise exception 'a stack pack needs a stack slug' using errcode = 'invalid_parameter_value';
    end if;
    insert into sensei.rule_packs (slug, name, kanji, area, attribution, summary,
                                   published_by, scope, origin)
    values ('stack:' || p_stack_slug, v_name, '技', 'tech_stack', 'sensei · detected stack',
            format('The %s stack''s rules, adopted at each repository where it is detected.', v_name),
            'sensei', 'local', 'learned')
    on conflict (slug) where owner_namespace_id is null do nothing;
    select p.id into v_id
      from sensei.rule_packs p
     where p.owner_namespace_id is null and p.slug = 'stack:' || p_stack_slug;
    return v_id;
end;
$$;

comment on function stack_pack(text, text) is
'The `stack:<slug>` tech_stack library pack of a detected stack, made on first
use (#318 R6, D29). A stack is a pack applied at a level, not a level.';
