-- Add user ownership to repos
alter table sensei.repos
  add column if not exists user_id uuid references auth.users(id) on delete set null;

-- Assign existing repos to dev@example-corp.com
update sensei.repos
set user_id = (select id from auth.users where email = 'dev@example-corp.com')
where user_id is null;
