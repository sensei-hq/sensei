set search_path to dojo, extensions;

create table if not exists dojo.notification_prefs (
  user_id       uuid        primary key
, events        jsonb       not null default '{}'
, quiet_hours   jsonb       not null default '{}'
, muted_tenants uuid[]      not null default '{}'
, created_at    timestamptz not null default now()
, updated_at    timestamptz not null default now()
);

comment on table dojo.notification_prefs is
'Per-user notification preferences — ONE table governing BOTH console notifications
(dojo.notifications) and Relay pushes (dojo.push_subscriptions). Not split in two.';

comment on column dojo.notification_prefs.events
     is 'Per-event push opt-ins, e.g. {"approvals":true,"decisions":true,"stalls":true}.';
comment on column dojo.notification_prefs.quiet_hours
     is 'Quiet-hours window: {"tz":"…","start":"22:00","end":"07:00"}.';
comment on column dojo.notification_prefs.muted_tenants
     is 'Per-Dōjō mute — tenant ids the user has silenced.';
