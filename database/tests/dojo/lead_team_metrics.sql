-- A lead sees their team's per-person metrics (#307 R8, D38).
--
-- | row                                   | the lead (tenant role lead) sees it? |
-- |---------------------------------------|--------------------------------------|
-- | own user row, a repository they're on | yes (own)                            |
-- | a teammate's row, a repository the    | YES — D38                            |
-- |   lead holds a role on                |                                      |
-- | a teammate's row, a repository the    | no — access lives on repositories    |
-- |   lead holds no role on               |   (D5)                               |
-- | a non-teammate's row                  | no — "team members" are the people   |
-- |                                       |   on a team the lead is on           |
-- | a teammate whose membership is        | no — a team member is an ACTIVE      |
-- |   disabled                            |   member of the dōjō                 |
-- | a teammate's row in another dōjō      | no — the lead role and the team are  |
-- |   where the lead is not a lead        |   both the row's dōjō's              |
--
-- And the pairs below the cell: a tenant MAINTAINER on the same team reads no
-- teammate's row (the lead role is the floor), and a contributor on the team
-- reads neither the lead's nor a peer's. Viewers stay aggregate-only (D6) —
-- pinned by viewer_audit.sql.
--
-- dojo.team_member_metrics, which the lead's console reads, is SECURITY
-- INVOKER: it returns only what this policy lets the caller read.
--
-- WHAT BREAKS THIS TEST: a lead clause that skips the repository check, the team
-- check, the member's active membership, or the tenant binding; a floor below
-- lead; team_member_metrics running as definer.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000801', 'a1000000-0000-4000-8000-000000000801', 'lena (lead)'),
  ('a0000000-0000-4000-8000-000000000802', 'a1000000-0000-4000-8000-000000000802', 'mo'),
  ('a0000000-0000-4000-8000-000000000803', 'a1000000-0000-4000-8000-000000000803', 'mia'),
  ('a0000000-0000-4000-8000-000000000804', 'a1000000-0000-4000-8000-000000000804', 'ned (other team)'),
  ('a0000000-0000-4000-8000-000000000805', 'a1000000-0000-4000-8000-000000000805', 'max (maintainer)'),
  ('a0000000-0000-4000-8000-000000000806', 'a1000000-0000-4000-8000-000000000806', 'uma (a U team only)'),
  ('a0000000-0000-4000-8000-000000000807', 'a1000000-0000-4000-8000-000000000807', 'gus (left)'),
  ('a0000000-0000-4000-8000-000000000809', 'a1000000-0000-4000-8000-000000000809', 'ada (admin)');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000080a', 'organization/ztest-r8-t', 'organization', 'ztest-r8-t', 'R8 T', 'x'),
  ('b0000000-0000-4000-8000-00000000080b', 'organization/ztest-r8-u', 'organization', 'ztest-r8-u', 'R8 U', 'x');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000801', 'lead',        'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000802', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000803', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000804', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000805', 'maintainer',  'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000806', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000807', 'contributor', 'employer', 'github_oauth', now()),
  ('b0000000-0000-4000-8000-00000000080a', 'a0000000-0000-4000-8000-000000000809', 'admin',       'employer', 'github_oauth', null),
  -- In U the lead is a contributor, on a team with mo.
  ('b0000000-0000-4000-8000-00000000080b', 'a0000000-0000-4000-8000-000000000801', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080b', 'a0000000-0000-4000-8000-000000000802', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000080b', 'a0000000-0000-4000-8000-000000000806', 'contributor', 'employer', 'github_oauth', null);

-- Repositories: R1, R2 (the lead is on both), R3 (the lead is not); RU in U.
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-000000000801', 'b0000000-0000-4000-8000-00000000080a', 'github.com/ztest-r8/r1', 'r1', 'github'),
  ('90000000-0000-4000-8000-000000000802', 'b0000000-0000-4000-8000-00000000080a', 'github.com/ztest-r8/r2', 'r2', 'github'),
  ('90000000-0000-4000-8000-000000000803', 'b0000000-0000-4000-8000-00000000080a', 'github.com/ztest-r8/r3', 'r3', 'github'),
  ('90000000-0000-4000-8000-000000000804', 'b0000000-0000-4000-8000-00000000080b', 'github.com/ztest-r8/ru', 'ru', 'github');
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000801', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000802', 'a0000000-0000-4000-8000-000000000801', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000804', 'a0000000-0000-4000-8000-000000000801', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000802', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000803', 'a0000000-0000-4000-8000-000000000802', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000802', 'a0000000-0000-4000-8000-000000000803', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000805', 'write', 'contributor');

-- Team A: the lead, mo, mia, max (a tenant maintainer) and gus (left).
-- Team B: ned. Team UA (in U): the lead, mo and uma — uma shares no team of T
-- with the lead, so her T rows stay hers.
insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-000000000801', 'b0000000-0000-4000-8000-00000000080a', 'ztest-r8-a'),
  ('d0000000-0000-4000-8000-000000000802', 'b0000000-0000-4000-8000-00000000080a', 'ztest-r8-b'),
  ('d0000000-0000-4000-8000-000000000803', 'b0000000-0000-4000-8000-00000000080b', 'ztest-r8-ua');
insert into dojo.team_members (team_id, principal_id) values
  ('d0000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000801'),
  ('d0000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000802'),
  ('d0000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000803'),
  ('d0000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000805'),
  ('d0000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000807'),
  ('d0000000-0000-4000-8000-000000000802', 'a0000000-0000-4000-8000-000000000804'),
  ('d0000000-0000-4000-8000-000000000803', 'a0000000-0000-4000-8000-000000000801'),
  ('d0000000-0000-4000-8000-000000000803', 'a0000000-0000-4000-8000-000000000802'),
  ('d0000000-0000-4000-8000-000000000803', 'a0000000-0000-4000-8000-000000000806');

-- One metric, nine rows, by their last two digits.
insert into dojo.repository_metrics (id, tenant_id, repository_id, metric_id, scope, principal_id, computed_on, value)
select v.id::uuid, v.tenant::uuid, v.repo::uuid, m.id, v.scope, v.who::uuid, current_date, v.value
  from (values
    ('80000000-0000-4000-8000-000000000801', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000801', 'user', 'a0000000-0000-4000-8000-000000000802', 3), -- mo on R1
    ('80000000-0000-4000-8000-000000000802', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000802', 'user', 'a0000000-0000-4000-8000-000000000803', 5), -- mia on R2
    ('80000000-0000-4000-8000-000000000803', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000803', 'user', 'a0000000-0000-4000-8000-000000000802', 7), -- mo on R3
    ('80000000-0000-4000-8000-000000000804', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000801', 'user', 'a0000000-0000-4000-8000-000000000804', 11), -- ned on R1
    ('80000000-0000-4000-8000-000000000805', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000801', 'user', 'a0000000-0000-4000-8000-000000000801', 13), -- the lead on R1
    ('80000000-0000-4000-8000-000000000806', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000801', 'user', 'a0000000-0000-4000-8000-000000000807', 17), -- gus (left) on R1
    ('80000000-0000-4000-8000-000000000807', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000801', 'repo', null, 19),                                  -- R1, repo-scoped
    ('80000000-0000-4000-8000-000000000808', 'b0000000-0000-4000-8000-00000000080b', '90000000-0000-4000-8000-000000000804', 'user', 'a0000000-0000-4000-8000-000000000802', 23), -- mo on RU (U)
    ('80000000-0000-4000-8000-000000000809', 'b0000000-0000-4000-8000-00000000080a', '90000000-0000-4000-8000-000000000801', 'user', 'a0000000-0000-4000-8000-000000000806', 29)  -- uma on R1
  ) as v(id, tenant, repo, scope, who, value)
 cross join (select id from sensei.metrics order by key limit 1) m;

set local role authenticated;

create temp table expect (who text, sub text, want text) on commit drop;
insert into expect values
  ('the lead',  'a1000000-0000-4000-8000-000000000801', '01,02,05,07'),
  ('mo',        'a1000000-0000-4000-8000-000000000802', '01,03,07,08'), -- own rows + R1's shared row; not the lead's, not mia's
  ('max',       'a1000000-0000-4000-8000-000000000805', '07'),          -- tenant maintainer on the team: below the lead floor
  ('ned',       'a1000000-0000-4000-8000-000000000804', '04'),
  ('the admin', 'a1000000-0000-4000-8000-000000000809', '01,02,03,04,05,06,07,09');

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(id::text, 2), ',' order by id), '') into got
          from dojo.repository_metrics
         where id::text like '80000000-0000-4000-8000-0000000008%';
        if got is distinct from r.want then
            raise exception 'lead team metrics: % should see [%], saw [%]', r.who, r.want, got;
        end if;
    end loop;
end $$;

-- The console's read, as the lead: asked for the whole team and ned, it returns
-- only the rows the policy grants — mo's R1 value (not R3's), mia's, and the
-- lead's own; never ned's, uma's or gus's.
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000801"}', true);
do $$
declare
    got text;
begin
    select string_agg(right(principal_id::text, 3) || '=' || total::int || '/' || repositories, ',' order by principal_id)
      into got
      from dojo.team_member_metrics(
             'b0000000-0000-4000-8000-00000000080a',
             array['a0000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000802',
                   'a0000000-0000-4000-8000-000000000803', 'a0000000-0000-4000-8000-000000000804',
                   'a0000000-0000-4000-8000-000000000806', 'a0000000-0000-4000-8000-000000000807']::uuid[],
             array[(select key from sensei.metrics order by key limit 1)],
             current_date - 7);
    if got is distinct from '801=13/1,802=3/1,803=5/1' then
        raise exception 'team_member_metrics as the lead returned [%], expected [801=13/1,802=3/1,803=5/1]', got;
    end if;
end $$;

-- A contributor asking the same question learns only their own number.
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000803"}', true);
do $$
declare
    got text;
begin
    select string_agg(right(principal_id::text, 3), ',' order by principal_id) into got
      from dojo.team_member_metrics(
             'b0000000-0000-4000-8000-00000000080a',
             array['a0000000-0000-4000-8000-000000000801', 'a0000000-0000-4000-8000-000000000802',
                   'a0000000-0000-4000-8000-000000000803']::uuid[],
             array[(select key from sensei.metrics order by key limit 1)],
             current_date - 7);
    if got is distinct from '803' then
        raise exception 'team_member_metrics as a contributor returned [%], expected [803]', got;
    end if;
end $$;

reset role;
rollback;
