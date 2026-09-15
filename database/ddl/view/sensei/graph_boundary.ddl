set search_path to sensei, extensions;

-- Where the code graph stops, and why — one row per unplaced use site, with the
-- prose that explains it.
--
-- The graph's honest answer to "what breaks if I change this" has two halves:
-- the callers it placed, and the use sites it could NOT place. A consumer shown
-- only the first reads a partial answer as a complete one. `call_graph` serves
-- the first half; this view serves the second, and the two are deliberately the
-- same shape so a reader can put them side by side.
--
-- Built for GROUP BY. Every column is either an identity to group on
-- (folder, project, language, source_kind, file) or prose to render
-- (summary, detail, remedy) — so "how much of this repo's graph is
-- unplaceable, by language and by reason" is one query and no joins:
--
--   select source_language, reason_code, reason_summary, count(*)
--     from sensei.graph_boundary
--    where folder_id = $1
--    group by 1, 2, 3
--    order by 4 desc;
--
-- reason_codes is joined LEFT, always. A code with no row must surface raw
-- rather than drop the site that carries it — losing a boundary from a
-- boundary report is strictly worse than showing an untranslated string, and
-- it is the failure that makes an incomplete answer look complete.
create or replace view graph_boundary as
select cg.folder_id
     , cg.folder
     , cg.project_id
     , cg.project
     , cg.edge_id
     , cg.edge_kind
     -- WHERE the unplaced use site sits: the symbol whose body holds it.
     , cg.source_id
     , cg.source_name
     , cg.source_kind
     , cg.source_file
     , cg.source_line
     , cg.source_language
     -- WHAT it names. No target_id by definition — that is what unplaced means
     -- — so this is the bare name the walk read at the site.
     , cg.target_symbol   as names
     , cg.target_file     as names_file
     -- WHY, as a code and as prose.
     , cg.unresolved_reason as reason_code
     , rc.kind            as reason_kind
     , rc.precedence      as reason_precedence
     , rc.summary         as reason_summary
     , rc.detail          as reason_detail
     , rc.remedy          as reason_remedy
     , rc.actor           as reason_actor
  from call_graph cg
  left join reason_codes rc
    on rc.domain = 'code_graph'
   and rc.code   = cg.unresolved_reason
 where cg.target_id is null;

comment on view graph_boundary is
'Every use site the resolver could not place, with the reason_codes prose that
explains it. The second half of an impact answer — `call_graph` is the first.

Shaped for GROUP BY: group on folder/project/source_language/source_kind/
reason_code, render summary/detail/remedy. reason_codes is joined LEFT so a
code with no prose surfaces raw instead of dropping the row.

reason_code is written by the indexer into edges.props->>''reason''; the
vocabulary lives in sensei.reason_codes under domain ''code_graph'' and the one
producer of these strings is Reason::as_label in crates/senseid.';

grant select on graph_boundary to authenticated, service_role;
