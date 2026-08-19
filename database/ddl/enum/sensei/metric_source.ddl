set search_path to sensei, extensions;

-- Provenance of a stored value. estimated is never rendered as truth; money-facing
-- metrics write no row on a price miss (fail closed) rather than estimate.
-- federated = pulled from the Dōjō (a teammate's device computed it; this device
-- never scanned that commit) — the shared compute cache, not a local measurement.
create type metric_source
    as enum ('estimated', 'federated', 'measured');
