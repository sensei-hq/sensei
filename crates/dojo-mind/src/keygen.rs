//! `sensei-dojo keygen` — mint a member + API key directly against the DB.
use crate::store::DojoStore;

/// Create a member with `role` and issue one key; returns the plaintext key.
pub async fn generate_key(store: &DojoStore, name: &str, role: &str, label: Option<&str>) -> Result<String, String> {
    if crate::auth::Role::parse(role).is_none() {
        return Err(format!("invalid role '{role}' (member|publisher|admin)"));
    }
    let member = store.create_member(name, None, role).await?;
    let issued = store.issue_key(&member, label).await?;
    store.record_audit(Some(&member), "key.issue", Some(&issued.key_id), serde_json::json!({ "via": "keygen", "role": role })).await?;
    Ok(issued.plaintext)
}
