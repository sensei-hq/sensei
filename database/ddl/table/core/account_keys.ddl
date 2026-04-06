set search_path to core, extensions;

create table if not exists account_keys (
  account_id  uuid  not null references core.accounts(id) on delete cascade
, wrapped_dek bytea not null
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
, primary key (account_id)
);

comment on table account_keys is
'Stores the wrapped Data Encryption Key (DEK) for each account.
- wrapped_dek: DEK encrypted with the account KEK (AES-256-GCM), never decrypted server-side
- One row per account; deleted when account is deleted';

comment on column account_keys.account_id is 'Foreign key to accounts.id; also serves as the primary key (one DEK per account).';
comment on column account_keys.wrapped_dek is 'Data Encryption Key wrapped with the account KEK (AES-256-GCM); never decrypted server-side.';
comment on column account_keys.modified_at is 'Timestamp of the last modification to this row.';
comment on column account_keys.modified_by is 'Identity (user, role, or service) that last modified this row.';
