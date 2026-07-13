set search_path to dojo, sensei, extensions;

create table if not exists dojo.audit_log (
  id         bigserial    primary key
, ts         timestamptz  not null default now()
, member_id  uuid         references dojo.members(id)
, action     text         not null
, target     text
, detail     jsonb        not null default '{}'
);

comment on table dojo.audit_log is
'Append-only audit of mutating API actions, stamped by the auth middleware.';
