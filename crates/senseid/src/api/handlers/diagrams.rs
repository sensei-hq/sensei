//! Observatory · Diagrams — read-only payloads for the diagram screens.
//!
//! One handler per diagram. Each assembles nothing: the shape it returns is the
//! shape its view already produces, so the screen and any other consumer cannot
//! disagree about what a span or a level means.

use crate::analysis::neighbourhood::Side;
use crate::api::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

/// The levels the Structure diagram can be rolled up to.
///
/// Validated rather than interpolated: `level` reaches a `GROUP BY` expression,
/// and an unrecognised value must be a 400 the caller can see rather than a
/// silently narrower grouping.
const LEVELS: [&str; 3] = ["file", "module", "package"];

/// The edge kinds `sensei.structure_edges` carries. A request naming anything
/// else is a 400, not an empty graph — an empty result that means "you asked
/// for a kind that does not exist" is indistinguishable from "this project has
/// no edges", which is the distinction this screen exists to make.
const EDGE_KINDS: [&str; 5] = ["calls", "references", "imports", "implements", "extends"];

/// Defaults to `calls` alone. Measured on sensei: all five kinds at file level
/// is 3,215 edges over 1,751 nodes, which is the hairball #205 names as a wrong
/// gate. `calls` is the relationship the diagram is about; the rest are opt-in.
const DEFAULT_KINDS: &str = "calls";

#[derive(Deserialize)]
pub(crate) struct StructureQuery {
    level: Option<String>,
    kinds: Option<String>,
}

/// GET /api/projects/{id}/diagrams/structure?level=file|module|package&kinds=calls,references
///
/// Returns `{ nodes, edges, coverage, level, kinds }`.
///
/// `coverage.unplaced` is NOT decoration. Calls are under half placed on this
/// corpus, so the diagram necessarily omits more edges than it draws; a sparse
/// picture with no count beside it reads as a simple codebase rather than an
/// unresolved one.
///
/// It counts `edges.target_id IS NULL` directly, NOT `graph_placement`. An
/// earlier version asked that view for `outcome <> 'resolved'` — a predicate
/// that matched every row, because its outcomes are `placed` / `missed` /
/// `no verdict` and none of them is `resolved`. The screen therefore reported
/// the project's TOTAL edge count as its unplaced one, which is a precise-looking
/// number that says nothing. It was also 107 of the endpoint's 122 seconds.
///
/// Serve a diagram payload from the cache, or compute it and store it (#233).
///
/// ONE PLACE, because the policy is one decision and three endpoints: probe the
/// project's graph version, return a stored payload only if it was stored
/// against THAT version, otherwise compute and keep the answer.
///
/// `compute` is taken as a future rather than a closure and is LAZY — an `async`
/// block is not polled until awaited, so a cache hit never starts the 1.5-74 s
/// read it exists to avoid.
///
/// `params` must spell every input that changes the answer. A key missing the
/// grain would serve the module picture to a file request, which is a wrong
/// answer rather than a slow one — hence `format!` at each call site, beside the
/// values, instead of a helper that could fall behind a new parameter.
///
/// THE VERSION PROBE IS NOT SKIPPED ON A MISS. It costs 93 ms and it is what the
/// entry is stored under; computing first and reading the version afterwards
/// would stamp a fresh payload with a version from after the read, so a change
/// landing mid-computation would be cached as though it were already included.
async fn cached(
    state: &AppState,
    diagram: &'static str,
    project_id: &uuid::Uuid,
    params: String,
    compute: impl std::future::Future<Output = Result<serde_json::Value, StatusCode>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::api::diagram_cache::DiagramKey;

    let key = DiagramKey::new(diagram, *project_id, params);
    // A FAILED PROBE IS A 500, never a silent recompute. If the daemon cannot
    // say when the graph last changed it cannot say whether an answer is
    // current, and serving one anyway is the stale read the version exists to
    // prevent.
    let version = state
        .pg
        .project_graph_version(project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(hit) = state.diagrams.get(&key, version) {
        return Ok(Json(hit));
    }
    let payload = compute.await?;
    state.diagrams.put(key, version, payload.clone());
    Ok(Json(payload))
}

/// `?groupBy=project|repository|kind` — which ring the World diagram leads with.
#[derive(serde::Deserialize)]
pub(crate) struct WorldQuery {
    #[serde(rename = "groupBy")]
    group_by: Option<String>,
}

/// A DB error is a 500. It is never an empty payload — that would be
/// indistinguishable from an unindexed project, which is the one thing this
/// screen must not do.
pub(crate) async fn structure(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<StructureQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let level = q.level.unwrap_or_else(|| "file".to_string());
    if !LEVELS.contains(&level.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let kinds = parse_kinds(q.kinds)?;
    let project_id = resolve_project_id(&state, &id).await?;
    let params = format!("{level}|{}", kinds.join(","));

    cached(&state, "structure", &project_id, params, async {
        // CONCURRENTLY, because the three reads share nothing. Sequentially the
        // response costs their SUM; the edge query alone dominates, so waiting for
        // the other two in series adds latency for no ordering anyone needs.
        let (nodes, edges, unplaced) = tokio::try_join!(
            state.pg.structure_nodes(&project_id, &level),
            state.pg.structure_edges(&project_id, &level, &kinds),
            state.pg.structure_unplaced(&project_id, &kinds),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        // `drawn` is the payload's OWN edge count, not a second query. The line
        // sits directly beside the picture, so deriving it twice is how the two
        // come to disagree — and at a rolled-up level the number on screen is the
        // rolled-up one, which only this function knows.
        let coverage = serde_json::json!({ "drawn": edges.len() as i64, "unplaced": unplaced });

        Ok(serde_json::json!({
            "level": level,
            "kinds": kinds,
            "nodes": nodes,
            "edges": edges,
            "coverage": coverage,
        }))
    })
    .await
}

/// GET /api/projects/{id}/diagrams/world?groupBy=project|repository|kind
///
/// Containment as area: every indexed declaration, nested by what holds it.
///
/// CROSS-PROJECT, and the project in the path scopes nothing. The picture is
/// "all indexed code" and PROJECT is its outermost ring, so filtering to one
/// would leave the top level with a single circle. The id is still required and
/// still resolved, for two reasons that are not decoration: the screen lives in
/// a project window and the payload names which circle the reader is standing
/// in, and a bad id must 404 here exactly as it does on every sibling endpoint.
///
/// A DB error is a 500. It is never an empty picture — on this screen an empty
/// picture is indistinguishable from a machine that has indexed nothing.
pub(crate) async fn world(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<WorldQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::analysis::world::{Cell, GroupFirst, Placement};

    let label = q.group_by.unwrap_or_else(|| "project".to_string());
    // An unknown grouping is a 400 and never a default. Silently answering a
    // different question than the one asked is how a control appears to work
    // while doing nothing.
    let group = GroupFirst::from_label(&label).ok_or(StatusCode::BAD_REQUEST)?;
    let project_id = resolve_project_id(&state, &id).await?;
    // The project's own name, so the payload can say which circle the reader is
    // standing in. A MISS is a 404 and never an unnamed picture: the id resolved
    // a moment ago, so a project that has vanished between the two reads is an
    // inconsistency rather than an anonymous view.
    let viewing = state
        .pg
        .get_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .and_then(|p| p.get("name").and_then(|n| n.as_str().map(str::to_string)))
        .ok_or(StatusCode::NOT_FOUND)?;

    cached(&state, "world", &project_id, label.clone(), async {
        // CONCURRENTLY: the two reads share nothing, and the edge pass is the
        // slow one — waiting for it in series would add the cell read's time
        // for no ordering anyone needs.
        let (cells, placements) =
            tokio::try_join!(state.pg.world_cells(), state.pg.world_placement())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let cells: Vec<Cell> = cells
            .into_iter()
            .map(|c| Cell {
                project: c.project,
                repository: c.repository,
                folder: c.folder_id,
                is_test: c.is_test,
                declarations: c.declarations,
                documented: c.documented,
            })
            .collect();
        let placements: Vec<Placement> = placements
            .into_iter()
            .map(|p| Placement { folder: p.folder_id, edges: p.edges, missed: p.missed })
            .collect();

        let nested = crate::analysis::world::nest(&cells, &placements, group);
        Ok(serde_json::json!({
            "groupBy": group.as_label(),
            // Which circle the reader is standing in, so the screen can say
            // "you are here" on a picture that is deliberately wider than the
            // project they opened.
            "viewing": viewing,
            "units": nested.units,
            "totals": nested.totals,
        }))
    })
    .await
}

/// How far out the Neighbourhood walks. `@rokkit/graph`'s `DepthControl`
/// offers 1 to 3, and so does this — a fourth ring of a hub is a census, not a
/// portrait.
const MAX_DEPTH: u8 = 3;

/// Cards kept per ring per side. Measured 2026-10-07 on project `sensei`: a
/// symbol's distinct callers are 1 at the median, 6 at p90 and 60 at p99, so 40
/// draws ~98% of symbols whole. The rest — `assert_eq` has 2,304 callers — are
/// cut to their most-called 40 and the cut is COUNTED in the payload.
const RING_CAP: usize = 40;

#[derive(Deserialize)]
pub(crate) struct NeighbourhoodQuery {
    focus: Option<String>,
    depth: Option<u8>,
}

/// GET /api/projects/{id}/diagrams/neighbourhood?focus=<node id>&depth=1..3
///
/// One symbol, what calls it and what it calls, a ring at a time. Edges are
/// `kind: "dependency"`, which is what makes the layout word its columns
/// "called by / calls" rather than an ER diagram's "referenced by".
///
/// A missing or malformed focus is a 400 — there is no default symbol, and
/// picking one would draw a neighbourhood the reader did not ask about. A focus
/// that is not this project's is a 404.
///
/// `coverage` says what the picture CANNOT show, and both counts sit beside it:
/// `unplacedCallees` are calls the focus makes that the graph could not place
/// (absent from the right-hand column), and `namedUnplaced` are unplaced calls
/// elsewhere in the project that use the focus's NAME — some may be calls to it,
/// so the left-hand column is a floor. `cut` is the cards a capped ring found
/// and did not draw.
pub(crate) async fn neighbourhood(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<NeighbourhoodQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let depth = q.depth.unwrap_or(1);
    if !(1..=MAX_DEPTH).contains(&depth) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let focus_id = q
        .focus
        .as_deref()
        .and_then(|f| uuid::Uuid::parse_str(f).ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let project_id = resolve_project_id(&state, &id).await?;
    let focus = state
        .pg
        .neighbourhood_focus(&project_id, &focus_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    cached(&state, "neighbourhood", &project_id, format!("{focus_id}|{depth}"), async {
        let ((left, left_cut), (right, right_cut), unplaced) = tokio::try_join!(
            walk(&state, &project_id, focus_id, Side::In, depth),
            walk(&state, &project_id, focus_id, Side::Out, depth),
            async {
                state
                    .pg
                    .neighbourhood_unplaced(&project_id, &focus_id, &focus.name)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            },
        )?;

        // One line per pair, whichever walk found it — a mutual neighbour is
        // reached by both, and two lines for one relationship would double it.
        let mut edges: std::collections::BTreeMap<(uuid::Uuid, uuid::Uuid), i64> =
            std::collections::BTreeMap::new();
        for c in left.iter().chain(right.iter()) {
            edges.entry((c.source, c.target)).or_insert(c.occurrences);
        }
        let mut ids: std::collections::BTreeSet<uuid::Uuid> =
            edges.keys().flat_map(|(s, t)| [*s, *t]).collect();
        ids.insert(focus_id);
        let ids: Vec<uuid::Uuid> = ids.into_iter().collect();
        let cards = state
            .pg
            .neighbourhood_nodes(&ids)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let out_edges: Vec<serde_json::Value> = edges
            .iter()
            .map(|((s, t), w)| {
                serde_json::json!({
                    "id": format!("{s}→{t}"),
                    "source": s,
                    "target": t,
                    "kind": "dependency",
                    "weight": w,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "depth": depth,
            "focus": card(&focus),
            "nodes": cards.iter().map(card).collect::<Vec<_>>(),
            "edges": out_edges,
            "coverage": {
                "drawn": out_edges.len() as i64,
                "cut": { "in": left_cut, "out": right_cut },
                "unplacedCallees": unplaced.callees,
                "namedUnplaced": unplaced.named,
            },
        }))
    })
    .await
}

/// Walk one side `depth` rings out. Returns every call kept, and the cards cut.
async fn walk(
    state: &AppState,
    project_id: &uuid::Uuid,
    focus: uuid::Uuid,
    side: Side,
    depth: u8,
) -> Result<(Vec<crate::analysis::neighbourhood::Call>, usize), StatusCode> {
    let mut claimed = std::collections::HashSet::from([focus]);
    let mut frontier = std::collections::HashSet::from([focus]);
    let mut calls = vec![];
    let mut cut = 0;
    for _ in 0..depth {
        if frontier.is_empty() {
            break;
        }
        let ids: Vec<uuid::Uuid> = frontier.iter().copied().collect();
        let hop = state
            .pg
            .neighbourhood_hop(project_id, &ids, side)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let ring =
            crate::analysis::neighbourhood::step(&hop, &frontier, side, &mut claimed, RING_CAP);
        cut += ring.cut;
        calls.extend(ring.calls);
        frontier = ring.frontier.into_iter().collect();
    }
    Ok((calls, cut))
}

/// One card as `@rokkit/graph` reads it. `group` is the PACKAGE, through the
/// one fqn decoder, so a card's tint says which crate or package it lives in —
/// the first thing a reader wants to know about a call crossing a boundary.
fn card(n: &crate::db::pg_store::NeighbourNode) -> serde_json::Value {
    let parsed = n.fqn.as_deref().and_then(|f| crate::indexer::fqn::parse(f).ok());
    serde_json::json!({
        "id": n.id,
        "label": n.name,
        "kind": n.kind,
        "group": parsed.as_ref().map(|p| p.package),
        "module": parsed.as_ref().and_then(|p| p.tail.first().copied()),
        "language": n.language,
        "file": n.file_path,
        "line": n.line_start,
        "external": n.locality.as_deref() == Some("external"),
        "rows": [],
    })
}

/// The grains Layers and Cycles can be ranked at.
///
/// `package` is deliberately absent. It is a level the Structure diagram offers,
/// but on this corpus a project's package graph is single digits of nodes —
/// measured 2026-10-05, project `sensei` is 14 packages and 9 dependencies —
/// which has no layering to show. The mockup agrees: Layers is module-only and
/// Cycles toggles module against file (`…v8.dc.html:2450`).
///
/// `file` IS safe here despite `structure_nodes` keying file nodes on their
/// PATH rather than their id, which merges two files that share a relative
/// path. Measured 2026-10-05 before shipping it, because a merge would
/// fabricate a cycle out of two unrelated files: in project `sensei` 1,745 of
/// 3,518 file rows collide on path and ALL 1,745 are same-package — two
/// checkouts of one repository, where merging is the right answer. Across six
/// projects and 36,229 paths, cross-package collisions are ZERO.
const LAYERING_LEVELS: [&str; 2] = ["module", "file"];

#[derive(Deserialize)]
pub(crate) struct LayeringQuery {
    level: Option<String>,
    kinds: Option<String>,
}

/// GET /api/projects/{id}/diagrams/layering?level=module|file&kinds=calls
///
/// ONE endpoint for TWO screens. Layers ranks the modules and says how each
/// dependency sits against that ranking; Cycles collapses the mutually
/// dependent ones and names the weakest link to cut. They are the same
/// computation read two ways — strongly-connected components, then longest-path
/// depth over the condensation — so serving them from one payload is what stops
/// the two screens disagreeing about which modules are in a cycle.
///
/// `conformance: "up"` IS the answer to "which calls break the downward flow",
/// and every one of them is a cycle closing. Between components nothing can
/// climb — `layer(source) < layer(target)` holds by the definition of longest
/// path — but inside one, `a → b → c → a` cannot be drawn with every arrow
/// pointing down, and `analysis::layering`'s feedback order says which arrow
/// closes the loop. An earlier version marked those `level`, which is true (the
/// members share a rank) and useless, because `ViolationsControl` filters on
/// `up`: a graph whose only violations were its cycles showed none.
///
/// `layerSource` is still `derived`, and still worth saying. It no longer means
/// "a violation is not expressible" — it means this layering was MEASURED rather
/// than declared, so it answers "do these calls flow downward" and not "is this
/// the architecture you intended". The second needs a declared layering (#230).
///
/// A DB error is a 500, never an empty payload — an empty graph and an
/// unreachable database must not render the same way.
pub(crate) async fn layering(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<LayeringQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let level = q.level.unwrap_or_else(|| "module".to_string());
    if !LAYERING_LEVELS.contains(&level.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let kinds = parse_kinds(q.kinds)?;
    let project_id = resolve_project_id(&state, &id).await?;
    let params = format!("{level}|{}", kinds.join(","));

    cached(&state, "layering", &project_id, params, async {
        // The node universe comes from `structure_nodes`, NOT from the dependency
        // rows. A module with no cross-module dependency has no edge to be inferred
        // from and would silently vanish — measured on project `sensei`, 9 modules
        // appear only in a dependency on themselves and are exactly that case.
        let (nodes, deps, unplaced) = tokio::try_join!(
            state.pg.structure_nodes(&project_id, &level),
            state.pg.dependency_graph(&project_id, &level, &kinds),
            state.pg.structure_unplaced(&project_id, &kinds),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let units: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        let ranked = crate::analysis::layering::analyse(&units, &deps);

        let placed: std::collections::HashMap<&str, &crate::analysis::layering::Placed> =
            ranked.units.iter().map(|p| (p.id.as_str(), p)).collect();
        let out_nodes: Vec<serde_json::Value> = nodes
            .iter()
            .map(|n| {
                let p = placed.get(n.id.as_str());
                serde_json::json!({
                    "id": n.id,
                    "label": n.label,
                    "group": n.package,
                    // The box's weight on the diagram, same measure the Structure
                    // screen sizes by, so one module is the same size on both.
                    "weight": n.symbols,
                    "files": n.files,
                    "language": n.language,
                    "layer": p.map(|p| p.layer),
                    "component": p.map(|p| p.component),
                    "componentSize": p.map(|p| p.component_size),
                })
            })
            .collect();

        let out_edges: Vec<serde_json::Value> = ranked
            .deps
            .iter()
            .map(|d| {
                serde_json::json!({
                    "id": format!("{}→{}", d.source, d.target),
                    "source": d.source,
                    "target": d.target,
                    "kind": "dependency",
                    "weight": d.occurrences,
                    "conformance": d.conformance,
                    "weakest": d.weakest,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "level": level,
            "kinds": kinds,
            "layerSource": "derived",
            "depth": ranked.depth,
            "nodes": out_nodes,
            "edges": out_edges,
            "cycles": ranked.cycles,
            // Kept rather than dropped: at module grain this is two files of one
            // module referring to each other, which is a fact about the module. It
            // is empty by construction at file grain — `structure_edges` has no
            // file-to-itself row to carry.
            "selfDependencies": ranked.self_deps,
            "coverage": {
                "drawn": ranked.deps.len() as i64,
                "unplaced": unplaced,
                "units": out_nodes.len() as i64,
                // Dependencies omitted because an endpoint owns no file and so is
                // not a unit. NOT decoration: measured 2026-10-05 on project
                // `sensei`, 32 of 234 module dependencies are in this state,
                // because an fqn's third segment is the module for most adapters
                // and a SYMBOL for some. A diagram that drops 13.7% of its edges
                // without a number beside it reads as a sparse codebase.
                "unknownUnit": ranked.dropped.len() as i64,
            },
        }))
    })
    .await
}

/// GET /api/projects/{id}/diagrams/zones
///
/// Martin's abstractness against instability, one point per module. The main
/// sequence is `A + I = 1`; `distance` is how far off it a module sits, and
/// `zone` names where — `pain` (concrete and depended upon), `useless`
/// (abstract and depended upon by nothing), `risk`, or `main`.
///
/// No parameters. The grain is the module, which is the grain the mockup uses
/// and the only one with enough points to read a sequence off — a project here
/// is single digits of packages.
///
/// `abstractness`, `instability`, `distance` and `zone` can be NULL, and the
/// screen MUST omit those points rather than drawing them at an axis origin.
/// `coverage.unplaceable` counts them, so an omitted point is distinguishable
/// from one that genuinely sits at zero.
pub(crate) async fn zones(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_id(&state, &id).await?;

    // NO PARAMETERS, so the key is the empty string — a project's zones are one
    // picture. Stated rather than omitted: an endpoint that later grows a knob
    // and forgets this line would serve one answer for every setting.
    cached(&state, "zones", &project_id, String::new(), async {
        let points = state
            .pg
            .component_zones(&project_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Counted here rather than in SQL: it is how many points the PAYLOAD cannot
        // place, so it has to be derived from the payload or the two can disagree.
        let unplaceable = points.iter().filter(|p| p.distance.is_none()).count() as i64;
        let mean_distance = {
            let placed: Vec<f64> = points.iter().filter_map(|p| p.distance).collect();
            // `None`, not 0.0, when nothing is placeable — a mean over no points is
            // not "perfectly on the sequence".
            (!placed.is_empty()).then(|| placed.iter().sum::<f64>() / placed.len() as f64)
        };

        Ok(serde_json::json!({
            "points": points,
            "meanDistance": mean_distance,
            "coverage": { "points": points.len() as i64, "unplaceable": unplaceable },
        }))
    })
    .await
}

/// Validate and split the `kinds` query parameter.
///
/// Shared by both diagram handlers so an unknown kind is a 400 on each — an
/// empty graph that means "you asked for a kind that does not exist" is
/// indistinguishable from "this project has no edges", which is the distinction
/// these screens exist to make.
fn parse_kinds(raw: Option<String>) -> Result<Vec<String>, StatusCode> {
    let kinds: Vec<String> = raw
        .unwrap_or_else(|| DEFAULT_KINDS.to_string())
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();
    if kinds.is_empty() || !kinds.iter().all(|k| EDGE_KINDS.contains(&k.as_str())) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(kinds)
}

/// A project NAME or a UUID, matching the read-side pattern the other
/// project-scoped handlers use (`get_folder_commands`, the MCP tools).
///
/// A name that resolves to nothing, and a UUID with no row, are both 404 — the
/// caller asked about a project that is not here, which is not a server error.
async fn resolve_project_id(state: &AppState, id: &str) -> Result<uuid::Uuid, StatusCode> {
    if let Some(row) =
        state.pg.get_project_by_name(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return crate::api::util::json_uuid(&row["id"]).ok_or(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let uid = uuid::Uuid::parse_str(id).map_err(|_| StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(uid)
}
