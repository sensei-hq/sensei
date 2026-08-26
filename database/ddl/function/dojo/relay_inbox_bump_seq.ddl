set search_path to dojo, extensions;

-- Filed under function/ and named for the FUNCTION it defines. It used to live in
-- procedure/dojo/relay_inbox_seq_bump.ddl — a file under `procedure/` that
-- defines neither a procedure nor a same-named object, with the two halves of the
-- name transposed against the function inside. dbd names an entity after its
-- FILE, so the deploy line read `procedure:dojo.relay_inbox_seq_bump` while the
-- thing created was `function dojo.relay_inbox_bump_seq()`, and grepping for
-- either one missed the other. The trigger keeps its own name.
--
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
