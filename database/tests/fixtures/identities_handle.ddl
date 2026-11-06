-- Fixture ALTER for `dojo.identities.handle` (F5 ruling: the column is declared
-- in CREATE TABLE; this ALTER is what `--apply` and the test use until L0
-- converges it with `dbd reconcile`). Idempotent: `if not exists` keeps it
-- green when applied twice (a CLI `--apply` arg plus the test's own `\ir`)
-- and after converge.
alter table dojo.identities add column if not exists handle text;

comment on column dojo.identities.handle
     is 'The forge login last seen at sign-in. Renameable: never the identity (that''s subject), refreshed on each sign-in like display_name. Null when the provider supplies no login or the sign-in predates handles — never backfilled, never invented.';
