set search_path to dojo, extensions;

-- The metric catalogue. PRODUCT-OWNED, pulled down, never authored by a tenant.
--
-- Every metric is bound to a worker by `task_name` — there is code that computes
-- it. A tenant-authored metric would therefore have no computation and could
-- only ever be empty, which is why this is a catalogue rather than user content.
-- A team must also compute a metric identically or the comparison between them
-- is meaningless.
--
-- What a tenant DOES control is activation — see dojo.metric_activations.
-- Disabling a metric hides it AND skips its computation, which is a real saving
-- rather than a display filter.
create table if not exists metrics (
  id            uuid        primary key default gen_random_uuid()
, key           text        not null unique
, name          text        not null
, description   text        not null
, family        text        not null
, type          text        not null
, unit          text
, direction     text        not null
, task_name     text        not null
, weight        numeric     not null default 1
, target        numeric
, rating_scale  jsonb
, derives_from  text[]
  -- Lifecycle, not visibility: effective_until retires a metric for EVERYONE.
  -- Per-tenant on/off is metric_activations, a separate axis.
, effective_from  date      not null default current_date
, effective_until date
, catalogue_version bigint  not null default 0
, updated_at    timestamptz not null default now()
);

comment on table metrics is
'Product-owned metric catalogue, mirrored from the daemon''s sensei.metrics.
Pulled, never pushed: a metric exists because a worker computes it.

effective_until retires a metric globally. A tenant switching one off is
metric_activations — a different axis, and the one that also stops the work.';

alter table metrics enable row level security;
-- Readable by any signed-in user: the catalogue is product content, not tenant
-- data, and the UI needs it to render names/units for whatever it is allowed to
-- see. Writes stay service_role only.
drop policy if exists metrics_read_all on metrics;
create policy metrics_read_all on metrics for select to authenticated using (true);
drop policy if exists metrics_no_client_write on metrics;
create policy metrics_no_client_write on metrics
    for all to authenticated, anon using (false) with check (false);

-- Catalogue is readable by any signed-in user (it is product content, and the UI
-- needs names/units to render whatever it may see). The grant pairs with the
-- read policy above.
grant select on dojo.metrics to authenticated;

