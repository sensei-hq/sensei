set search_path to sensei, extensions;

drop function if exists rank_bfs cascade;

create or replace function rank_bfs(
  p_folder_id        uuid,
  p_changed_files text[]
)
returns table(file_path text, score float)
language plpgsql
stable
as $$
declare
  max_depth constant int := 3;
begin
  return query
  with recursive bfs(file_path, depth) as (
    select unnest(p_changed_files) as file_path, 0 as depth

    union

    select i.source_file, b.depth + 1
    from bfs b
    join sensei.imports i on i.target_path = b.file_path
      and i.folder_id = p_folder_id
    where b.depth < max_depth
  )
  select
    b.file_path,
    max(1.0 / (b.depth + 1.0))::float as score
  from bfs b
  where b.file_path not in (select unnest(p_changed_files))
  group by b.file_path
  order by score desc
  limit 20;
end;
$$;

comment on function rank_bfs is
'BFS traversal of import graph starting from changed files.
Returns files reachable from changed files, scored by inverse distance.
p_folder_id: scope to a specific folder
p_changed_files: array of file paths that were modified';

grant execute on function rank_bfs(uuid, text[]) to authenticated, service_role;