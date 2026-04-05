-- Bootstrap: create the schemas that PostgREST needs at startup.
-- Table/function definitions are managed by dbd apply, not migrations.
create schema if not exists sensei;
create schema if not exists core;
create schema if not exists platform;
