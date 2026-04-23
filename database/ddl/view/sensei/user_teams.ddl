set search_path to sensei, extensions;

create or replace view sensei.user_teams as
select
  tm.user_id,
  t.slug,
  t.display_name,
  a.slug as account_slug,
  tm.role
from core.team_members tm
join core.teams t on t.id = tm.team_id
join core.accounts a on a.id = t.account_id;

comment on view sensei.user_teams is
'View: sensei.user_teams.
Exposes core team membership for the current user via the /data endpoint.';