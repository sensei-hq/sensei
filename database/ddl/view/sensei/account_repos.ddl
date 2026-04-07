set search_path to sensei, extensions;

-- View: sensei.account_repos
-- Exposes all repos owned by any member of an account.
-- Security: security_invoker=false lets the view bypass per-user repo RLS so
-- admins can see all members' repos; the WHERE clause enforces membership via
-- is_account_member() which still resolves auth.uid() from the caller's JWT.
CREATE OR REPLACE VIEW sensei.account_repos AS
SELECT
  a.id               AS account_id
, a.slug             AS account_slug
, r.id               AS repo_id
, r.name
, r.remote_url
, r.stack
, r.last_indexed_at
, r.is_public
, r.owner_id
, r.created_at
FROM core.accounts a
JOIN core.profile_accounts pa ON pa.account_id = a.id
JOIN sensei.repos r            ON r.owner_id    = pa.user_id
WHERE public.is_account_member(a.id);
