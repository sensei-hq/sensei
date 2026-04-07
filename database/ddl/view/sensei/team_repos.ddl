set search_path to sensei, extensions;

-- View: sensei.team_repos
-- Exposes all repos owned by any member of a team.
-- Security: security_invoker=false lets the view bypass per-user repo RLS;
-- the WHERE clause restricts results to teams where the caller is a member
-- OR is an account admin, resolving auth.uid() from the caller's JWT.
CREATE OR REPLACE VIEW sensei.team_repos AS
SELECT
  t.id               AS team_id
, t.slug             AS team_slug
, t.account_id
, r.id               AS repo_id
, r.name
, r.remote_url
, r.stack
, r.last_indexed_at
, r.is_public
, r.owner_id
, r.created_at
FROM core.teams t
JOIN core.team_members tm ON tm.team_id  = t.id
JOIN sensei.repos r        ON r.owner_id  = tm.user_id
WHERE
  EXISTS (
    SELECT 1 FROM core.team_members my
    WHERE my.team_id = t.id AND my.user_id = auth.uid()
  )
  OR public.has_account_role(t.account_id, 'admin');
