-- sensei seed data loader
-- Called by dbd import after staging tables are populated
-- Executes import procedures in dependency order

-- Phase 1: Independent catalog tables
call staging.import_repos();
call staging.import_libraries();

-- Phase 2: Event data (references repo_id from phase 1)
call staging.import_events();
call staging.import_benchmark_reports();

-- Phase 3: Core accounts and teams (no auth.users dependency)
-- profiles, profile_accounts, and team_members require real auth.users rows.
-- After reset, run: bash scripts/create-test-user.sh
call staging.import_accounts();
call staging.import_teams();

-- Legacy individual dev account (kept for MCP server compatibility)
insert into core.accounts (id, slug, display_name, account_type)
values (
  '00000002-0000-0000-0000-000000000001',
  'sensei-dev',
  'Sensei Dev Account',
  'individual'
) on conflict (id) do nothing;

-- Phase 4: Local dev repo (repo_id from .sensei/config.yaml on this machine)
-- This row must exist for the MCP server to serve get_session_context.
insert into sensei.repos (id, name, remote_url, default_branch, description, stack, is_public)
values (
  '8dd1e3de-6b3e-4b64-a9e5-7a5113c01b9d',
  'sensei',
  'https://github.com/jerrythomas/sensei',
  'main',
  'AI-powered codebase intelligence and developer analytics',
  ARRAY['TypeScript','Bun','SvelteKit','PostgreSQL'],
  false
) on conflict (id) do nothing;
