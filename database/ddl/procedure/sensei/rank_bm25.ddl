set search_path to sensei, extensions;

-- Keyword-based file ranking using ilike substring matching on symbol names, signatures, and docstrings.
-- Returns files ranked by how many symbols match the query terms.
-- Phase 1 implementation — full BM25 via pg_trgm can be added later.
create or replace function rank_bm25(
  p_repo_id uuid,
  p_query   text
)
returns table(file_path text, score float)
language sql
stable
as $$
  with terms as (
    -- Split query into individual words
    select lower(unnest(string_to_array(trim(p_query), ' '))) as term
  ),
  matches as (
    -- Count how many query terms each symbol matches
    select
      s.file_path,
      count(t.term) as matched_terms
    from sensei.symbols s
    cross join terms t
    where s.repo_id = p_repo_id
      and (
        lower(coalesce(s.name, ''))      like '%' || t.term || '%'
        or lower(coalesce(s.signature, '')) like '%' || t.term || '%'
        or lower(coalesce(s.docstring, '')) like '%' || t.term || '%'
      )
    group by s.file_path, t.term
  )
  select
    m.file_path,
    (sum(m.matched_terms)::float / greatest(array_length(string_to_array(trim(p_query), ' '), 1), 1)) as score
  from matches m
  group by m.file_path
  order by score desc
  limit 20;
$$;
