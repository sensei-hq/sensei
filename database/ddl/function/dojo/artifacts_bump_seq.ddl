set search_path to dojo, extensions;

-- Advance the federation pull cursor on every UPDATE, not only on insert —
-- the precedent is dojo.relay_inbox_bump_seq. A puller resumes from
-- `seq > since`; a row whose seq stayed at its insert value when it was
-- promoted (submitted → published) would never
-- reach a puller already past it (#236; database/tests/dojo/federation_seq_advances.sql).
create or replace function dojo.artifacts_bump_seq()
returns trigger
language plpgsql
set search_path = dojo, extensions
as $$
begin
  new.seq := nextval('dojo.artifacts_seq');
  return new;
end;
$$;

drop trigger if exists artifacts_seq_bump on dojo.artifacts;
create trigger artifacts_seq_bump
  before update on dojo.artifacts
  for each row
  execute function dojo.artifacts_bump_seq();
