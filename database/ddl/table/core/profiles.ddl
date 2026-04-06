set search_path to core, extensions;

create table if not exists profiles (
  user_id      uuid        primary key references auth.users(id) on delete cascade
, slug         text        not null unique
, display_name text        not null
, avatar_url   text
, created_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
);

comment on table profiles is
'User profile linked to Supabase auth.users.
- slug: URL-safe username (e.g. bob-kim), unique across all users.
  Derived from display_name at signup. Used as the /[userSlug] dashboard URL.
- display_name: human-readable name shown in UI
- avatar_url: optional profile picture URL';

comment on column profiles.user_id is 'Primary key; foreign key to auth.users identifying the owning user.';
comment on column profiles.slug is 'URL-safe identifier, unique within the relevant scope.';
comment on column profiles.display_name is 'Human-readable display name shown in the UI.';
comment on column profiles.avatar_url is 'Optional URL of the user''s profile picture.';
comment on column profiles.created_at is 'Timestamp when the row was first created.';
comment on column profiles.modified_at is 'Timestamp of the last modification to this row.';
comment on column profiles.modified_by is 'Identity (user, role, or service) that last modified this row.';
