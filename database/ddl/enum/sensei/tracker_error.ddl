set search_path to sensei, extensions;

-- Why a tracker call failed. A fixed vocabulary, never a raw error string: a
-- transport error carries the request URL and a response body can carry
-- client-confidential issue text, so neither may be persisted or returned.
-- Order and labels are pinned to `crate::tracker::TrackerError::ALL` by a test.
create type tracker_error as enum (
  'unauthorized'          -- 401: credential rejected
, 'forbidden'             -- 403: credential valid, no access to the board
, 'not_found'             -- 404: the board or a resource is gone
, 'rate_limited'          -- 429 after the retry budget
, 'http_error'            -- any other non-2xx
, 'transport'             -- network failure or the whole-sync deadline
, 'decode'                -- a response was not the expected shape
, 'redirect'              -- a 3xx; redirects are never followed
, 'token_missing'         -- no keyring entry for the connection
, 'credential_store'      -- the keyring could not be used (or timed out)
, 'unsupported_tracker'   -- no adapter registered for the kind
, 'db'                    -- a database failure during a sync
, 'bad_request'           -- 400: the provider rejected the request (e.g. a key that exists nowhere)
);

comment on type tracker_error is
'Failure code of a tracker connection (sensei.trackers.last_error) and of the tracker API. Never a raw message: transport errors carry URLs and bodies carry issue text.';
