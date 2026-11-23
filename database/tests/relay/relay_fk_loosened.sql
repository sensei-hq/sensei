-- Live references survive archival (#393): `relay.items.added_by` and
-- `relay.actions.reply_to` are plain ids with no foreign key, so a live row
-- may name a lane or action that has moved to the archive.
begin;

\ir ../fixtures/relay_apply.ddl

do $$
declare
  u uuid := '01938931-0000-7000-8000-0000000000a1';
  ghost uuid := '01938931-0000-7000-8000-00000000ff01';
begin
  -- A backlog item added by a lane that is nowhere live.
  insert into relay.items (user_id, id, repo_key, title, kind, state, added_by)
  values (u, '01938931-0000-7000-8000-00000000ff02', 'github.com/o/r',
          'Item', 'gap', 'backlog', ghost);

  -- An action answering an action that is nowhere live.
  insert into relay.crews (user_id, id, status)
  values (u, '01938931-0000-7000-8000-0000000000c1', 'active');
  insert into relay.actions (user_id, id, crew_id, kind, reply_to)
  values (u, '01938931-0000-7000-8000-00000000ff03',
          '01938931-0000-7000-8000-0000000000c1', 'answered', ghost);
end $$;

rollback;
