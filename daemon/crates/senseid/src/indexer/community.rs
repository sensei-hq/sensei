use std::collections::HashMap;

/// Run Leiden-inspired community detection.
/// TODO: implement using nodes/edges queries.
#[allow(dead_code)]
pub fn detect_communities(_project: &str) -> Result<HashMap<String, u32>, String> {
    Ok(HashMap::new())
}

#[allow(dead_code)] // TODO: wire up community detection UI
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommunityInfo {
    pub id: u32,
    pub size: u32,
    pub sample_members: Vec<String>,
}
