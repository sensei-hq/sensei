set search_path to dojo, extensions;

-- Lifecycle of a tenant's billing account. Provider-agnostic (no payment
-- provider is wired yet — D-BILLING is schema + route only): `trialing` before
-- payment, `active` in good standing, `past_due` when a charge fails, `canceled`
-- when the subscription ends. Declared in rough lifecycle order.
create type dojo.billing_status
    as enum ('trialing', 'active', 'past_due', 'canceled');
