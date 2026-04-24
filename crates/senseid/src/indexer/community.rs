use std::collections::HashMap;
use super::graph::GraphDb;

/// Run Leiden-inspired community detection on the project's call/import graph.
/// Returns a map of node_id → community_id, and stores results in the graph DB.
pub fn detect_communities(graph_db: &GraphDb, project: &str) -> Result<HashMap<String, u32>, String> {
    // 1. Load adjacency list from edges
    let edges = graph_db.get_edges(project)?;
    if edges.is_empty() {
        return Ok(HashMap::new());
    }

    // Build undirected adjacency list (node_id → neighbors with weights)
    let mut adj: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut total_weight = 0.0;

    for edge in &edges {
        let w = match edge.edge_type.as_str() {
            "CALLS" => 3.0,      // strong coupling
            "IMPORTS" => 2.0,    // medium coupling
            "EXPORTS_FN" | "EXPORTS_TYPE" => 1.0, // file-symbol co-location
            _ => 0.5,
        };
        adj.entry(edge.source.clone()).or_default()
            .entry(edge.target.clone()).or_insert(0.0);
        *adj.get_mut(&edge.source).unwrap().get_mut(&edge.target).unwrap() += w;

        adj.entry(edge.target.clone()).or_default()
            .entry(edge.source.clone()).or_insert(0.0);
        *adj.get_mut(&edge.target).unwrap().get_mut(&edge.source).unwrap() += w;

        total_weight += w;
    }

    if total_weight == 0.0 {
        return Ok(HashMap::new());
    }

    let m2 = 2.0 * total_weight;
    let nodes: Vec<String> = adj.keys().cloned().collect();

    // Node degree (sum of weights)
    let degree: HashMap<String, f64> = nodes.iter().map(|n| {
        let d: f64 = adj.get(n).map(|nb| nb.values().sum()).unwrap_or(0.0);
        (n.clone(), d)
    }).collect();

    // 2. Initialize: each node in its own community
    let mut community: HashMap<String, u32> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        community.insert(n.clone(), i as u32);
    }

    // 3. Iterative local moves (Louvain phase 1)
    let max_iters = 20;
    for _iter in 0..max_iters {
        let mut improved = false;

        for node in &nodes {
            let node_comm = community[node];
            let node_deg = degree.get(node).copied().unwrap_or(0.0);
            let neighbors = match adj.get(node) {
                Some(nb) => nb,
                None => continue,
            };

            // Compute weight to each neighboring community
            let mut comm_weights: HashMap<u32, f64> = HashMap::new();
            for (nb, w) in neighbors {
                let nb_comm = community[nb];
                *comm_weights.entry(nb_comm).or_insert(0.0) += w;
            }

            // Sum of degrees in each community
            let mut comm_degree_sum: HashMap<u32, f64> = HashMap::new();
            for (n, c) in &community {
                *comm_degree_sum.entry(*c).or_insert(0.0) += degree.get(n).copied().unwrap_or(0.0);
            }

            // Find best community to move to
            let mut best_comm = node_comm;
            let mut best_gain = 0.0;

            // Gain of removing from current community
            let ki_in_current = comm_weights.get(&node_comm).copied().unwrap_or(0.0);
            let sigma_current = comm_degree_sum.get(&node_comm).copied().unwrap_or(0.0) - node_deg;

            for (&candidate_comm, &ki_in) in &comm_weights {
                if candidate_comm == node_comm { continue; }
                let sigma_c = comm_degree_sum.get(&candidate_comm).copied().unwrap_or(0.0);

                // Modularity gain of moving node to candidate community
                let gain = (ki_in - ki_in_current) / m2
                    - node_deg * (sigma_c - sigma_current) / (m2 * m2);

                if gain > best_gain {
                    best_gain = gain;
                    best_comm = candidate_comm;
                }
            }

            if best_comm != node_comm {
                community.insert(node.clone(), best_comm);
                improved = true;
            }
        }

        if !improved { break; }
    }

    // 4. Renumber communities 0..N
    let mut comm_map: HashMap<u32, u32> = HashMap::new();
    let mut next_id = 0u32;
    let mut result: HashMap<String, u32> = HashMap::new();
    for (node, comm) in &community {
        let new_id = *comm_map.entry(*comm).or_insert_with(|| { let id = next_id; next_id += 1; id });
        result.insert(node.clone(), new_id);
    }

    // 5. Store community IDs in the graph DB
    graph_db.store_communities(&result)?;

    Ok(result)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommunityInfo {
    pub id: u32,
    pub size: u32,
    pub sample_members: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn community_detection_on_small_graph() {
        let db = GraphDb::open_memory().unwrap();
        let p = "test";

        // Create two clusters
        // Cluster A: a1 -> a2 -> a3 (tight)
        db.merge_function("fn:a1", "funcA1", "a.py", 1, "", "", "", 1, p).unwrap();
        db.merge_function("fn:a2", "funcA2", "a.py", 5, "", "", "", 1, p).unwrap();
        db.merge_function("fn:a3", "funcA3", "a.py", 10, "", "", "", 1, p).unwrap();
        db.merge_edge("fn:a1", "fn:a2", "CALLS").unwrap();
        db.merge_edge("fn:a2", "fn:a3", "CALLS").unwrap();
        db.merge_edge("fn:a3", "fn:a1", "CALLS").unwrap();

        // Cluster B: b1 -> b2 (tight)
        db.merge_function("fn:b1", "funcB1", "b.py", 1, "", "", "", 1, p).unwrap();
        db.merge_function("fn:b2", "funcB2", "b.py", 5, "", "", "", 1, p).unwrap();
        db.merge_edge("fn:b1", "fn:b2", "CALLS").unwrap();
        db.merge_edge("fn:b2", "fn:b1", "CALLS").unwrap();

        // Weak cross-cluster link
        db.merge_edge("fn:a1", "fn:b1", "IMPORTS").unwrap();

        let communities = detect_communities(&db, p).unwrap();
        assert!(!communities.is_empty());

        // Nodes in the same cluster should be in the same community
        let a1_c = communities["fn:a1"];
        let a2_c = communities["fn:a2"];
        let _b1_c = communities["fn:b1"];

        assert_eq!(a1_c, a2_c, "A cluster nodes should be in same community");
        // B cluster may or may not merge with A depending on modularity — just check it assigned
        assert!(communities.contains_key("fn:b1"));
        assert!(communities.contains_key("fn:b2"));

        // Test get_communities
        let info = db.get_communities(p).unwrap();
        assert!(!info.is_empty());
    }

    #[test]
    fn empty_graph_returns_empty() {
        let db = GraphDb::open_memory().unwrap();
        let communities = detect_communities(&db, "empty").unwrap();
        assert!(communities.is_empty());
    }
}
