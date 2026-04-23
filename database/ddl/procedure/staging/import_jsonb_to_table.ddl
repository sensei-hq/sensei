set search_path to staging, extensions;

drop procedure if exists import_jsonb_to_table;

create or replace procedure import_jsonb_to_table(
  source varchar,
  target varchar
)
language plpgsql
as $$
declare
  type_definition text;
  dyn_sql text;
begin
  select string_agg(
    column_name || ' ' ||
    case
      when data_type = 'ARRAY' then regexp_replace(udt_name, '^_', '') || '[]'
      else data_type
    end,
    ', '
  )
    into type_definition
    from information_schema.columns
    where table_schema = split_part(target, '.', 1)
      and table_name = split_part(target, '.', 2);

  dyn_sql := format(
    'insert into %s select rec.* from %s, lateral jsonb_to_record(data::jsonb) as rec(%s) on conflict do nothing',
    target, source, type_definition
  );

  execute dyn_sql;
end;
$$;

comment on procedure import_jsonb_to_table is
'Bulk import JSONB data into a target table.
Dynamically constructs the target table type from information_schema.
source: staging table with a data jsonb column
target: target table in schema.table format';