set search_path to dojo, extensions;

-- Advance dojo.relay_inbox.seq on every UPDATE so a daemon polling
-- `GET relay/inbox?since=<seq>` re-surfaces a row that changed *after* it was
-- created — above all the pending→answered transition (the human's reply). The
-- INSERT column default (nextval) covers new rows; this trigger covers mutations.
-- Mirrors the artifact contract's "seq advanced on every write" (dojo.artifacts.seq),
-- but enforced in-DB so it holds regardless of which route/writer touches the row.
create or replace function dojo.relay_inbox_bump_seq()
returns trigger
language plpgsql
set search_path = dojo, extensions
as $$
begin
  new.seq := nextval('dojo.relay_inbox_seq');
  return new;
end;
$$;

drop trigger if exists relay_inbox_seq_bump on dojo.relay_inbox;
create trigger relay_inbox_seq_bump
  before update on dojo.relay_inbox
  for each row
  execute function dojo.relay_inbox_bump_seq();
