set search_path to sensei, extensions;

-- A stack detected on a repository adopts its pack there (#318 R6, D33).
--
-- What the scan calls for every stack it detects on a repository, and what the
-- upgrade called for every repository a `technology` namespace was bound to — so
-- a stack reaches exactly the repositories it is detected in, then as now.
-- Additive and idempotent, like the binding it replaces: detection never
-- un-adopts (a stack that stops being detected keeps its pack until someone
-- removes the adoption).
create or replace function adopt_stack_pack(p_repository_id uuid, p_stack_slug text, p_stack_name text)
returns uuid
language plpgsql
volatile
as $$
declare
    v_pack uuid := sensei.stack_pack(p_stack_slug, p_stack_name);
begin
    insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, adopted_by)
    select v_pack, sensei.repository_namespace(p_repository_id), p.version, 'sensei'
      from sensei.rule_packs p
     where p.id = v_pack
    on conflict (pack_id, namespace_id) do nothing;
    return v_pack;
end;
$$;

comment on function adopt_stack_pack(uuid, text, text) is
'Adopt a detected stack''s pack (stack_pack) at the repository''s own namespace
(repository_namespace). Additive, idempotent; returns the pack id (#318 R6, D33).';
