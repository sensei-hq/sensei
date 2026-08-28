set search_path to dojo, sensei, extensions;

-- The reason vocabulary, readable from the dōjō's API.
--
-- Same sanctioned pattern as dojo.metric_catalogue: `sensei` is deliberately NOT
-- exposed to PostgREST (its daemon tables have RLS disabled), so a dojo view
-- qualifies sensei.* internally. The BASE table also carries a grant — the view's
-- alone is not enough under security_invoker, which is what made
-- metric_catalogue answer "permission denied" to a service_role Worker.
create or replace view dojo.reason_codes
with (security_invoker = on)
as
select r.domain
     , r.code
     , r.kind
     , r.precedence
     , r.summary
     , r.detail
     , r.remedy
     , r.actor
  from sensei.reason_codes r;

comment on view dojo.reason_codes is
'Human-readable reasons, keyed (domain, code). Reporting data only — the domain
decides which code applies; this says what it means and who can act.

Joined LEFT, always: a code with no row here must surface as the raw code, never
remove the row that carries it. Dropping a repository from a sync-decision view
is worse than showing an untranslated string.';

grant select on dojo.reason_codes to authenticated;
