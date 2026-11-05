-- `dojo.identities.handle` — the forge login last seen at sign-in, stored so a
-- member with no profile name still reads as a handle instead of words (F4a).
-- Renameable: never the identity (that's `subject`), refreshed on each
-- sign-in like `display_name`. Null when the provider supplies no login
-- (Azure) or the sign-in predates handles — never backfilled, never invented.
--
-- WHAT BREAKS THIS TEST: the column missing, renamed, non-nullable, or
-- carrying a default; the comment dropped.
--
-- Run with the identities DDL applied (`run.sh --apply dojo
-- ../ddl/table/dojo/identities.ddl dojo/identities_handle.sql`), or against a
-- database the DDL has deployed to: opens its own transaction, asserts,
-- rolls back.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('f1111111-1111-1111-1111-111111111111', 'f2222222-2222-2222-2222-222222222222', null);

insert into dojo.identities (principal_id, provider, subject, email, display_name, handle) values
  ('f1111111-1111-1111-1111-111111111111', 'github_oauth', '4242', null, null, 'octocat');

insert into dojo.identities (principal_id, provider, subject) values
  ('f1111111-1111-1111-1111-111111111111', 'sso', 'sso-sub-1');

do $$
declare
  v_handle text;
  v_null text;
  n int;
begin
  -- 1. The column exists, nullable, no default.
  select count(*) into n
    from information_schema.columns
   where table_schema = 'dojo' and table_name = 'identities'
     and column_name = 'handle' and data_type = 'text'
     and is_nullable = 'YES' and column_default is null;
  if n <> 1 then
    raise exception 'identities.handle must be a nullable text with no default.';
  end if;

  -- 2. Roundtrip: what provisioning wrote reads back.
  select handle into v_handle
    from dojo.identities
   where provider = 'github_oauth' and subject = '4242';
  if v_handle is distinct from 'octocat' then
    raise exception 'handle should read octocat, saw %.', v_handle;
  end if;

  -- 3. No handle written, no handle invented.
  select handle into v_null
    from dojo.identities
   where provider = 'sso' and subject = 'sso-sub-1';
  if v_null is not null then
    raise exception 'an identity without a handle must read NULL, saw %.', v_null;
  end if;

  -- 4. The column carries its comment.
  select count(*) into n
    from pg_catalog.pg_description d
    join pg_catalog.pg_class c on c.oid = d.objoid
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
    join pg_catalog.pg_attribute a on a.attrelid = c.oid and a.attnum = d.objsubid
   where s.nspname = 'dojo' and c.relname = 'identities' and a.attname = 'handle'
     and d.description is not null;
  if n <> 1 then
    raise exception 'identities.handle must carry a comment.';
  end if;
end $$;

rollback;
