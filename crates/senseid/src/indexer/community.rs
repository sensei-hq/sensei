use std::collections::HashMap;

/// Run label propagation community detection for a folder.
/// Queries nodes and edges from the database, assigns community IDs,
/// and persists results to inference.communities + nodes.community_id.
pub async fn detect_communities_for_folder(
    pg: &crate::db::pg_store::PgStore,
    folder_id: &uuid::Uuid,
) -> Result<u32, String> {
    // Load all nodes for this folder
    let nodes = pg.get_nodes_by_folder(folder_id).await
        .map_err(|e| format!("Failed to load nodes: {}", e))?;

    if nodes.is_empty() {
        return Ok(0);
    }

    // Build node index: uuid -> position
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();
    let mut node_ids: Vec<String> = Vec::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        let id = node["id"].as_str().unwrap_or("").to_string();
        id_to_idx.insert(id.clone(), i);
        node_ids.push(id);
    }

    // Load resolved edges (calls, implements, imports) for this folder
    let adjacency = build_adjacency(pg, folder_id, &id_to_idx, nodes.len()).await?;

    // Run label propagation
    let labels = label_propagation(&adjacency, nodes.len(), 20);

    // Group nodes by community label
    let mut communities: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        communities.entry(label).or_default().push(i);
    }

    // Persist to inference.communities and update nodes.community_id
    let mut community_count = 0u32;
    for (community_id, members) in &communities {
        if members.len() < 2 {
            continue; // skip singletons
        }

        // Generate label from most common node kind + first member name
        let label = generate_community_label(&nodes, members);

        pg.upsert_community(folder_id, *community_id as i32, &label, members.len() as i32).await
            .map_err(|e| format!("upsert_community failed: {}", e))?;

        // Update nodes.community_id for each member
        for &idx in members {
            let node_id = uuid::Uuid::parse_str(&node_ids[idx]).unwrap_or_default();
            pg.update_node_community(&node_id, *community_id as i32).await.ok();
        }

        community_count += 1;
    }

    tracing::info!(
        "detect_communities: folder {} — {} communities from {} nodes",
        folder_id, community_count, nodes.len()
    );
    Ok(community_count)
}

/// Build undirected adjacency list from resolved edges.
async fn build_adjacency(
    pg: &crate::db::pg_store::PgStore,
    folder_id: &uuid::Uuid,
    id_to_idx: &HashMap<String, usize>,
    n: usize,
) -> Result<Vec<Vec<usize>>, String> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for kind in &["calls", "implements", "imports"] {
        let edges = pg.get_edges_by_kind(folder_id, kind).await
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
            let best = counts.into_iter()
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

    let dominant_kind = kind_counts.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(kind, _)| kind)
        .unwrap_or("mixed");

    // Use first member's file_path for context
    let first_file = members.first()
        .and_then(|&idx| nodes[idx]["file_path"].as_str())
        .unwrap_or("unknown");

    let dir = std::path::Path::new(first_file)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");

    format!("{} ({})", dominant_kind, dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_propagation_basic() {
        // Triangle: 0-1, 1-2, 0-2 → all should converge to same label
        let adj = vec![
            vec![1, 2],
            vec![0, 2],
            vec![0, 1],
        ];
        let labels = label_propagation(&adj, 3, 20);
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[1], labels[2]);
    }

    #[test]
    fn label_propagation_disconnected() {
        // Two disconnected pairs: 0-1, 2-3
        let adj = vec![
            vec![1],
            vec![0],
            vec![3],
            vec![2],
        ];
        let labels = label_propagation(&adj, 4, 20);
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[2], labels[3]);
        assert_ne!(labels[0], labels[2]);
    }

    #[test]
    fn label_propagation_isolated_nodes() {
        // Node 0 connected to 1, node 2 isolated
        let adj = vec![
            vec![1],
            vec![0],
            vec![],
        ];
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
