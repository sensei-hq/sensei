-- Grant PostgREST roles access to sensei schema
GRANT USAGE ON SCHEMA sensei TO anon, authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA sensei TO anon, authenticated, service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA sensei TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA sensei GRANT ALL ON TABLES TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA sensei GRANT ALL ON SEQUENCES TO anon, authenticated, service_role;

-- Grant PostgREST roles access to core schema
GRANT USAGE ON SCHEMA core TO anon, authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA core TO anon, authenticated, service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA core TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA core GRANT ALL ON TABLES TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA core GRANT ALL ON SEQUENCES TO anon, authenticated, service_role;

-- Row-level security: api_keys visible only to account members
ALTER TABLE core.api_keys ENABLE ROW LEVEL SECURITY;
DO $$ BEGIN
  CREATE POLICY "account_members_own_keys" ON core.api_keys
    USING (
      account_id IN (
        SELECT account_id FROM core.profile_accounts WHERE user_id = auth.uid()
      )
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Grant service_role access to platform schema (admin views only)
GRANT USAGE ON SCHEMA platform TO service_role;
GRANT SELECT ON ALL TABLES IN SCHEMA platform TO service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA platform GRANT SELECT ON TABLES TO service_role;
