set search_path to dojo, extensions;

-- A maintainer's verdict on a triage candidate. `approve` requires a
-- distribution scope; `revise` sends it back with edits; `decline` requires a
-- non-empty reason. The full audit trail lives in dojo.events.
create type dojo.decision_status
    as enum ('approve', 'revise', 'decline');
