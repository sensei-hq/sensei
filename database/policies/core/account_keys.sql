set search_path to core, extensions;

grant all on account_keys to service_role;

-- no policies = deny all for non-service_role
alter table account_keys enable row level security;
