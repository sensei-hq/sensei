//! Embed phase: generate vector embeddings for code-graph nodes so semantic
//! search and duplicate detection can rank by cosine similarity. Runs as a
//! barrier task after a folder's files are processed.

use super::super::executor::TaskContext;
use super::super::Task;

/// Texts embedded per gateway call.
const EMBED_BATCH: usize = 64;
/// Expected embedding width — matches the `vector(384)` column on sensei.nodes.
const EMBED_DIM: usize = 384;

/// Build the text fed to the embedding model for a node. Combines kind, name,
/// signature, and file location so semantically related symbols land near each
/// other in vector space.
fn embed_text(kind: &str, name: &str, signature: Option<&str>, file_path: &str) -> String {
    let mut s = String::with_capacity(name.len() + 32);
    s.push_str(kind);
    s.push(' ');
    s.push_str(name);
    if let Some(sig) = signature.map(str::trim).filter(|s| !s.is_empty()) {
        s.push_str(" — ");
        s.push_str(sig);
    }
    if !file_path.is_empty() {
        s.push_str(" [");
        s.push_str(file_path);
        s.push(']');
    }
    // Cap length so an unusually long signature/name can't exceed the embedding
    // model's context window (all-minilm ~256 tokens). Truncate on a char
    // boundary; the head (kind + name + signature start) carries the signal.
    const MAX: usize = 800;
    if s.len() > MAX {
        let mut end = MAX;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    s
}

/// Embed every not-yet-embedded code-graph node for the task's folder.
/// `folder_path` is the repo abs_path (Task contract).
pub async fn embed_nodes(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Payload};

    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let Some(folder_id) = folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]))
    else {
        return Ok(0); // folder removed before this barrier ran — nothing to embed
    };

    // (id, kind, name, signature, file_path) for each node still missing an embedding.
    let nodes = ctx.pg().nodes_without_embeddings(&folder_id, 10_000).await?;
    if nodes.is_empty() {
        return Ok(0);
    }

    let mut embedded = 0u32;
    for chunk in nodes.chunks(EMBED_BATCH) {
        let texts: Vec<String> = chunk
            .iter()
            .map(|(_, kind, name, sig, fp)| embed_text(kind, name, sig.as_deref(), fp))
            .collect();

        let request = InferenceRequest {
            capability: Capability::TextEmbed,
            model: None,
            router: None,
            // Pin the 384-dim `embed` chain so node embeddings always match the
            // vector(384) column, regardless of other TextEmbed models present.
            chain: Some("embed".to_string()),
            payload: Payload::Embed { texts },
            budget: None,
        };

        let response = ctx
            .app_state
            .gateway
            .execute(&request)
            .await
            .map_err(|e| format!("gateway embed failed: {e}"))?;
        let vectors = response
            .embeddings
            .ok_or_else(|| "gateway returned no embeddings".to_string())?;
        if vectors.len() != chunk.len() {
            return Err(format!(
                "embedding count mismatch: {} texts, {} vectors",
                chunk.len(),
                vectors.len()
            ));
        }

        for ((id, _, _, _, _), vector) in chunk.iter().zip(vectors.iter()) {
            if vector.len() != EMBED_DIM {
                return Err(format!(
                    "expected {EMBED_DIM}-dim embeddings (sensei.nodes.embedding is vector({EMBED_DIM})), \
                     got {} — check the embedding model bound to the inference role",
                    vector.len()
                ));
            }
            ctx.pg().set_node_embedding(id, vector).await?;
            embedded += 1;
        }
    }

    tracing::info!("embed_nodes: {} — embedded {} nodes", task.folder_name(), embedded);
    Ok(embedded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_text_combines_kind_name_signature_and_path() {
        assert_eq!(
            embed_text("function", "get_user", Some("(id: Uuid) -> User"), "src/api.rs"),
            "function get_user — (id: Uuid) -> User [src/api.rs]"
        );
    }

    #[test]
    fn embed_text_omits_blank_signature_and_path() {
        assert_eq!(embed_text("class", "UserStore", None, ""), "class UserStore");
        assert_eq!(embed_text("const", "MAX", Some("   "), "lib.rs"), "const MAX [lib.rs]");
    }

    #[test]
    fn embed_text_caps_length() {
        let huge = "x".repeat(5000);
        let t = embed_text("function", "f", Some(&huge), "a.rs");
        assert!(t.len() <= 800, "embed text should be capped, got {}", t.len());
    }
}
