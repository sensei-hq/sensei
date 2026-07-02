set search_path to sensei, extensions;

-- Outcome verdict for a shipped change (impact log entry).
-- `pending` covers freshly logged changes that haven't been evaluated yet;
-- `success` / `mixed` / `failure` are the terminal verdicts. `mixed` sits
-- between success and failure — the change delivered value but also
-- introduced regressions worth remembering.
create type impact_verdict
    as enum ('pending', 'success', 'mixed', 'failure');
