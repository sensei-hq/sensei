set search_path to dojo, extensions;

-- A connection that goes — deleted, or with its dōjō — takes its Vault secret
-- with it (#308 R5b, D42), so no token outlives the connection it was pasted for.
create or replace function dojo.tenant_connections_forget_token()
returns trigger
language plpgsql
security definer
set search_path = dojo, pg_temp
as $$
begin
  if old.token_secret_id is not null then
    delete from vault.secrets s where s.id = old.token_secret_id;
  end if;
  return old;
end;
$$;

revoke all on function dojo.tenant_connections_forget_token() from public, anon, authenticated;

comment on function dojo.tenant_connections_forget_token() is
'Trigger: delete a deleted connection''s Vault secret (#308 R5b).';

drop trigger if exists tenant_connections_forget_token on dojo.tenant_connections;
create trigger tenant_connections_forget_token
  after delete on dojo.tenant_connections
  for each row
  execute function dojo.tenant_connections_forget_token();
