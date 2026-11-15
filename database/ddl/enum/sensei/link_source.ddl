set search_path to sensei, extensions;

-- Where a session's reference to a ticket was found.
create type link_source as enum (
  'branch'    -- a git branch the session's turns ran on
, 'prompt'    -- the text the user typed
, 'fetch'     -- the assistant fetched the ticket through Sensei (get_ticket, D63)
, 'commit'    -- reserved: commit-message linkage is deferred, never written yet
);

comment on type link_source is
'Provenance of a session-to-ticket link. `fetch` marks the start of work (the assistant fetched the ticket; never written by a sync). `commit` is reserved for the deferred commit-message linkage and is not written by v1.';
