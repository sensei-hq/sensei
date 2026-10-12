set search_path to dojo, extensions;

-- Elect (or un-elect) one repository for sharing, as the signed-in person (#236;
-- access matrix "Projects and repositories").
--
-- Decides from the CALLER's own verdict row of dojo.all_my_repositories — the
-- row dojo.my_repositories() returns them — so who holds the authority and
-- whether this person may change it are the view's answers, computed once:
--   no row for the caller        → no_data_found                    (404)
--   no authority yet (no capture) → object_not_in_prerequisite_state (409):
--                                   an election filed under a guessed authority
--                                   would stop counting at the next capture
--   not configurable_by_me        → insufficient_privilege           (403)
-- A user election carries the caller as its principal; an org election carries
-- none (the table's CHECK). Upserts on the (repository, authority, principal)
-- key, then re-reads the verdict it reports — electing is half the decision, and
-- a repository whose entitlement still refuses must come back sync_enabled=false.
--
-- SECURITY DEFINER: dojo.repository_elections has no session grant.
create or replace function dojo.set_repository_election(p_repo_key text, p_elected boolean)
    returns table (repo_key text, elected boolean, authority dojo.share_authority,
                   sync_enabled boolean, reason_code text)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
-- The OUT columns share names with the table's; inside the body, a bare name is
-- the column.
#variable_conflict use_column
declare
    v_principal uuid := dojo.current_principal_id();
    mine        dojo.all_my_repositories;
begin
    select * into mine
      from dojo.all_my_repositories v
     where v.principal_id = v_principal
       and v.repo_key = p_repo_key;
    if not found then
        raise exception 'no repository % for this account', p_repo_key using errcode = 'no_data_found';
    end if;
    if mine.authority is null then
        raise exception '% has no authority yet: %', p_repo_key,
              coalesce(mine.reason, mine.reason_code, 'forge visibility is not captured')
            using errcode = 'object_not_in_prerequisite_state';
    end if;
    if not mine.configurable_by_me then
        raise exception 'you may not change sharing for %', p_repo_key
              || case when mine.reason_actor is not null then ' (' || mine.reason_actor || ' can change it)' else '' end
            using errcode = 'insufficient_privilege';
    end if;

    insert into dojo.repository_elections as e (tenant_id, repository_id, authority, principal_id, elected)
    values (mine.tenant_id, mine.repository_id, mine.authority,
            case when mine.authority = 'user' then v_principal end, p_elected)
    on conflict (repository_id, authority, principal_id)
    do update set elected = excluded.elected, elected_at = now(), modified_at = now();

    return query
    select v.repo_key, v.elected, mine.authority, v.sync_enabled, v.reason_code
      from dojo.all_my_repositories v
     where v.principal_id = v_principal
       and v.repo_key = p_repo_key;
end;
$$;

revoke all on function dojo.set_repository_election(text, boolean) from public;
grant execute on function dojo.set_repository_election(text, boolean) to authenticated;

comment on function dojo.set_repository_election(text, boolean) is
'Elect or un-elect a repository for sharing, decided from the caller''s own
verdict row (authority, configurable_by_me). Refuses no_data_found (not theirs),
object_not_in_prerequisite_state (no authority yet), insufficient_privilege (not
theirs to change). Returns the re-read verdict (#236).';
