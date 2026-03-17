-- Add optional display metadata to shared_libs
alter table sensei.shared_libs
  add column if not exists icon_url text,
  add column if not exists category text
    check (category is null or category in ('ui','auth','api','data','test','build','other'));
