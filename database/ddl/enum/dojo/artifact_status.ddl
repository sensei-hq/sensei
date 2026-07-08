set search_path to dojo, extensions;

-- Publication state of an artifact. `submitted` = contributed, pending triage;
-- `published` = approved and live (distributes downstream to matching scopes);
-- `archived` = declined or retired (retained for history, not distributed).
create type dojo.artifact_status
    as enum ('submitted', 'published', 'archived');
