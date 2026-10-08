//! The Neighbourhood diagram's reads (#220): one ring's calls, the cards for
//! what the rings reached, and what the picture cannot show.
//!
//! The walk itself is `analysis::neighbourhood`, a pure function. These reads
//! only answer the question each ring step asks of the database, so a depth-2
//! picture is four indexed reads rather than the project's whole call graph.
//!
//! PROJECT-SCOPED THROUGH `sensei.folder_projects`, like every sibling diagram
//! (#211). A caller in another project is real and is not this project's
//! neighbourhood; a CALLEE in another project, or in a library, is drawn — the
//! focus really does call it — but never walked through.

use super::*;
use crate::analysis::neighbourhood::{Call, Side};
use crate::indexer::facts::Reason;
use crate::languages::fqn::sql_is_not_external;

/// One card's facts, read from `sensei.graph_nodes` so the path a reader sees
/// is the same repo-relative one every other screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeighbourNode {
    pub id: uuid::Uuid,
    pub name: String,
    pub kind: Option<String>,
    pub fqn: Option<String>,
    pub language: Option<String>,
    pub file_path: Option<String>,
    pub line_start: Option<i32>,
    /// `internal` / `external` / `unknown`, as the writer recorded it.
    pub locality: Option<String>,
}

/// Two counts the picture cannot draw, and must not be summed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unplaced {
    /// Calls the focus makes that the graph could not place: absent from the
    /// right-hand column, whatever it shows.
    pub callees: i64,
    /// Unplaced calls elsewhere in the project that NAME the focus. Some may be
    /// calls to it, so the left-hand column is a floor, not a census.
    pub named: i64,
}

type NeighbourNodeRow = (
    uuid::Uuid,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
);

fn neighbour_node(row: NeighbourNodeRow) -> NeighbourNode {
    let (id, name, kind, fqn, language, file_path, line_start, locality) = row;
    NeighbourNode {
        id,
        name: name.unwrap_or_default(),
        kind,
        fqn,
        language,
        file_path,
        line_start,
        locality,
    }
}

/// The SQL for one call site count: every `use` the indexer recorded on the
/// edge, across every file it was seen in. NOT `count(*)` of edge rows — one
/// row per (folder, source, target) is one in almost every case, which would
/// make the cap's "most-called first" an ordering by id.
const CALL_SITES: &str = "(SELECT coalesce(sum(jsonb_array_length(o.v)), 0)
                             FROM jsonb_each(e.props -> 'occurrences') o(k, v)
                            WHERE jsonb_typeof(o.v) = 'array')";

/// The reasons that leave a REAL gap, through the one owner of that partition.
/// `plumbing` and `external_boundary` are the ladder placing a site outside, not
/// losing it; counting them would make a well-placed symbol look uncertain.
fn doubtful_reasons() -> Vec<String> {
    Reason::ALL.iter().filter(|r| r.casts_doubt()).map(|r| r.as_label().to_string()).collect()
}

impl PgStore {
    /// The focus, if it is a node of this project. A node of another project is
    /// `None` — a neighbourhood drawn somewhere the reader is not standing is a
    /// wrong answer, not a wider one.
    pub async fn neighbourhood_focus(
        &self,
        project_id: &uuid::Uuid,
        node_id: &uuid::Uuid,
    ) -> Result<Option<NeighbourNode>, String> {
        let row: Option<NeighbourNodeRow> = sqlx_core::query_as::query_as(
            "SELECT id, name, kind, fqn, language, file_path, line_start, locality
               FROM sensei.graph_nodes
              WHERE id = $2 AND $1 = ANY(project_ids)",
        )
        .bind(project_id)
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("neighbourhood_focus: {e}"))?;
        Ok(row.map(neighbour_node))
    }

    /// One ring's placed calls, anchored on `frontier` at the end `side` names.
    ///
    /// Every edge row between a pair folds into one [`Call`], weighted by call
    /// sites. `far_walkable` is false for a library node or one outside the
    /// project: it is drawn and never seeds the next ring.
    pub async fn neighbourhood_hop(
        &self,
        project_id: &uuid::Uuid,
        frontier: &[uuid::Uuid],
        side: Side,
    ) -> Result<Vec<Call>, String> {
        // Two statements, not one with the anchor column spliced in, because the
        // walkability test differs — a CALLER shares the edge's folder, so it is
        // in the project by the join, while a callee may sit anywhere.
        //
        // Both are `format!`-assembled (the call-site count and the one owner of
        // the `lib·` test), so `check-sql-against-schema.py` CANNOT plan them.
        // `neighbourhood_tests` runs each against a real schema instead.
        let sql = match side {
            Side::In => format!(
                "SELECT e.source_id, e.target_id
                      , sum({CALL_SITES})::bigint
                      , bool_and({walkable})
                   FROM sensei.edges e
                   JOIN sensei.folder_projects fp ON fp.folder_id = e.folder_id
                                                 AND fp.project_id = $1
                   JOIN sensei.nodes n ON n.id = e.source_id
                  WHERE e.kind = 'calls' AND e.target_id = ANY($2)
                  GROUP BY 1, 2",
                walkable = sql_is_not_external("n.fqn"),
            ),
            Side::Out => format!(
                "SELECT e.source_id, e.target_id
                      , sum({CALL_SITES})::bigint
                      , bool_and({walkable} AND EXISTS (
                            SELECT 1 FROM sensei.folder_projects tp
                             WHERE tp.folder_id = n.folder_id AND tp.project_id = $1))
                   FROM sensei.edges e
                   JOIN sensei.folder_projects fp ON fp.folder_id = e.folder_id
                                                 AND fp.project_id = $1
                   JOIN sensei.nodes n ON n.id = e.target_id
                  WHERE e.kind = 'calls' AND e.source_id = ANY($2)
                  GROUP BY 1, 2",
                walkable = sql_is_not_external("n.fqn"),
            ),
        };
        let rows: Vec<(uuid::Uuid, uuid::Uuid, i64, bool)> = sqlx_core::query_as::query_as(&sql)
            .bind(project_id)
            .bind(frontier)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("neighbourhood_hop: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(source, target, occurrences, far_walkable)| Call {
                source,
                target,
                occurrences,
                far_walkable,
            })
            .collect())
    }

    /// The cards for every node the rings reached. Unscoped by project on
    /// purpose: a library callee is drawn, and the walk already decided who is in.
    pub async fn neighbourhood_nodes(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<Vec<NeighbourNode>, String> {
        let rows: Vec<NeighbourNodeRow> = sqlx_core::query_as::query_as(
            "SELECT id, name, kind, fqn, language, file_path, line_start, locality
               FROM sensei.graph_nodes
              WHERE id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("neighbourhood_nodes: {e}"))?;
        Ok(rows.into_iter().map(neighbour_node).collect())
    }

    /// What the picture cannot show — see [`Unplaced`].
    ///
    /// `named` matches the BARE name, because that is all an unplaced call
    /// records (`target_name`). It is a count of calls that MAY reach the focus,
    /// and the screen words it that way.
    pub async fn neighbourhood_unplaced(
        &self,
        project_id: &uuid::Uuid,
        focus: &uuid::Uuid,
        name: &str,
    ) -> Result<Unplaced, String> {
        let (callees, named): (i64, i64) = sqlx_core::query_as::query_as(
            "SELECT count(*) FILTER (WHERE e.source_id = $2)::bigint
                  , count(*) FILTER (WHERE e.target_name = $3)::bigint
               FROM sensei.edges e
               JOIN sensei.folder_projects fp ON fp.folder_id = e.folder_id
                                             AND fp.project_id = $1
              WHERE e.kind = 'calls' AND e.target_id IS NULL
                AND e.unresolved_reason = ANY($4)
                AND (e.source_id = $2 OR e.target_name = $3)",
        )
        .bind(project_id)
        .bind(focus)
        .bind(name)
        .bind(doubtful_reasons())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("neighbourhood_unplaced: {e}"))?;
        Ok(Unplaced { callees, named })
    }
}
