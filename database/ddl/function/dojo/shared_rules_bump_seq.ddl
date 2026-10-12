set search_path to dojo, extensions;

-- Advance the federation pull cursor on every UPDATE, not only on insert —
-- the precedent is dojo.relay_inbox_bump_seq. A puller resumes from
-- `seq > since`; a row whose seq stayed at its insert value when it was
-- tombstoned or republished would never
-- reach a puller already past it (#236; database/tests/dojo/federation_seq_advances.sql).
create or replace function dojo.shared_rules_bump_seq()
returns trigger
language plpgsql
set search_path = dojo, extensions
as $$
begin
  new.seq := nextval('dojo.shared_rules_seq');
  return new;
end;
$$;

drop trigger if exists shared_rules_seq_bump on dojo.shared_rules;
create trigger shared_rules_seq_bump
  before update on dojo.shared_rules
  for each row
  execute function dojo.shared_rules_bump_seq();
