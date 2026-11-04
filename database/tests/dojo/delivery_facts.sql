-- A board's facts filed by a contributor's daemon, kept as one copy per board
-- (spec docs/spec/daemon/tracker-delivery-metrics.md D4; #352).
--
-- dojo.file_delivery_facts files the Jira-derived facts of one board for the
-- caller: every repo_key must be the caller's own shared and elected repository
-- in the tenant (their verdict row says sync_enabled), the workspace is the one
-- those repositories are filed in (none → no_data_found, two → 55000), and the
-- stored copy is replaced whole only by a copy that is not lesser: complete
-- (pending_tickets = 0) beats incomplete, then the later history_at (null
-- lowest), then the later arrival. A lesser copy answers 'stale' and changes
-- nothing. The reads answer only a reader of the workspace, hide removed
-- sprints, order sprints as the daemon does and tickets as the app does.
--
-- WHAT BREAKS THIS TEST: a gate that trusts the repo_keys or the tenant in the
-- input; a workspace matched by repo_key across tenants; an incomplete or older
-- copy replacing a complete one; a replace that keeps the old copy's tickets; a
-- read that shows removed sprints, sorts AUTH-10 before AUTH-2, or answers a
-- non-member.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000521', 'a1000000-0000-4000-8000-000000000521', 'ada'),
  ('a0000000-0000-4000-8000-000000000522', 'a1000000-0000-4000-8000-000000000522', 'ola'),
  ('a0000000-0000-4000-8000-000000000523', 'a1000000-0000-4000-8000-000000000523', 'bob');

-- T: an organisation that shares its private repositories. U: another, not ola's.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000052a', 'organization/ztest-df-t', 'organization', 'ztest-df-t', 'T', 'x', now()),
  ('b0000000-0000-4000-8000-00000000052c', 'organization/ztest-df-u', 'organization', 'ztest-df-u', 'U', 'x', now());
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000052a', 'a0000000-0000-4000-8000-000000000521', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000052a', 'a0000000-0000-4000-8000-000000000522', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000052c', 'a0000000-0000-4000-8000-000000000521', 'admin',       'employer', 'github_oauth');
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000052a', 'team', 'active', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-00000000052c', 'team', 'active', current_date - 1, current_date + 30);
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values
  ('b0000000-0000-4000-8000-00000000052a', true),
  ('b0000000-0000-4000-8000-00000000052c', true);

-- 1 T private, filed in W1 · 2 T private, filed in W2 · 3 T private, filed nowhere ·
-- 4 U private with repo 1's key, filed in U's W3
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('91000000-0000-4000-8000-000000000521', 'b0000000-0000-4000-8000-00000000052a', 'github.com/ztest-df/one',   'one',   'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000522', 'b0000000-0000-4000-8000-00000000052a', 'github.com/ztest-df/two',   'two',   'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000523', 'b0000000-0000-4000-8000-00000000052a', 'github.com/ztest-df/three', 'three', 'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000524', 'b0000000-0000-4000-8000-00000000052c', 'github.com/ztest-df/one',   'one',   'private', now(), 'github');
insert into dojo.workspaces (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-000000000521', 'b0000000-0000-4000-8000-00000000052a', 'ztest-df-w1', 'W1'),
  ('c0000000-0000-4000-8000-000000000522', 'b0000000-0000-4000-8000-00000000052a', 'ztest-df-w2', 'W2'),
  ('c0000000-0000-4000-8000-000000000523', 'b0000000-0000-4000-8000-00000000052c', 'ztest-df-w3', 'W3');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-000000000521', '91000000-0000-4000-8000-000000000521', 'b0000000-0000-4000-8000-00000000052a'),
  ('c0000000-0000-4000-8000-000000000522', '91000000-0000-4000-8000-000000000522', 'b0000000-0000-4000-8000-00000000052a'),
  ('c0000000-0000-4000-8000-000000000523', '91000000-0000-4000-8000-000000000524', 'b0000000-0000-4000-8000-00000000052c');

-- ola writes to repo 1 on the forge, so she reads W1 (dojo.can_read_workspace).
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('91000000-0000-4000-8000-000000000521', 'a0000000-0000-4000-8000-000000000522', 'write', 'contributor');

-- The board as the daemon sends it (D3), with its facts varied per call below.
create temp table fixture (board jsonb);
insert into fixture values (jsonb_build_object(
  'board_key', repeat('ab', 32), 'kind', 'jira', 'board_name', 'Mobile Scrum',
  'facts', jsonb_build_object(
    'history_at', '2026-11-01T10:00:00Z', 'pending_tickets', 0,
    'predictability', 12.5, 'predictability_reason', null,
    'sprints', jsonb_build_array(
      jsonb_build_object('external_id', '101', 'seq', 101, 'name', 'Sprint 1', 'state', 'closed',
        'start_at', '2026-10-01T00:00:00Z', 'end_at', '2026-10-14T00:00:00Z', 'complete_at', '2026-10-14T00:00:00Z',
        'removed', false, 'sdlc', jsonb_build_object('velocity', 21)),
      jsonb_build_object('external_id', '102', 'seq', 102, 'name', 'Sprint 2', 'state', 'active',
        'start_at', '2026-10-15T00:00:00Z', 'end_at', '2026-10-28T00:00:00Z', 'complete_at', null,
        'removed', false, 'sdlc', jsonb_build_object('velocity', null)),
      jsonb_build_object('external_id', '103', 'seq', 103, 'name', 'Sprint 3', 'state', 'future',
        'start_at', null, 'end_at', null, 'complete_at', null, 'removed', false, 'sdlc', null),
      jsonb_build_object('external_id', '99', 'seq', 99, 'name', 'Gone', 'state', 'closed',
        'start_at', '2026-09-01T00:00:00Z', 'end_at', '2026-09-14T00:00:00Z', 'complete_at', null,
        'removed', true, 'sdlc', null)),
    'tickets', jsonb_build_array(
      jsonb_build_object('key', 'AUTH-10', 'title', 'Ten', 'status_category', 'done', 'points', 3,
        'sprints', jsonb_build_array('101', '102'), 'issue_created_at', '2026-09-20T00:00:00Z'),
      jsonb_build_object('key', 'AUTH-2', 'title', 'Two', 'status_category', 'new', 'points', null,
        'sprints', jsonb_build_array('102'), 'issue_created_at', null),
      jsonb_build_object('key', 'AUTH-3', 'title', 'Backlog', 'status_category', 'new', 'points', null,
        'sprints', jsonb_build_array(), 'issue_created_at', null)))));
grant select on fixture to authenticated;

set local role authenticated;

do $$
declare
    t   constant uuid := 'b0000000-0000-4000-8000-00000000052a';
    w1  constant uuid := 'c0000000-0000-4000-8000-000000000521';
    one constant text[] := array['github.com/ztest-df/one'];
    base jsonb := (select board from fixture);
    board_id uuid;
    out text;
    n int;
    keys text;
    ids text;
    -- the base board with its completeness and history moved
    with_facts jsonb;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000522"}', true);

    -- ── Consent: only the caller's own shared and elected repositories ──────
    begin
        perform dojo.file_delivery_facts(t, array['github.com/ztest-df/nope'], base);
        raise exception 'consent: an unknown repo_key was accepted';
    exception when insufficient_privilege then null;
    end;
    begin
        -- repo 1's key exists in U too, but ola has no verdict row there
        perform dojo.file_delivery_facts('b0000000-0000-4000-8000-00000000052c', one, base);
        raise exception 'consent: another tenant''s repository was accepted';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.file_delivery_facts(t, array[]::text[], base);
        raise exception 'consent: no repositories was accepted';
    exception when invalid_parameter_value then null;
    end;

    -- ── The workspace: none → 404, two → 409 ────────────────────────────────
    begin
        perform dojo.file_delivery_facts(t, array['github.com/ztest-df/three'], base);
        raise exception 'workspace: repositories filed nowhere were accepted';
    exception when no_data_found then null;
    end;
    begin
        perform dojo.file_delivery_facts(t, array['github.com/ztest-df/one', 'github.com/ztest-df/two'], base);
        raise exception 'workspace: repositories in two workspaces were accepted';
    exception when object_not_in_prerequisite_state then null;
    end;

    -- ── First copy ──────────────────────────────────────────────────────────
    out := dojo.file_delivery_facts(t, one, base);
    if out <> 'accepted' then raise exception 'first copy: answered %', out; end if;
    select b.board_id into board_id from dojo.workspace_delivery_boards(w1) b;
    if board_id is null then raise exception 'first copy: the board is not readable by its reporter'; end if;
    if (select count(*) from dojo.workspace_delivery_boards(w1)) <> 1 then
        raise exception 'first copy: the workspace does not hold exactly one board';
    end if;
    if (select predictability from dojo.workspace_delivery_boards(w1)) <> 12.5 then
        raise exception 'first copy: predictability did not travel';
    end if;

    -- Sprints: removed hidden; start date newest first, undated last, then seq.
    select string_agg(external_id, ',' order by ord) into ids
      from dojo.workspace_delivery_sprints(w1, board_id, 100, 0) with ordinality as s(external_id, seq, name, state, start_at, end_at, complete_at, sdlc, ord);
    if ids is distinct from '102,101,103' then raise exception 'sprints: order or removal wrong: %', ids; end if;
    if (select sdlc from dojo.workspace_delivery_sprints(w1, board_id, 100, 0) where external_id = '103') is not null then
        raise exception 'sprints: a sprint with no replay was given figures';
    end if;

    -- Tickets: the app's order; one sprint; the backlog.
    select string_agg(key, ',' order by ord) into keys
      from dojo.workspace_delivery_tickets(w1, board_id, null, false, 100, 0) with ordinality as x(key, title, status_category, points, sprints, issue_created_at, ord);
    if keys is distinct from 'AUTH-2,AUTH-3,AUTH-10' then raise exception 'tickets: order wrong: %', keys; end if;
    select string_agg(key, ',' order by key) into keys from dojo.workspace_delivery_tickets(w1, board_id, '101', false, 100, 0);
    if keys is distinct from 'AUTH-10' then raise exception 'tickets: sprint 101 holds %', keys; end if;
    select string_agg(key, ',' order by key) into keys from dojo.workspace_delivery_tickets(w1, board_id, null, true, 100, 0);
    if keys is distinct from 'AUTH-3' then raise exception 'tickets: the backlog holds %', keys; end if;
    if (select title from dojo.workspace_delivery_tickets(w1, board_id, null, false, 100, 0) where key = 'AUTH-10') <> 'Ten' then
        raise exception 'tickets: the title did not travel';
    end if;

    -- ── One copy ────────────────────────────────────────────────────────────
    -- An incomplete copy never replaces a complete one, however late its history.
    with_facts := jsonb_set(jsonb_set(base, '{facts,pending_tickets}', '4'), '{facts,history_at}', '"2026-11-03T10:00:00Z"');
    out := dojo.file_delivery_facts(t, one, with_facts);
    if out <> 'stale' then raise exception 'one copy: an incomplete copy answered %', out; end if;
    if (select pending_tickets from dojo.workspace_delivery_boards(w1)) <> 0 then
        raise exception 'one copy: an incomplete copy replaced a complete one';
    end if;

    -- An older complete copy is stale; a null history_at loses to any.
    out := dojo.file_delivery_facts(t, one, jsonb_set(base, '{facts,history_at}', '"2026-10-01T10:00:00Z"'));
    if out <> 'stale' then raise exception 'one copy: an older copy answered %', out; end if;
    out := dojo.file_delivery_facts(t, one, jsonb_set(base, '{facts,history_at}', 'null'));
    if out <> 'stale' then raise exception 'one copy: a null history_at answered %', out; end if;

    -- A later complete copy replaces it WHOLE: AUTH-3 is not in it, so it goes.
    with_facts := jsonb_set(jsonb_set(base, '{facts,history_at}', '"2026-11-02T10:00:00Z"'),
                            '{facts,tickets}', (base -> 'facts' -> 'tickets') - 2);
    out := dojo.file_delivery_facts(t, one, with_facts);
    if out <> 'accepted' then raise exception 'one copy: a later copy answered %', out; end if;
    if exists (select 1 from dojo.workspace_delivery_tickets(w1, board_id, null, false, 100, 0) where key = 'AUTH-3') then
        raise exception 'one copy: the replaced copy''s ticket survived';
    end if;
    if (select history_at from dojo.workspace_delivery_boards(w1)) <> '2026-11-02T10:00:00Z' then
        raise exception 'one copy: history_at did not move';
    end if;

    -- Equal history_at: the later arrival wins.
    out := dojo.file_delivery_facts(t, one, jsonb_set(with_facts, '{board_name}', '"Renamed"'));
    if out <> 'accepted' then raise exception 'one copy: an equal copy answered %', out; end if;
    if (select name from dojo.workspace_delivery_boards(w1)) <> 'Renamed' then
        raise exception 'one copy: the later of two equal copies did not win';
    end if;
end
$$;

-- ── The table holds exactly what the copy said (as the owner) ─────────────
reset role;
do $$
begin
    if (select count(*) from dojo.delivery_boards where board_key = repeat('ab', 32)) <> 1 then
        raise exception 'storage: not one board row';
    end if;
    if (select reported_by from dojo.delivery_boards where board_key = repeat('ab', 32))
       <> 'a0000000-0000-4000-8000-000000000522' then
        raise exception 'storage: reported_by is not the caller';
    end if;
    if exists (select 1 from dojo.delivery_boards b where b.board_key = repeat('ab', 32)
                and b.workspace_id <> 'c0000000-0000-4000-8000-000000000521') then
        raise exception 'storage: the board was filed outside W1';
    end if;
    if (select count(*) from dojo.delivery_sprints s join dojo.delivery_boards b on b.id = s.board_id
         where b.board_key = repeat('ab', 32)) <> 4 then
        raise exception 'storage: the copy''s four sprints (one removed) are not all kept';
    end if;
end
$$;

-- ── A non-member reads nothing and writes nothing ─────────────────────────
set local role authenticated;
do $$
declare
    w1 constant uuid := 'c0000000-0000-4000-8000-000000000521';
    b  uuid := (select null::uuid);
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000523"}', true);
    if exists (select 1 from dojo.workspace_delivery_boards(w1)) then
        raise exception 'reads: a non-member read the workspace''s boards';
    end if;
    begin
        perform dojo.file_delivery_facts('b0000000-0000-4000-8000-00000000052a', array['github.com/ztest-df/one'],
                                         (select board from fixture));
        raise exception 'consent: a non-member filed a board';
    exception when insufficient_privilege then null;
    end;
end
$$;
reset role;

rollback;
