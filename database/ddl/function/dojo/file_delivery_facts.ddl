set search_path to dojo, extensions;

-- File one board's tracker-derived facts for the signed-in person's daemon, kept as
-- ONE copy per board (spec docs/spec/daemon/tracker-delivery-metrics.md D4; #352).
--
-- Consent is the caller's own: every repo_key must be their verdict row of
-- dojo.all_my_repositories in p_tenant with sync_enabled, else nothing is
-- written (insufficient_privilege). The workspace is the one those verified
-- repositories are filed in — resolved from their ids, never by matching a
-- repo_key across tenants: none → no_data_found ('workspace_not_filed'), more
-- than one → object_not_in_prerequisite_state ('workspace_ambiguous').
--
-- One copy: the board row is inserted when missing, then locked FOR UPDATE, so
-- two daemons pushing one board at once are ordered. The stored copy is kept
-- (answer 'stale', nothing written) when it is complete and the incoming is not;
-- or both are equally complete, either has a history_at, and the incoming one is
-- older (NULL is the oldest); or neither has a history_at, both carry a synced_at
-- and the incoming one is older (A5). A copy's synced_at is now() minus the sync
-- age its daemon sent, so two daemons' clocks are never compared. Otherwise the
-- incoming copy replaces the board row and ALL its sprints and tickets (answer
-- 'accepted') — so a copy's sprints and tickets always come from one report.
--
-- SECURITY DEFINER: the delivery tables take no session write.
create or replace function dojo.file_delivery_facts(
    p_tenant    uuid,
    p_repo_keys text[],
    p_board     jsonb
)
    returns text
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_principal  uuid := dojo.current_principal_id();
    v_workspace  uuid;
    v_facts      jsonb := p_board -> 'facts';
    v_key        text := p_board ->> 'board_key';
    v_pending    integer := (v_facts ->> 'pending_tickets')::integer;
    v_history    timestamptz := (v_facts ->> 'history_at')::timestamptz;
    v_age        bigint;
    v_synced     timestamptz;
    v_board      dojo.delivery_boards%rowtype;
    v_new        uuid;
begin
    if v_key is null or v_key !~ '^[0-9a-f]{64}$' or p_board ->> 'kind' is null
       or p_board ->> 'board_name' is null or v_pending is null or v_pending < 0
       or jsonb_typeof(v_facts -> 'sprints') is distinct from 'array'
       or jsonb_typeof(v_facts -> 'tickets') is distinct from 'array' then
        raise exception 'a board needs board_key, kind, board_name and facts with pending_tickets, sprints and tickets'
            using errcode = 'invalid_parameter_value';
    end if;
    -- The bounds hold here, whoever calls: at most 5 000 tickets and 4 MB (D3).
    if jsonb_array_length(v_facts -> 'tickets') > 5000 or octet_length(p_board::text) > 4 * 1024 * 1024 then
        raise exception 'a board carries at most 5000 tickets and 4 MB' using errcode = 'invalid_parameter_value';
    end if;
    -- Every sprint's figures are an object whose reasons map figure keys to codes
    -- (or null, for a sprint with no replay): every reader expands them.
    if exists (
        select 1 from jsonb_array_elements(v_facts -> 'sprints') x
         where jsonb_typeof(x) <> 'object'
            or (x -> 'sdlc' is not null and jsonb_typeof(x -> 'sdlc') not in ('object', 'null'))
            or (jsonb_typeof(x -> 'sdlc') = 'object'
                and case when jsonb_typeof(x -> 'sdlc' -> 'reasons') = 'object'
                         then exists (select 1 from jsonb_each(x -> 'sdlc' -> 'reasons') r
                                       where jsonb_typeof(r.value) <> 'string')
                         else true end)) then
        raise exception 'a sprint''s figures are an object whose reasons map figure keys to codes'
            using errcode = 'invalid_parameter_value';
    end if;
    -- A history from the future would win every comparison and never be replaced.
    if v_history > now() + interval '1 day' then
        raise exception 'history_at is in the future' using errcode = 'invalid_parameter_value';
    end if;

    -- A sync age is whole seconds, at most ten years (A5): make_interval alone
    -- takes a negative one (a future synced_at) and errors on a huge one.
    if v_facts ? 'synced_age_s' and jsonb_typeof(v_facts -> 'synced_age_s') <> 'null' then
        if jsonb_typeof(v_facts -> 'synced_age_s') <> 'number'
           or (v_facts ->> 'synced_age_s') !~ '^[0-9]{1,9}$'
           or (v_facts ->> 'synced_age_s')::bigint > 315360000 then
            raise exception 'synced_age_s is whole seconds from 0 to 315360000'
                using errcode = 'invalid_parameter_value';
        end if;
        v_age := (v_facts ->> 'synced_age_s')::bigint;
        v_synced := now() - make_interval(secs => v_age);
    end if;

    -- Consent and the workspace are the caller's own (dojo.delivery_scope).
    select distinct sc.workspace_id into strict v_workspace
      from dojo.delivery_scope(p_tenant, p_repo_keys) sc;

    insert into dojo.delivery_boards
           (tenant_id, workspace_id, board_key, kind, name, reported_by, history_at, synced_at,
            pending_tickets, predictability, predictability_reason)
    values (p_tenant, v_workspace, v_key, p_board ->> 'kind', p_board ->> 'board_name', v_principal,
            v_history, v_synced, v_pending, (v_facts ->> 'predictability')::numeric, v_facts ->> 'predictability_reason')
    on conflict (workspace_id, board_key) do nothing
    returning id into v_new;

    if v_new is null then
        select * into v_board
          from dojo.delivery_boards b
         where b.workspace_id = v_workspace and b.board_key = v_key
           for update;
        if (v_board.pending_tickets = 0 and v_pending > 0)
           or ((v_board.pending_tickets = 0) = (v_pending = 0)
               and (v_history is not null or v_board.history_at is not null)
               and coalesce(v_history, '-infinity') < coalesce(v_board.history_at, '-infinity'))
           or ((v_board.pending_tickets = 0) = (v_pending = 0)
               and v_history is null and v_board.history_at is null
               and v_synced < v_board.synced_at) then
            return 'stale';
        end if;
        update dojo.delivery_boards
           set kind = p_board ->> 'kind', name = p_board ->> 'board_name', reported_by = v_principal,
               reported_at = now(), history_at = v_history, synced_at = v_synced,
               pending_tickets = v_pending,
               predictability = (v_facts ->> 'predictability')::numeric,
               predictability_reason = v_facts ->> 'predictability_reason'
         where id = v_board.id;
        delete from dojo.delivery_sprints where board_id = v_board.id;
        delete from dojo.delivery_tickets where board_id = v_board.id;
        v_new := v_board.id;
    end if;

    insert into dojo.delivery_sprints
           (board_id, external_id, seq, name, state, start_at, end_at, complete_at, removed, sdlc)
    select v_new, x.external_id, x.seq, x.name, x.state, x.start_at, x.end_at, x.complete_at,
           x.removed, x.sdlc
      from jsonb_to_recordset(v_facts -> 'sprints') as x(
             external_id text, seq bigint, name text, state text, start_at timestamptz,
             end_at timestamptz, complete_at timestamptz, removed boolean, sdlc jsonb);

    insert into dojo.delivery_tickets (board_id, key, title, status_category, points, sprints, issue_created_at)
    select v_new, x.key, x.title, x.status_category, x.points,
           array(select jsonb_array_elements_text(x.sprints)), x.issue_created_at
      from jsonb_to_recordset(v_facts -> 'tickets') as x(
             key text, title text, status_category text, points numeric, sprints jsonb,
             issue_created_at timestamptz);

    return 'accepted';
end
$$;

revoke all on function dojo.file_delivery_facts(uuid, text[], jsonb) from public;
grant execute on function dojo.file_delivery_facts(uuid, text[], jsonb) to authenticated;

comment on function dojo.file_delivery_facts(uuid, text[], jsonb) is
'File one tracker board''s facts for the caller (#352, tracker-delivery-metrics D4): every repo_key must be the caller''s own sync_enabled repository in p_tenant (insufficient_privilege); the workspace comes from those repositories (no_data_found when none, 55000 when several). Keeps ONE copy per board: the row is locked, then a lesser copy (incomplete against complete; an older history_at when either has one; or, when neither has and both carry one, an older synced_at = now() - synced_age_s, A5) answers ''stale'' and writes nothing; a synced_age_s that is not whole seconds in 0..315360000 is invalid_parameter_value; any other replaces the board, its sprints and its tickets whole and answers ''accepted''.';
