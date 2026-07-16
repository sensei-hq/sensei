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
