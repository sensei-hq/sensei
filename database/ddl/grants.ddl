-- Grant PostgREST roles access to sensei schema
GRANT USAGE ON SCHEMA sensei TO anon, authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA sensei TO anon, authenticated, service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA sensei TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA sensei GRANT ALL ON TABLES TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA sensei GRANT ALL ON SEQUENCES TO anon, authenticated, service_role;
