set search_path to dojo, extensions;

create table if not exists dojo.push_subscriptions (
  id            uuid        primary key default gen_random_uuid()
, user_id       uuid        not null
, membership_id uuid        references dojo.memberships(id)
, platform      text        not null
, web_push      jsonb
, native_token  text
, enabled       boolean     not null default true
, last_seen     timestamptz
, created_at    timestamptz not null default now()
, updated_at    timestamptz not null default now()
);

create index if not exists push_subscriptions_user_idx on dojo.push_subscriptions(user_id);
create index if not exists push_subscriptions_enabled_idx on dojo.push_subscriptions(user_id) where enabled;
-- Covering index for the membership_id FK (nullable) — avoids a seq-scan on
-- dojo.memberships deletes (Supabase unindexed_foreign_keys).
create index if not exists push_subscriptions_membership_idx on dojo.push_subscriptions(membership_id) where membership_id is not null;

comment on table dojo.push_subscriptions is
'Per user × device push registration. Defined now, unused until P4 (away-from-keyboard
push). platform = web | ios | android; web_push = {endpoint, p256dh, auth} for Web
Push (VAPID); native_token = the APNs/FCM token for the thin native wrapper. VAPID /
APNs / FCM credentials are secrets — never in git.';

comment on column dojo.push_subscriptions.platform
     is 'web | ios | android.';
comment on column dojo.push_subscriptions.web_push
     is 'Web Push subscription {endpoint, p256dh, auth} (platform=web).';
comment on column dojo.push_subscriptions.native_token
     is 'APNs/FCM device token (platform=ios/android).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.push_subscriptions enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists push_subscriptions_service_only on dojo.push_subscriptions;
create policy push_subscriptions_service_only on dojo.push_subscriptions
    for all to authenticated, anon
    using (false) with check (false);
