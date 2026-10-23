set search_path to sensei, extensions;

-- A repository's own namespace on the ladder's `repository` rung, made on first
-- use and bound to the repository (#318 R6, D33).
--
-- The repository rung is where a stack pack applies (D29: a stack is a pack
-- applied at a level, not a level) — so a repository needs a namespace to adopt
-- one at. Made here, in one place, because two writers need it: the scan, when
-- it detects a stack (`adopt_stack_pack`), and the upgrade that converted the
-- retired `technology` rung into packs.
--
-- Its identity is the repository's: slugged by `repo_key` — the normalized
-- remote, the same on every machine and in the dōjō — or by the repository's id
-- when it has no remote (a local-only repository is never federated). Not by
-- name: two repositories can share one, and one namespace for both would hand
-- each the other's packs. Bound through `repository_namespaces`, so
-- `namespaces_for_folder` reaches it like any other namespace of the repository.
--
-- Raises when the repository does not exist: a namespace for no repository
-- would be a fabricated identity.
create or replace function repository_namespace(p_repository_id uuid)
returns uuid
language plpgsql
volatile
as $$
declare
    v_name text;
    v_slug text;
    v_id   uuid;
begin
    select r.name, coalesce(r.repo_key, r.id::text) into v_name, v_slug
      from sensei.repositories r
     where r.id = p_repository_id;
    if v_slug is null then
        raise exception 'no repository %', p_repository_id using errcode = 'foreign_key_violation';
    end if;

    insert into sensei.namespaces (scope_key, name, slug)
    values ('repository', v_name, v_slug)
    on conflict (scope_key, slug) do nothing;
    select n.id into v_id
      from sensei.namespaces n
     where n.scope_key = 'repository' and n.slug = v_slug;

    insert into sensei.repository_namespaces (repository_id, namespace_id)
    values (p_repository_id, v_id)
    on conflict do nothing;
    return v_id;
end;
$$;

comment on function repository_namespace(uuid) is
'The repository''s own namespace on the `repository` rung — slugged by its
repo_key (its id when it has no remote), named after it — made on first use and
bound to it in repository_namespaces. Where a stack pack is adopted (#318 R6,
D33). Raises for an unknown repository.';
