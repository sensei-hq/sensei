set search_path to dojo, extensions;

-- A tenant's billing account. One per tenant. Provider-agnostic: no payment
-- provider is wired yet (D-BILLING = schema + route only), so external_customer_ref
-- stays NULL until a provider is chosen. The billable unit is the per-seat count
-- from dojo.seats (unique active users on private projects); seats_used here is a
-- cached snapshot the billing route refreshes, with dojo.tenant_seat_usage as the
-- live source of truth.
create table if not exists dojo.billing_accounts (
  id                   uuid                primary key default gen_random_uuid()
, tenant_id            uuid                not null references dojo.tenants(id) on delete cascade
, plan                 text                not null default 'free'   -- free | team | … (label only until a provider is wired)
, status               dojo.billing_status not null default 'active'
, seats_included       integer             not null default 0        -- seats the plan covers (0 = free/unbounded-by-plan)
, seats_used           integer             not null default 0        -- cached COUNT(DISTINCT user_id) of active seats on private projects
, seats_computed_at    timestamptz                                   -- when seats_used was last refreshed from dojo.tenant_seat_usage
, external_customer_ref text                                         -- the payment provider's customer id; NULL until a provider is chosen
, period_start         date
, period_end           date
, created_at           timestamptz         not null default now()
, updated_at           timestamptz         not null default now()
, constraint billing_accounts_tenant_unique unique (tenant_id)
);

comment on table dojo.billing_accounts is
'A tenant''s billing account (one per tenant). Per-seat model: the billable count
is unique active users on the tenant''s private projects (dojo.seats via
dojo.tenant_seat_usage). Provider-agnostic — external_customer_ref and any charge
wiring are deferred (D-BILLING is schema + route only). seats_used is a cached
snapshot refreshed by the billing route; the view is the live truth.';
comment on column dojo.billing_accounts.seats_used
     is 'Cached snapshot of the tenant''s active billable seats (unique users on private projects). Refreshed from dojo.tenant_seat_usage; seats_computed_at records when.';
comment on column dojo.billing_accounts.external_customer_ref
     is 'The payment provider''s customer identifier. NULL until a provider is chosen — no provider is wired in the unattended run (D-BILLING).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.billing_accounts enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists billing_accounts_service_only on dojo.billing_accounts;
create policy billing_accounts_service_only on dojo.billing_accounts
    for all to authenticated, anon
    using (false) with check (false);
