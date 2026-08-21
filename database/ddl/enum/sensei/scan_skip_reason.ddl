set search_path to sensei, extensions;

-- Why the indexer examined a file but deliberately did NOT index it.
--
-- A `scan_state` row means "examined at this fingerprint"; a non-null
-- `skip_reason` says it was examined and intentionally not indexed. Recording
-- the fingerprint is what makes the skip STICK: without it the mtime gate has
-- nothing to compare against, so the file looks changed on every reconcile and
-- is re-enqueued forever (the pass that re-indexed 46 files every 5 minutes).
--
-- The skip is self-healing rather than permanent: it is keyed to the fingerprint,
-- so when the user fixes the file (re-encodes it to UTF-8, say) the mtime/hash
-- change and it is re-attempted automatically — no manual reset.
--
-- Actionability differs by value:
--   invalid_utf8       — the user can fix this (re-encode); surface it.
--   parse_error        — may be a real defect in the file or the adapter; surface it.
--   unsupported_format — expected and uninteresting; ignore quietly.
--   binary_content     — expected and uninteresting; ignore quietly.
--   excluded_by_config — the user asked for this; ignore quietly.
create type scan_skip_reason
    as enum (
      'unsupported_format'
    , 'binary_content'
    , 'invalid_utf8'
    , 'parse_error'
    , 'excluded_by_config'
    );
