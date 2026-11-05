-- A board's effort, filed per contributor and merged for its readers (spec
-- docs/spec/daemon/tracker-delivery-metrics.md D1, D4, A2, A3, B16; #353).
--
-- dojo.file_delivery files a push's facts and the caller's effort in one
-- transaction, replacing everything the caller reported for the board. The merge
-- (dojo.delivery_effort_merged) sums known values with partial counts, takes the
-- earliest start and the earliest PR, and recomputes every ratio from numerators
-- and denominators summed over the same contributors. Readers see it under B16:
-- nothing from one contributor, and no cost at anyone's rate.
--
-- WHAT BREAKS THIS TEST: a ratio averaged or computed over mismatched people; the
-- latest submission taken; a not-applicable null counted as unknown; an unknown
-- merged as 0; a push that leaves the caller's old rows; a row naming a
-- repository the push did not; one contributor's figures, cost or report time
-- shown to a workspace reader.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000531', 'a1000000-0000-4000-8000-000000000531', 'ada'),
  ('a0000000-0000-4000-8000-000000000532', 'a1000000-0000-4000-8000-000000000532', 'ola'),
  ('a0000000-0000-4000-8000-000000000533', 'a1000000-0000-4000-8000-000000000533', 'bob');

-- T: an organisation that shares its private repositories. U: another, not ola's.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000053a', 'organization/ztest-de-t', 'organization', 'ztest-de-t', 'T', 'x', now()),
  ('b0000000-0000-4000-8000-00000000053c', 'organization/ztest-de-u', 'organization', 'ztest-de-u', 'U', 'x', now());
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000053a', 'a0000000-0000-4000-8000-000000000531', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000053a', 'a0000000-0000-4000-8000-000000000532', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000053c', 'a0000000-0000-4000-8000-000000000531', 'admin',       'employer', 'github_oauth');
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000053a', 'team', 'active', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-00000000053c', 'team', 'active', current_date - 1, current_date + 30);
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values
  ('b0000000-0000-4000-8000-00000000053a', true),
  ('b0000000-0000-4000-8000-00000000053c', true);

-- 1 T private, filed in W1 · 2 T private, filed in W2 · 3 T private, filed nowhere ·
-- 4 U private with repo 1's key, filed in U's W3
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('91000000-0000-4000-8000-000000000531', 'b0000000-0000-4000-8000-00000000053a', 'github.com/ztest-de/one',   'one',   'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000532', 'b0000000-0000-4000-8000-00000000053a', 'github.com/ztest-de/two',   'two',   'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000533', 'b0000000-0000-4000-8000-00000000053a', 'github.com/ztest-de/three', 'three', 'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000534', 'b0000000-0000-4000-8000-00000000053c', 'github.com/ztest-de/one',   'one',   'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000535', 'b0000000-0000-4000-8000-00000000053a', 'github.com/ztest-de/optout', 'optout', 'public', now(), 'github');
insert into dojo.workspaces (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-000000000531', 'b0000000-0000-4000-8000-00000000053a', 'ztest-de-w1', 'W1'),
  ('c0000000-0000-4000-8000-000000000532', 'b0000000-0000-4000-8000-00000000053a', 'ztest-de-w2', 'W2'),
  ('c0000000-0000-4000-8000-000000000533', 'b0000000-0000-4000-8000-00000000053c', 'ztest-de-w3', 'W3');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-000000000531', '91000000-0000-4000-8000-000000000531', 'b0000000-0000-4000-8000-00000000053a'),
  ('c0000000-0000-4000-8000-000000000532', '91000000-0000-4000-8000-000000000532', 'b0000000-0000-4000-8000-00000000053a'),
  ('c0000000-0000-4000-8000-000000000533', '91000000-0000-4000-8000-000000000534', 'b0000000-0000-4000-8000-00000000053c'),
  ('c0000000-0000-4000-8000-000000000531', '91000000-0000-4000-8000-000000000535', 'b0000000-0000-4000-8000-00000000053a');
-- ola has opted out of syncing repo 5: shared, but not elected for her (sync_enabled false).
insert into dojo.repository_elections (tenant_id, repository_id, authority, principal_id, elected) values
  ('b0000000-0000-4000-8000-00000000053a', '91000000-0000-4000-8000-000000000535', 'user',
   'a0000000-0000-4000-8000-000000000532', false);

-- ola writes to repo 1 on the forge, so she reads W1 (dojo.can_read_workspace).
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('91000000-0000-4000-8000-000000000531', 'a0000000-0000-4000-8000-000000000532', 'write', 'contributor'),
  ('91000000-0000-4000-8000-000000000532', 'a0000000-0000-4000-8000-000000000532', 'write', 'contributor');

-- The board as the daemon sends it (D3), with its facts varied per call below.
create temp table fixture (board jsonb);
insert into fixture values (jsonb_build_object(
  'board_key', repeat('ab', 32), 'kind', 'jira', 'board_name', 'Mobile Scrum',
  'facts', jsonb_build_object(
    'history_at', '2026-11-01T10:00:00Z', 'pending_tickets', 0,
    'predictability', null, 'predictability_reason', 'too_few_sprints',
    'sprints', jsonb_build_array(
      jsonb_build_object('external_id', '101', 'seq', 101, 'name', 'Sprint 1', 'state', 'closed',
        'start_at', '2026-10-01T00:00:00Z', 'end_at', '2026-10-14T00:00:00Z', 'complete_at', '2026-10-14T00:00:00Z',
        'removed', false, 'sdlc', jsonb_build_object('velocity', 21, 'reasons', '{}'::jsonb)),
      jsonb_build_object('external_id', '102', 'seq', 102, 'name', 'Sprint 2', 'state', 'active',
        'start_at', '2026-10-15T00:00:00Z', 'end_at', '2026-10-28T00:00:00Z', 'complete_at', null,
        'removed', false, 'sdlc', jsonb_build_object('velocity', null, 'reasons', jsonb_build_object('velocity', 'history_pending'))),
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
        'sprints', jsonb_build_array(), 'issue_created_at', null),
      jsonb_build_object('key', 'AUTH-4', 'title', 'Only in a removed sprint', 'status_category', 'new', 'points', null,
        'sprints', jsonb_build_array('99'), 'issue_created_at', null)))));
grant select on fixture to authenticated;

-- ── The merge, pure (as the owner: no client calls these directly) ─────────────
do $$
declare
    m jsonb;
    a jsonb := jsonb_build_object('values', jsonb_build_object(
        'sessions', 2, 'api_cost_n', 2, 'api_cost_partial', 0, 'prompts', 4, 'ai_token_cost', null,
        'worked_ms', 3600000, 'on_hold_ms', 360000, 'execution_ms', 3600000, 'rework_ms', 900000,
        'build_ok', 3, 'build_fail', 1, 'recovery_ms_sum', 600000, 'recovery_n', 1,
        'compile_defects', 1, 'test_defects', 0, 'review_fixes', 1, 'escaped_task_defects', 0,
        'effort_cost', 50, 'rework_cost', 12.5,
        'work_started_at', '2026-11-02T09:00:00Z', 'submitted_at', '2026-11-02T10:00:00Z', 'submitted_via', 'pr'),
        'reasons', jsonb_build_object('ai_token_cost', 'not_priced'));
    -- B started earlier and never submitted: no window, no rework (not applicable).
    b jsonb := jsonb_build_object('values', jsonb_build_object(
        'sessions', 1, 'api_cost_n', 1, 'api_cost_partial', 0, 'prompts', 2, 'ai_token_cost', 3.0,
        'worked_ms', 1800000, 'on_hold_ms', null, 'execution_ms', null, 'rework_ms', null,
        'build_ok', 1, 'build_fail', 1, 'recovery_ms_sum', 1200000, 'recovery_n', 1,
        'compile_defects', 0, 'test_defects', 2, 'review_fixes', 0, 'escaped_task_defects', 1,
        'effort_cost', 70, 'rework_cost', null,
        'work_started_at', '2026-11-02T08:00:00Z', 'submitted_at', null, 'submitted_via', null),
        'reasons', jsonb_build_object('on_hold_ms', 'not_submitted', 'execution_ms', 'not_submitted',
            'rework_ms', 'not_submitted', 'rework_cost', 'not_submitted', 'submitted_at', 'not_submitted',
            'submitted_via', 'not_submitted'));
begin
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(a, b), '2026-11-01T10:00:00Z', null);
    -- Gate 14: sums, the earliest start, the earliest PR, ratios from summed parts.
    if (m #>> '{values,worked_minutes}')::numeric is distinct from 90 then raise exception 'worked_minutes: %', m; end if;
    if (m #>> '{values,work_started_at}')::timestamptz is distinct from '2026-11-02T08:00:00Z' then raise exception 'start: %', m; end if;
    if (m #>> '{values,submitted_at}')::timestamptz is distinct from '2026-11-02T10:00:00Z' or m #>> '{values,submitted_via}' is distinct from 'pr' then
        raise exception 'submission: %', m; end if;
    if (m #>> '{values,execution_hours}')::numeric is distinct from 2 then raise exception 'execution_hours: %', m; end if;
    if (m #>> '{values,lead_hours}')::numeric is distinct from 24 then raise exception 'lead_hours: %', m; end if;
    if round((m #>> '{values,build_failure_rate}')::numeric, 4) is distinct from round(2.0 / 6 * 100, 4) then raise exception 'build_failure_rate: %', m; end if;
    if (m #>> '{values,mean_build_recovery_minutes}')::numeric is distinct from 15 then raise exception 'recovery: %', m; end if;
    if (m #>> '{values,total_defects}')::numeric is distinct from 5 or (m #>> '{values,review_defects}')::numeric is distinct from 2 then
        raise exception 'defects: %', m; end if;
    if (m #>> '{values,testing_efficiency}')::numeric is distinct from 60 then raise exception 'testing_efficiency: %', m; end if;
    if (m #>> '{values,effort_cost}')::numeric is distinct from 120 then raise exception 'effort_cost: %', m; end if;
    -- Gate 15: an unknown merges as partial, never 0; a not-applicable null leaves
    -- that contributor out of the ratio and is not partial.
    if (m #>> '{values,ai_token_cost}')::numeric is distinct from 3.0 or (m #>> '{partial,ai_token_cost}')::int is distinct from 1 then
        raise exception 'ai_token_cost: %', m; end if;
    if m #> '{values,cost_per_prompt_cycle}' is distinct from 'null'::jsonb or m #>> '{reasons,cost_per_prompt_cycle}' is distinct from 'partial_cost' then
        raise exception 'cost_per_prompt_cycle: %', m; end if;
    if (m #>> '{values,post_review_rework_ratio}')::numeric is distinct from 25 or m -> 'partial' ? 'post_review_rework_ratio' then
        raise exception 'post_review_rework_ratio is A''s alone and whole: %', m; end if;
    if (m #>> '{values,on_hold_ratio}')::numeric is distinct from 10 or m -> 'partial' ? 'on_hold_ratio' then
        raise exception 'on_hold_ratio: %', m; end if;
    if m -> 'partial' ? 'rework_minutes' or (m #>> '{values,rework_minutes}')::numeric is distinct from 15 then
        raise exception 'not submitted is not unknown: %', m; end if;

    -- Submission is the earliest PR, never a later one nor an earlier push (A3).
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(a,
        jsonb_set(jsonb_set(b, '{values,submitted_at}', '"2026-11-02T12:00:00Z"'), '{values,submitted_via}', '"pr"'),
        jsonb_set(jsonb_set(b, '{values,submitted_at}', '"2026-11-02T09:30:00Z"'), '{values,submitted_via}', '"push"')), null, null);
    if (m #>> '{values,submitted_at}')::timestamptz is distinct from '2026-11-02T10:00:00Z' or m #>> '{values,submitted_via}' is distinct from 'pr' then
        raise exception 'earliest PR: %', m; end if;
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(b,
        jsonb_set(jsonb_set(b, '{values,submitted_at}', '"2026-11-02T09:30:00Z"'), '{values,submitted_via}', '"push"')), null, null);
    if m #>> '{values,submitted_via}' is distinct from 'push' then raise exception 'no PR: the earliest push: %', m; end if;

    -- Known costs, but a session unpriced: the local rule, summed → partial_cost.
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(
        jsonb_set(jsonb_set(a, '{values,ai_token_cost}', '5'), '{reasons}', '{}'),
        jsonb_set(jsonb_set(b, '{values,api_cost_n}', '0'), '{values,ai_token_cost}', '1')), null, null);
    if m #>> '{reasons,cost_per_prompt_cycle}' is distinct from 'partial_cost' then raise exception 'api_cost_n < sessions: %', m; end if;
    if m #>> '{reasons,lead_hours}' is distinct from 'not_synced' then raise exception 'lead without a creation: %', m; end if;
    -- Every cost whole: cost ÷ prompts.
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(
        jsonb_set(jsonb_set(a, '{values,ai_token_cost}', '5'), '{reasons}', '{}'),
        jsonb_set(b, '{values,ai_token_cost}', '1')), null, null);
    if (m #>> '{values,cost_per_prompt_cycle}')::numeric is distinct from 1 then raise exception 'cost per prompt: %', m; end if;

    -- Nobody knew it: the shared reason, else no_contributor_value; never 0.
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(
        jsonb_set(a, '{values,effort_cost}', 'null') || jsonb_build_object('reasons', jsonb_build_object('effort_cost', 'no_rate')),
        jsonb_set(b, '{values,effort_cost}', 'null') || jsonb_build_object('reasons', jsonb_build_object('effort_cost', 'no_rate'))), null, null);
    if m #> '{values,effort_cost}' is distinct from 'null'::jsonb or m #>> '{reasons,effort_cost}' is distinct from 'no_rate' then
        raise exception 'shared reason: %', m; end if;
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(
        jsonb_set(a, '{values,effort_cost}', 'null') || jsonb_build_object('reasons', jsonb_build_object('effort_cost', 'no_rate')),
        jsonb_set(b, '{values,effort_cost}', 'null') || jsonb_build_object('reasons', jsonb_build_object('effort_cost', 'not_derived'))), null, null);
    if m #>> '{reasons,effort_cost}' is distinct from 'no_contributor_value' then raise exception 'mixed reasons: %', m; end if;

    -- A sprint's usage sums, with its coverage in _n; earliest first, latest last.
    m := dojo.delivery_effort_merged('sprint', jsonb_build_array(
        jsonb_build_object('values', jsonb_build_object('sessions', 2, 'active_secs', 60, 'active_n', 2, 'tokens_fresh', null,
            'tokens_fresh_n', 0, 'by_source', jsonb_build_object('branch', 1, 'prompt', 1),
            'first_at', '2026-11-02T09:00:00Z', 'last_at', '2026-11-02T10:00:00Z'), 'reasons', '{}'::jsonb),
        jsonb_build_object('values', jsonb_build_object('sessions', 1, 'active_secs', 30, 'active_n', 1, 'tokens_fresh', 7,
            'tokens_fresh_n', 1, 'by_source', jsonb_build_object('branch', 0, 'prompt', 1),
            'first_at', '2026-11-02T08:00:00Z', 'last_at', '2026-11-02T09:30:00Z'), 'reasons', '{}'::jsonb)), null, null);
    if (m #>> '{values,active_secs}')::numeric is distinct from 90 or (m #>> '{values,tokens_fresh}')::numeric is distinct from 7
       or (m #>> '{values,tokens_fresh_n}')::numeric is distinct from 1 or (m #>> '{values,by_source,prompt}')::numeric is distinct from 2
       or (m #>> '{values,first_at}')::timestamptz is distinct from '2026-11-02T08:00:00Z'
       or (m #>> '{values,last_at}')::timestamptz is distinct from '2026-11-02T10:00:00Z' or m -> 'partial' is distinct from '{}'::jsonb then
        raise exception 'sprint usage: %', m; end if;

    -- The project: per ticket over the board's tickets; partial tokens → null.
    m := dojo.delivery_effort_merged('project', jsonb_build_array(
        jsonb_build_object('values', jsonb_build_object('sessions', 2, 'tokens', 900, 'sessions_without_tokens', 0, 'prompts', 6), 'reasons', '{}'::jsonb),
        jsonb_build_object('values', jsonb_build_object('sessions', 1, 'tokens', 300, 'sessions_without_tokens', 0, 'prompts', 2), 'reasons', '{}'::jsonb)), null, 4);
    if (m #>> '{values,tokens_per_ticket}')::numeric is distinct from 300 or (m #>> '{values,prompts_per_ticket}')::numeric is distinct from 2 then
        raise exception 'project: %', m; end if;
    m := dojo.delivery_effort_merged('project', jsonb_build_array(
        jsonb_build_object('values', jsonb_build_object('sessions', 2, 'tokens', 900, 'sessions_without_tokens', 1, 'prompts', null),
            'reasons', jsonb_build_object('prompts', 'not_derived'))), null, 4);
    if m #>> '{reasons,tokens_per_ticket}' is distinct from 'partial_tokens' or m #>> '{reasons,prompts_per_ticket}' is distinct from 'not_derived' then
        raise exception 'project unknowns: %', m; end if;

    -- An unknown contributor makes a ratio partial; never a hidden 0 (Wrong gate).
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(a,
        jsonb_set(jsonb_set(jsonb_set(b, '{values,build_ok}', 'null'), '{values,build_fail}', 'null'),
                  '{reasons}', (b -> 'reasons') || '{"build_ok": "not_derived", "build_fail": "not_derived"}')), null, null);
    if round((m #>> '{values,build_failure_rate}')::numeric, 4) is distinct from 25 or (m #>> '{partial,build_failure_rate}')::int is distinct from 1 then
        raise exception 'build_failure_rate over A alone, partial 1: %', m; end if;
    -- Not applicable to everyone: the merged reason is theirs, not "unknown".
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(b,
        jsonb_build_object('values', jsonb_build_object('sessions', 1, 'submitted_at', null, 'work_started_at', null),
                           'reasons', jsonb_build_object('submitted_at', 'not_started', 'work_started_at', 'not_started'))), null, null);
    if m #>> '{reasons,submitted_at}' is distinct from 'not_submitted' or m #>> '{reasons,execution_hours}' is distinct from 'not_submitted' then
        raise exception 'not applicable kept: %', m; end if;
    -- An unknown start makes the execution partial.
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(a,
        jsonb_build_object('values', jsonb_build_object('sessions', 1, 'work_started_at', null),
                           'reasons', jsonb_build_object('work_started_at', 'not_derived'))), null, null);
    if (m #>> '{partial,execution_hours}')::int is distinct from 1 then raise exception 'execution partial: %', m; end if;

    -- No prompts at all, and a creation after the submission.
    m := dojo.delivery_effort_merged('ticket', jsonb_build_array(
        jsonb_set(jsonb_set(jsonb_set(a, '{values,prompts}', '0'), '{values,ai_token_cost}', '5'), '{reasons}', '{}')),
        '2026-11-03T00:00:00Z', null);
    if m #>> '{reasons,cost_per_prompt_cycle}' is distinct from 'no_prompts' then raise exception 'no_prompts: %', m; end if;
    if m #> '{values,lead_hours}' is distinct from 'null'::jsonb or m #>> '{reasons,lead_hours}' is distinct from 'created_after_submit' then
        raise exception 'negative lead: %', m; end if;

    -- B16: a reader sees nothing from one contributor, and no cost from two.
    if dojo.delivery_team_view(m, 1) is not null then raise exception 'one contributor shown'; end if;
    m := dojo.delivery_team_view(dojo.delivery_effort_merged('ticket', jsonb_build_array(a, b), null, null), 2);
    if m is null or m -> 'values' is null or (m #>> '{values,worked_minutes}')::numeric is distinct from 90
       or m -> 'values' ? 'effort_cost' or m -> 'values' ? 'rework_cost' or m -> 'reasons' ? 'rework_cost' then
        raise exception 'cost shown, or the merge hidden: %', m; end if;
end
$$;

set local role authenticated;

do $$
declare
    t     constant uuid := 'b0000000-0000-4000-8000-00000000053a';
    w1    constant uuid := 'c0000000-0000-4000-8000-000000000531';
    one   constant text[] := array['github.com/ztest-de/one'];
    base  jsonb := (select board from fixture);
    board uuid;
    out   jsonb;
    r     record;
    row_a jsonb := jsonb_build_object('repo_keys', jsonb_build_array('github.com/ztest-de/one'),
                     'values', jsonb_build_object('sessions', 1, 'worked_ms', 3600000, 'build_ok', 3, 'build_fail', 1,
                        'work_started_at', '2026-11-02T09:00:00Z', 'submitted_at', '2026-11-02T10:00:00Z',
                        'submitted_via', 'pr', 'effort_cost', 50),
                     'reasons', '{}'::jsonb);
    row_b jsonb := jsonb_build_object('repo_keys', jsonb_build_array('github.com/ztest-de/one'),
                     'values', jsonb_build_object('sessions', 1, 'worked_ms', 1800000, 'build_ok', 1, 'build_fail', 1,
                        'work_started_at', '2026-11-02T08:00:00Z', 'submitted_at', null, 'submitted_via', null,
                        'effort_cost', 70),
                     'reasons', jsonb_build_object('submitted_at', 'not_submitted', 'submitted_via', 'not_submitted'));
    proj_a jsonb := jsonb_build_object('repo_keys', jsonb_build_array('github.com/ztest-de/one'),
                     'values', jsonb_build_object('sessions', 2, 'tokens', 900, 'sessions_without_tokens', 0, 'prompts', 4),
                     'reasons', '{}'::jsonb);
    proj_b jsonb := jsonb_build_object('repo_keys', jsonb_build_array('github.com/ztest-de/one'),
                     'values', jsonb_build_object('sessions', 1, 'tokens', 300, 'sessions_without_tokens', 0, 'prompts', 2),
                     'reasons', '{}'::jsonb);
    spr    jsonb := jsonb_build_object('repo_keys', jsonb_build_array('github.com/ztest-de/one'),
                     'values', jsonb_build_object('sessions', 1, 'active_secs', 60, 'active_n', 1),
                     'reasons', '{}'::jsonb);
    effort_a jsonb;
    -- Each install's id: ola's laptop and desktop, ada's one machine.
    ola_1  constant uuid := '5a000000-0000-4000-8000-000000000001';
    ola_2  constant uuid := '5a000000-0000-4000-8000-000000000002';
    ada_1  constant uuid := '5b000000-0000-4000-8000-000000000001';
    src    uuid := ola_1;
begin
    -- A (ola) reports AUTH-10 and AUTH-2 and the project; B (ada) AUTH-10 only.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000532"}', true);
    effort_a := jsonb_build_object(
        'source', src,
        'project', proj_a,
        'sprints', jsonb_build_array(spr || '{"external_id": "101"}'),
        'tickets', jsonb_build_array(row_a || '{"key": "AUTH-10"}', row_a || '{"key": "AUTH-2"}'));
    out := dojo.file_delivery(t, one, base, effort_a);
    if out is distinct from '{"facts": "accepted", "effort": "accepted"}'::jsonb then raise exception 'A: %', out; end if;

    -- No effort: the facts alone, as before.
    out := dojo.file_delivery(t, one, base, null);
    if out is distinct from '{"facts": "accepted"}'::jsonb then raise exception 'facts only: %', out; end if;

    -- A row naming a repository the push did not → refused, nothing changes.
    begin
        perform dojo.file_delivery(t, one, base, jsonb_build_object('source', src, 'project', null, 'sprints', '[]'::jsonb,
            'tickets', jsonb_build_array(jsonb_set(row_a, '{repo_keys}', '["github.com/ztest-de/two"]') || '{"key": "AUTH-3"}')));
        raise exception 'a row outside the request''s repositories was filed';
    exception when invalid_parameter_value then null;
    end;
    -- A value every reader's merge would fail on, a key off the wire, or a decoy row
    -- with no session: refused in the database, whoever calls.
    foreach out in array array[
        jsonb_set(row_a, '{values,work_started_at}', '"zzz"'),
        jsonb_set(row_a, '{values,hourly_rate}', '90'),
        jsonb_set(row_a, '{values,sessions}', '0'),
        jsonb_set(row_a, '{values,submitted_via}', '"email"')] loop
        begin
            perform dojo.file_delivery(t, one, base, jsonb_build_object('source', src, 'project', null, 'sprints', '[]'::jsonb,
                'tickets', jsonb_build_array(out || '{"key": "AUTH-3"}')));
            raise exception 'an off-wire effort row was filed: %', out;
        exception when invalid_parameter_value then null;
        end;
    end loop;
    begin
        perform dojo.file_delivery(t, one, base, jsonb_build_object('source', src, 'project', row_a, 'sprints', '[]'::jsonb, 'tickets', '[]'::jsonb));
        raise exception 'a project row with ticket fields was filed';
    exception when invalid_parameter_value then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000531"}', true);
    src := ada_1;
    -- B's facts are lesser (incomplete): stale, yet B's effort is filed.
    out := dojo.file_delivery(t, one, jsonb_set(base, '{facts,pending_tickets}', '3'),
        jsonb_build_object('source', src, 'project', proj_b, 'sprints', jsonb_build_array(spr || '{"external_id": "101"}'),
            'tickets', jsonb_build_array(row_b || '{"key": "AUTH-10"}')));
    if out is distinct from '{"facts": "stale", "effort": "accepted"}'::jsonb then raise exception 'B: %', out; end if;

    select b.board_id into board from dojo.workspace_delivery_boards(w1) b;

    -- Gate 14/18: two contributors on AUTH-10, merged; one on AUTH-2, hidden (B16).
    select * into r from dojo.workspace_delivery_tickets(w1, board, null, false, 100, 0) x where x.key = 'AUTH-10';
    if r.contributors is distinct from 2 or r.last_report is null then raise exception 'AUTH-10 contributors: %', r; end if;
    if (r.team #>> '{values,worked_minutes}')::numeric is distinct from 90
       or (r.team #>> '{values,work_started_at}')::timestamptz is distinct from '2026-11-02T08:00:00Z'
       or (r.team #>> '{values,submitted_at}')::timestamptz is distinct from '2026-11-02T10:00:00Z'
       or (r.team #>> '{values,execution_hours}')::numeric is distinct from 2
       or round((r.team #>> '{values,build_failure_rate}')::numeric, 4) is distinct from round(2.0 / 6 * 100, 4) then
        raise exception 'AUTH-10 merged: %', r.team; end if;
    if r.team -> 'values' ? 'effort_cost' then raise exception 'cost shown to a reader: %', r.team; end if;
    select * into r from dojo.workspace_delivery_tickets(w1, board, null, false, 100, 0) x where x.key = 'AUTH-2';
    if r.contributors is distinct from 1 or r.team is not null or r.last_report is not null then
        raise exception 'one contributor''s figures or report time shown: %', r; end if;
    select * into r from dojo.workspace_delivery_sprints(w1, board, 100, 0) x where x.external_id = '101';
    if r.contributors is distinct from 2 or (r.team #>> '{values,sessions}')::numeric is distinct from 2 then raise exception 'sprint 101: %', r; end if;
    select * into r from dojo.workspace_delivery_boards(w1) x;
    -- Per ticket over the board's tickets with a row from anyone: AUTH-10, AUTH-2.
    if r.contributors is distinct from 2 or (r.team #>> '{values,tokens_per_ticket}')::numeric is distinct from 600
       or (r.team #>> '{values,prompts_per_ticket}')::numeric is distinct from 3 then
        raise exception 'project: %', r; end if;

    -- One person on a second machine: one more contributor of nobody, rows added up,
    -- and neither install's push replaces the other's rows.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000532"}', true);
    src := ola_2;
    perform dojo.file_delivery(t, one, base, jsonb_build_object('source', src, 'project', null, 'sprints', '[]'::jsonb,
        'tickets', jsonb_build_array(row_a || '{"key": "AUTH-10"}')));
    select * into r from dojo.workspace_delivery_tickets(w1, board, null, false, 100, 0) x where x.key = 'AUTH-10';
    if r.contributors is distinct from 2 or (r.team #>> '{values,worked_minutes}')::numeric is distinct from 150 then
        raise exception 'a second install: %', r; end if;
    select * into r from dojo.workspace_delivery_tickets(w1, board, null, false, 100, 0) x where x.key = 'AUTH-2';
    if r.contributors is distinct from 1 or r.team is not null then raise exception 'ola alone on AUTH-2 is one contributor: %', r; end if;

    -- Gate 16: A pushes AUTH-10 only from the first machine → its AUTH-2 row is gone.
    src := ola_1;
    perform dojo.file_delivery(t, one, base, jsonb_build_object('source', src, 'project', proj_a, 'sprints', '[]'::jsonb,
        'tickets', jsonb_build_array(row_a || '{"key": "AUTH-10"}')));
    select * into r from dojo.workspace_delivery_tickets(w1, board, null, false, 100, 0) x where x.key = 'AUTH-2';
    if r.contributors is distinct from 0 then raise exception 'a replaced push kept A''s AUTH-2 row: %', r; end if;
    select * into r from dojo.workspace_delivery_tickets(w1, board, null, false, 100, 0) x where x.key = 'AUTH-10';
    if (r.team #>> '{values,worked_minutes}')::numeric is distinct from 150 then
        raise exception 'one install''s push replaced the other''s rows: %', r; end if;
    begin
        perform dojo.file_delivery(t, one, base, jsonb_build_object('project', null, 'sprints', '[]'::jsonb, 'tickets', '[]'::jsonb));
        raise exception 'effort without its install was filed';
    exception when invalid_parameter_value then null;
    end;
    select * into r from dojo.workspace_delivery_sprints(w1, board, 100, 0) x where x.external_id = '101';
    if r.contributors is distinct from 1 or r.team is not null or r.last_report is not null then
        raise exception 'a replaced push kept A''s sprint row, or one contributor''s sprint is shown: %', r; end if;
end
$$;
reset role;

rollback;
