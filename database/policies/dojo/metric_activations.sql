-- RLS for dojo.metric_activations (#236 step 3; access matrix "Metrics").
--
-- A tenant's members read what it switched off — the sync plan needs it to tell
-- the daemon which metrics to skip for each repository.
-- Covered by database/tests/dojo/my_repositories_access.sql.
alter table dojo.metric_activations enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists metric_activations_service_only on dojo.metric_activations;

grant select on dojo.metric_activations to authenticated;

drop policy if exists metric_activations_read_member on dojo.metric_activations;
create policy metric_activations_read_member on dojo.metric_activations
    for select to authenticated
    using (dojo.is_member(tenant_id));
