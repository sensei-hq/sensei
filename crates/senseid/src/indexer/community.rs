use std::collections::HashMap;

/// Run label propagation community detection for a folder.
/// Queries nodes and edges from the database, assigns community IDs,
/// and persists results to inference.communities + nodes.community_id.
///
/// This is the AUTHORITATIVE, deterministic step and the terminal scan barrier's
/// gate: it makes no model calls and writes each community with an honest-empty
/// description. The non-authoritative model-authored descriptions are filled in
/// afterwards, OFF this barrier, by [`enrich_community_descriptions`] — so a slow
/// or failing model can never block a folder from reaching `indexed` (spec W3).
pub async fn detect_communities_for_folder(
    pg: &crate::db::pg_store::PgStore,
    folder_id: &uuid::Uuid,
) -> Result<u32, String> {
    // Load all nodes for this folder.
    let mut nodes = pg
        .get_nodes_by_folder(folder_id)
        .await
        .map_err(|e| format!("Failed to load nodes: {}", e))?;

    if nodes.is_empty() {
        // Clear any stale communities for a now-empty folder (D4 invariant 5).
        pg.replace_communities_for_folder(folder_id, &[]).await?;
        return Ok(0);
    }

    // D4: order nodes by a STABLE natural key (independent of the random node
    // UUID) so both label propagation and the community-id remap below are
    // deterministic for an identical tree (invariant 2).
    nodes.sort_by_key(natural_key);

    // Build node index: uuid -> position (in natural-key order).
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();
    let mut node_ids: Vec<String> = Vec::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        let id = node["id"].as_str().unwrap_or("").to_string();
        id_to_idx.insert(id.clone(), i);
        node_ids.push(id);
    }

    let adjacency = build_adjacency(pg, folder_id, &id_to_idx, &nodes).await?;
    let mut labels = label_propagation(&adjacency, nodes.len(), 20);

    // Group members by raw propagation label.
    let mut groups: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        groups.entry(label).or_default().push(i);
    }

    // D4.4 singleton assignment: a node the propagation left alone (its label
    // group has one member — no calls/imports/extends/references edge AND no
    // parent inside the folder) inherits its enclosing FILE community rather than
    // being dropped, so coverage is ~100 % of nodes (invariant 5). Re-point each
    // such singleton's label at its file node's label; a lone file (no children,
    // no edges) keeps its own label and forms a one-node community.
    let singletons: Vec<usize> =
        groups.values().filter(|members| members.len() == 1).map(|members| members[0]).collect();
    if !singletons.is_empty() {
        let file_idx_by_path: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node["kind"].as_str() == Some("file"))
            .filter_map(|(i, node)| node["file_path"].as_str().map(|fp| (fp, i)))
            .collect();
        for i in singletons {
            if nodes[i]["kind"].as_str() == Some("file") {
                continue; // a lone file forms its own community — nothing to inherit
            }
            if let Some(fp) = nodes[i]["file_path"].as_str()
                && let Some(&f) = file_idx_by_path.get(fp)
                && f != i
            {
                labels[i] = labels[f];
            }
        }
        // Re-group after folding singletons into their file's label.
        groups.clear();
        for (i, &label) in labels.iter().enumerate() {
            groups.entry(label).or_default().push(i);
        }
    }

    // D4 deterministic ids: KEEP EVERY group — singletons now belong to their
    // file community, so every node carries a community_id (invariant 5). Since
    // nodes are natural-key sorted, a community's MIN member index IS its min
    // natural key; rank by it and assign community_id = 1..k, so an identical
    // tree always yields the same ids regardless of raw label values (invariant 2).
    let mut kept: Vec<(usize, Vec<usize>)> = groups
        .into_values()
        .map(|mut members| {
            members.sort_unstable();
            (members[0], members)
        })
        .collect();
    kept.sort_by_key(|(min_idx, _)| *min_idx);

    let mut assignments = Vec::with_capacity(kept.len());
    for (rank, (_min_idx, members)) in kept.iter().enumerate() {
        let label = generate_community_label(&nodes, members);
        let member_node_ids: Vec<uuid::Uuid> =
            members.iter().filter_map(|&idx| uuid::Uuid::parse_str(&node_ids[idx]).ok()).collect();
        // D4.5 god nodes: the community's top-5 members by degree (the hubs).
        //
        // Degree is counted from `adjacency`, which this function has already built
        // from every calls/imports/extends/references edge plus parent containment.
        // It used to be read from a `nodes.degree` column that a separate
        // `build_connections` barrier had to populate first — a cache of a
        // `count(*)`, whose only consumer was this loop, running immediately after
        // it. Measured live: 56 of 430,988 rows were stale and 19 were NULL, and
        // that drift produced wrong god nodes in 4 of 76,973 communities.
        //
        // The adjacency count is also the BETTER number here: it spans exactly the
        // semantic + containment edges that formed the community, where the column
        // counted every edge kind plus source-side occurrences of unresolved edges.
        //
        // Rank by degree desc, tie-break on member index asc (== natural key, since
        // nodes are sorted) so the set is deterministic for an unchanged graph.
        let mut by_degree = members.clone();
        by_degree
            .sort_by(|&a, &b| adjacency[b].len().cmp(&adjacency[a].len()).then_with(|| a.cmp(&b)));
        let god_node_ids: Vec<uuid::Uuid> = by_degree
            .iter()
            .take(5)
            .filter_map(|&idx| uuid::Uuid::parse_str(&node_ids[idx]).ok())
            .collect();
        assignments.push(crate::db::pg_store::CommunityAssignment {
            community_id: (rank + 1) as i32,
            label,
            member_node_ids,
            god_node_ids,
        });
    }

    // D4 durability: replace the folder's ENTIRE community set in one tx (kills
    // stale rows + orphaned community_ids — invariant 5).
    let community_count = assignments.len() as u32;
    pg.replace_communities_for_folder(folder_id, &assignments).await?;

    tracing::info!(
        "detect_communities: folder {} — {} communities from {} nodes",
        folder_id,
        community_count,
        nodes.len()
    );
    Ok(community_count)
}

/// Stable per-node ordering key so community detection and id assignment are
/// deterministic for an unchanged tree (D4 invariant 2). `parent_id` then `id` are
/// the final tiebreak (must-fix #8): once D5c nests symbols under their type/impl,
/// two symbols can share `(file_path, line_start, kind, name)` under DIFFERENT
/// parents — `parent_id` separates those, and `id` makes the order TOTAL so the
/// result never depends on the DB's unspecified row order for a tied key. Node ids
/// are stable across re-scans (get-or-create by fqn / upsert-then-prune), so this
/// stays deterministic across re-runs of an unchanged graph.
fn natural_key(node: &serde_json::Value) -> (String, i64, String, String, String, String) {
    (
        node["file_path"].as_str().unwrap_or("").to_string(),
        node["line_start"].as_i64().unwrap_or(-1),
        node["kind"].as_str().unwrap_or("").to_string(),
        node["name"].as_str().unwrap_or("").to_string(),
        node["parent_id"].as_str().unwrap_or("").to_string(),
        node["id"].as_str().unwrap_or("").to_string(),
    )
}

/// Build the undirected adjacency list community detection runs over.
///
/// D4.4 — two adjacency sources:
/// - **Semantic edges** `calls,imports,extends,references` (resolved only). The
///   dead `implements` kind (0 rows produced) is dropped.
/// - **Structural containment** via `parent_id`: a node is adjacent to its
///   enclosing parent (file/class/module). This clusters a file's symbols
///   together and gives a symbol with no semantic edge a path into its
///   container's community (so coverage reaches ~100 %).
async fn build_adjacency(
    pg: &crate::db::pg_store::PgStore,
    folder_id: &uuid::Uuid,
    id_to_idx: &HashMap<String, usize>,
    nodes: &[serde_json::Value],
) -> Result<Vec<Vec<usize>>, String> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];

    for kind in &["calls", "imports", "extends", "references"] {
        let edges = pg
            .get_edges_by_kind(folder_id, kind)
            .await
            .map_err(|e| format!("Failed to load {} edges: {}", kind, e))?;

        for edge in &edges {
            let src = edge["source_id"].as_str().unwrap_or("");
            let tgt = match edge["target_id"].as_str() {
                Some(t) => t,
                None => continue, // skip unresolved edges
            };

            if let (Some(&si), Some(&ti)) = (id_to_idx.get(src), id_to_idx.get(tgt)) {
                adj[si].push(ti);
                adj[ti].push(si); // undirected
            }
        }
    }

    // parent_id containment — connect each node to its enclosing parent.
    for (i, node) in nodes.iter().enumerate() {
        if let Some(pid) = node["parent_id"].as_str()
            && let Some(&pi) = id_to_idx.get(pid)
        {
            adj[i].push(pi);
            adj[pi].push(i); // undirected
        }
    }

    Ok(adj)
}

/// Label propagation: each node starts with its own label,
/// then iteratively adopts the most common label among neighbors.
fn label_propagation(adjacency: &[Vec<usize>], n: usize, max_iterations: usize) -> Vec<u32> {
    let mut labels: Vec<u32> = (0..n as u32).collect();

    for _iter in 0..max_iterations {
        let mut changed = false;

        for i in 0..n {
            if adjacency[i].is_empty() {
                continue;
            }

            // Count neighbor labels
            let mut counts: HashMap<u32, usize> = HashMap::new();
            for &neighbor in &adjacency[i] {
                *counts.entry(labels[neighbor]).or_insert(0) += 1;
            }

            // Pick the most frequent label (ties broken by smallest label)
            let best = counts
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|(label, _)| label)
                .unwrap_or(labels[i]);

            if best != labels[i] {
                labels[i] = best;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    labels
}

/// Generate a human-readable label for a community from its member nodes.
fn generate_community_label(nodes: &[serde_json::Value], members: &[usize]) -> String {
    // Count node kinds
    let mut kind_counts: HashMap<&str, usize> = HashMap::new();
    for &idx in members {
        let kind = nodes[idx]["kind"].as_str().unwrap_or("unknown");
        *kind_counts.entry(kind).or_insert(0) += 1;
    }

    let dominant_kind = kind_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(kind, _)| kind)
        .unwrap_or("mixed");

    // Use first member's file_path for context
    let first_file =
        members.first().and_then(|&idx| nodes[idx]["file_path"].as_str()).unwrap_or("unknown");

    let dir = std::path::Path::new(first_file).parent().and_then(|p| p.to_str()).unwrap_or("");

    format!("{} ({})", dominant_kind, dir)
}

/// How many communities (largest first) a single detect run will attempt to
/// describe. Bounds the synchronous, off-barrier enrichment so a huge cold repo
/// can't approach the DetectCommunities watchdog; smaller clusters stay
/// honest-empty and warm on a later re-detect.
const DESCRIPTION_ENRICH_CAP: i64 = 25;

/// D4.5 community-description enrichment — NON-authoritative and fail-open (spec
/// W3). Called AFTER `detect_communities_for_folder` has written the communities
/// and the folder has reached `indexed`, so nothing here can strand the folder:
/// every error/timeout is swallowed. For the folder's largest
/// [`DESCRIPTION_ENRICH_CAP`] communities it generates a one-line model summary
/// (cache-first, so a re-detect of an unchanged community reuses it) and stamps
/// `description` + `props.source='narration-cache'`; on any miss the honest-empty
/// NULL / `'null'` written by the authoritative step is left in place — never a
/// static template (never-fabricate).
pub async fn enrich_community_descriptions(
    pg: &crate::db::pg_store::PgStore,
    gateway: &gateway::Gateway,
    folder_id: &uuid::Uuid,
) {
    let communities = match pg
        .list_communities_with_god_nodes(folder_id, DESCRIPTION_ENRICH_CAP)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, folder = %folder_id, "enrich_community_descriptions: list failed");
            return;
        }
    };

    for (community_id, label, node_count, god_ids) in communities {
        // Facts: the cluster's hubs (name + kind, in god-node order) and size.
        let meta = pg.get_node_name_kind(&god_ids).await.unwrap_or_default();
        let by_id: HashMap<uuid::Uuid, (String, String)> =
            meta.into_iter().map(|(id, name, kind)| (id, (name, kind))).collect();
        let god: Vec<serde_json::Value> = god_ids
            .iter()
            .filter_map(|id| by_id.get(id))
            .map(|(name, kind)| serde_json::json!({ "name": name, "kind": kind }))
            .collect();
        let facts = serde_json::json!({ "label": label, "size": node_count, "god_nodes": god });

        // Only overwrite the honest-empty placeholder on a real model success.
        if let Some((text, source)) = generate_description(pg, gateway, &facts).await
            && let Err(e) =
                pg.set_community_description(folder_id, community_id, &text, source).await
        {
            tracing::warn!(error = %e, folder = %folder_id, community_id, "enrich_community_descriptions: set failed");
        }
    }
}

/// Generate one community's description via narration-cache, or honest-empty.
/// Returns `Some((prose, "narration-cache"))` only when the model authored a valid
/// summary; `None` on cache-miss + model failure / breaker back-off / validation
/// rejection — NEVER a static template. Cache-first so an unchanged community
/// reuses its copy (stable text across re-detects).
async fn generate_description(
    pg: &crate::db::pg_store::PgStore,
    gateway: &gateway::Gateway,
    facts: &serde_json::Value,
) -> Option<(String, &'static str)> {
    use crate::analysis::narration_cache::{self, CopyLimits, InsightKind};

    let copy = match narration_cache::read_cached_copy(pg, InsightKind::CommunityDescription, facts)
        .await
    {
        Some(c) => Some(c),
        None => {
            narration_cache::generate_and_cache(
                pg,
                gateway,
                InsightKind::CommunityDescription,
                facts,
                CopyLimits::default(),
            )
            .await
        }
    };
    copy.map(|c| (c.detail, "narration-cache"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn community_ids_deterministic_under_nesting() {
        // must-fix #8 (guards D4a): once D5c nests symbols under their type/impl,
        // two symbols can share (file_path, line_start, kind, name) under DIFFERENT
        // parents. The natural key — which orders nodes for the deterministic
        // community-id assignment — must separate them (parent_id) and be TOTAL (id),
        // so the result never depends on the DB's unspecified row order for a tie.
        let mk = |parent: &str, id: &str| {
            serde_json::json!({
                "file_path": "a.rs", "line_start": 5, "kind": "method", "name": "fmt",
                "parent_id": parent, "id": id,
            })
        };
        // Same (file,line,kind,name), different enclosing type → distinct keys.
        assert_ne!(
            natural_key(&mk("p-A", "n-1")),
            natural_key(&mk("p-B", "n-2")),
            "different parents disambiguate an otherwise-identical key"
        );
        // Even a full tie on parent is broken by id → the order is total.
        assert_ne!(
            natural_key(&mk("p-A", "n-1")),
            natural_key(&mk("p-A", "n-2")),
            "id breaks a full tie so the sort order is total (deterministic)"
        );
        assert!(
            natural_key(&mk("p-A", "n-1")) < natural_key(&mk("p-A", "n-2")),
            "when all else ties, ordering is by id"
        );
    }

    #[test]
    fn label_propagation_basic() {
        // Triangle: 0-1, 1-2, 0-2 → all should converge to same label
        let adj = vec![vec![1, 2], vec![0, 2], vec![0, 1]];
        let labels = label_propagation(&adj, 3, 20);
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[1], labels[2]);
    }

    #[test]
    fn label_propagation_disconnected() {
        // Two disconnected pairs: 0-1, 2-3
        let adj = vec![vec![1], vec![0], vec![3], vec![2]];
        let labels = label_propagation(&adj, 4, 20);
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[2], labels[3]);
        assert_ne!(labels[0], labels[2]);
    }

    #[test]
    fn label_propagation_isolated_nodes() {
        // Node 0 connected to 1, node 2 isolated
        let adj = vec![vec![1], vec![0], vec![]];
        let labels = label_propagation(&adj, 3, 20);
        assert_eq!(labels[0], labels[1]);
        // Node 2 keeps its original label
        assert_eq!(labels[2], 2);
    }

    #[test]
    fn generate_label_from_nodes() {
        let nodes = vec![
            serde_json::json!({"kind": "function", "file_path": "src/api/handler.rs"}),
            serde_json::json!({"kind": "function", "file_path": "src/api/routes.rs"}),
            serde_json::json!({"kind": "struct", "file_path": "src/api/types.rs"}),
        ];
        let label = generate_community_label(&nodes, &[0, 1, 2]);
        assert!(label.contains("function"));
        assert!(label.contains("src/api"));
    }
}
