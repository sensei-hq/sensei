set search_path to core, extensions;

create table if not exists account_keys (
  account_id  uuid  not null references core.accounts(id) on delete cascade
, wrapped_dek bytea not null
, primary key (account_id)
);

comment on table account_keys is
'Stores the wrapped Data Encryption Key (DEK) for each account.
- wrapped_dek: DEK encrypted with the account KEK (AES-256-GCM), never decrypted server-side
- One row per account; deleted when account is deleted';
