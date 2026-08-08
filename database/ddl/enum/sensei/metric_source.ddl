set search_path to sensei, extensions;

-- Provenance of a stored value. estimated is never rendered as truth; money-facing
-- metrics write no row on a price miss (fail closed) rather than estimate.
create type metric_source
    as enum ('measured', 'estimated');
