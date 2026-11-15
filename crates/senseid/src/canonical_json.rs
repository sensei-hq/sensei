//! JSON with object keys sorted recursively, for hashing: two values that differ
//! only in key insertion order render identically, whatever serde_json's
//! `preserve_order` feature is set to by feature unification (it is on in this
//! build, through agent-client-protocol). Arrays keep their order; a caller that
//! needs an order-free array sorts it first.

/// Serialize `v` with object keys sorted recursively (arrays keep order).
pub fn canonical_json(v: &serde_json::Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out);
    out
}

fn write_canonical(v: &serde_json::Value, out: &mut String) {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_canonical(&map[*k], out);
            }
            out.push('}');
        }
        Value::Array(arr) => {
            out.push('[');
            for (i, e) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(e, out);
            }
            out.push(']');
        }
        // Scalars: serde_json's own rendering is already canonical.
        other => out.push_str(&serde_json::to_string(other).unwrap_or_default()),
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_json;

    #[test]
    fn key_order_does_not_matter_and_arrays_keep_theirs() {
        let a: serde_json::Value =
            serde_json::from_str(r#"{"b":1,"a":{"y":[2,1],"x":null}}"#).unwrap();
        let mut b = serde_json::Map::new();
        b.insert("a".into(), serde_json::json!({"x": null, "y": [2, 1]}));
        b.insert("b".into(), serde_json::json!(1));
        assert_eq!(canonical_json(&a), canonical_json(&serde_json::Value::Object(b)));
        assert_eq!(canonical_json(&a), r#"{"a":{"x":null,"y":[2,1]},"b":1}"#);
    }
}
