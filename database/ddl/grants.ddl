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

-- Grant service_role access to platform schema (admin views only)
GRANT USAGE ON SCHEMA platform TO service_role;
GRANT SELECT ON ALL TABLES IN SCHEMA platform TO service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA platform GRANT SELECT ON TABLES TO service_role;
