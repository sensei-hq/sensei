-- `sensei.assistants` is retired (#455): nothing ever read or wrote it, and
-- `sensei.assistant_installs` (keyed by the seeded catalogue) replaces it.
--
-- The daemon's reconcile does not prune a table whose DDL file is gone (it runs
-- without `--prune`), so the table would linger on every upgraded machine. It is
-- dropped here, but only while empty: a row in it is data nobody expected, and
-- the deploy refuses rather than deleting it. A no-op once gone.
do $$
declare
  n bigint;
begin
  if to_regclass('sensei.assistants') is null then
    return;
  end if;
  execute 'select count(*) from sensei.assistants' into n;
  if n > 0 then
    raise exception 'sensei.assistants is retired (#455) but holds % row(s); nothing writes it, so inspect them before deploying again.', n
      using hint = 'Copy what you need, empty the table, then deploy again.';
  end if;
  drop table sensei.assistants;
end $$;
