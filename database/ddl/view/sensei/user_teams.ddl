-- View: sensei.user_teams
-- Exposes core team membership for the current user via the /data endpoint.
CREATE OR REPLACE VIEW sensei.user_teams AS
SELECT
  tm.user_id,
  t.slug,
  t.display_name,
  a.slug AS account_slug,
  tm.role
FROM core.team_members tm
JOIN core.teams t ON t.id = tm.team_id
JOIN core.accounts a ON a.id = t.account_id;
