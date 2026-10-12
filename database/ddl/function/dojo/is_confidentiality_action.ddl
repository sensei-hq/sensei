set search_path to dojo, extensions;

-- Whether an audit action belongs to the client confidentiality ledger — the
-- ONE place these actions are named. The lead's read policy on
-- dojo.audit_events and the dojo.confidentiality_ledger view both ask it.
--   publish            a lesson crossed the boundary (source-stripped by construction)
--   contained / held   the guard blocked a would-be leak (the guard working)
create or replace function dojo.is_confidentiality_action(p_action text)
    returns boolean
    language sql
    immutable
    set search_path = dojo, pg_temp
as $$
    select p_action in ('publish', 'contained', 'held');
$$;

grant execute on function dojo.is_confidentiality_action(text) to authenticated;

comment on function dojo.is_confidentiality_action(text) is
'Whether an audit action is a client-confidentiality event (publish, contained,
held) — the single definition behind the lead''s audit read and the ledger view.';
