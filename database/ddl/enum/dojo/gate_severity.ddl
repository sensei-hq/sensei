set search_path to dojo, extensions;

-- How a gate interrupts the run. `blocking` = a hard-block (merge/main, destructive,
-- money/credentials, out-of-scope) — halts the unit until answered. `advisory` = the
-- common case — a logged reasoned assumption, reviewed async (never halts). See the
-- engine's progress-over-asking taxonomy.
create type dojo.gate_severity
    as enum ('blocking', 'advisory');
