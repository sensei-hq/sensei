set search_path to hive, sensei, extensions;

create table if not exists hive.audit_log (
  id         bigserial    primary key
, ts         timestamptz  not null default now()
, member_id  uuid         references hive.members(id)
, action     text         not null
, target     text
, detail     jsonb        not null default '{}'
);

comment on table hive.audit_log is
'Append-only audit of mutating API actions, stamped by the auth middleware.';
