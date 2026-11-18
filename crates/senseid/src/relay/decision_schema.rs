//! A decision's response schema (#389): the closed subset a lane may ask with,
//! and the check an answer's values must pass before the daemon stores them.
//!
//! `response = {schema, layout?}`. `schema` is a flat JSON Schema subset that
//! `@rokkit/forms` draws as is; `layout` is an optional rokkit vertical layout.
//! Both are refused, by name, outside the subset (design D1). Values are checked
//! against the stored schema on every answer, whatever the client checked (D3).

use serde_json::Value;

/// At ask time: `response` is inside the subset, or the first keyword outside it.
pub fn validate_response(_response: &Value) -> Result<(), String> {
    Ok(())
}

/// At answer time: `values` satisfy the stored `response.schema`, or the first
/// property and rule that fails.
pub fn validate_values(_response: &Value, _values: &Value) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "target": {"type": "string", "enum": ["develop", "main"], "title": "Target"},
                "count": {"type": "integer", "minimum": 1, "maximum": 5, "default": 2},
                "ship": {"type": "boolean", "description": "Ship it now?"}
            },
            "required": ["target"]
        })
    }

    fn response() -> Value {
        json!({"schema": schema()})
    }

    fn with_prop(name: &str, prop: Value) -> Value {
        let mut s = schema();
        s["properties"][name] = prop;
        json!({"schema": s})
    }

    fn refused(response: &Value, needles: &[&str]) {
        let err = validate_response(response).expect_err(&format!("accepted {response}"));
        for n in needles {
            assert!(err.contains(n), "`{err}` should name `{n}`");
        }
    }

    // ── validate_response ──

    #[test]
    fn accepts_a_three_property_schema_with_and_without_a_layout() {
        assert_eq!(validate_response(&response()), Ok(()));
        let layout = json!({"type": "vertical", "elements": [
            {"scope": "#/target", "label": "Where", "description": "The branch"},
            {"scope": "#/count", "props": {"placeholder": "1-5"}},
            {"scope": "#/ship"}
        ]});
        assert_eq!(validate_response(&json!({"schema": schema(), "layout": layout})), Ok(()));
        // String constraints and annotations.
        let r = with_prop("path", json!({"type": "string", "maxLength": 200, "pattern": "^src/"}));
        assert_eq!(validate_response(&r), Ok(()));
        let r = with_prop("ratio", json!({"type": "number", "minimum": 0.5, "maximum": 0.5}));
        assert_eq!(validate_response(&r), Ok(()));
    }

    #[test]
    fn refuses_a_response_that_is_not_the_shape() {
        refused(&json!("form"), &["response"]);
        refused(&json!({}), &["schema"]);
        refused(&json!({"schema": schema(), "extra": 1}), &["extra"]);
        let mut s = schema();
        s["type"] = json!("array");
        refused(&json!({"schema": s}), &["type"]);
        let mut s = schema();
        s["additionalProperties"] = json!(false);
        refused(&json!({"schema": s}), &["additionalProperties"]);
    }

    #[test]
    fn refuses_property_counts_and_names() {
        refused(&json!({"schema": {"type": "object", "properties": {}}}), &["properties"]);
        let many: serde_json::Map<String, Value> =
            (0..13).map(|i| (format!("p{i}"), json!({"type": "boolean"}))).collect();
        refused(&json!({"schema": {"type": "object", "properties": many}}), &["12"]);
        refused(&with_prop("Bad-Name", json!({"type": "boolean"})), &["Bad-Name"]);
        refused(&with_prop(&"a".repeat(41), json!({"type": "boolean"})), &["name"]);
    }

    #[test]
    fn refuses_required_naming_an_unknown_or_repeated_property() {
        let mut s = schema();
        s["required"] = json!(["nope"]);
        refused(&json!({"schema": s}), &["required", "nope"]);
        let mut s = schema();
        s["required"] = json!(["target", "target"]);
        refused(&json!({"schema": s}), &["required", "target"]);
    }

    #[test]
    fn refuses_pick_several_nested_objects_and_composition() {
        refused(
            &with_prop("files", json!({"type": "array", "items": {"enum": ["a", "b"]}})),
            &["properties.files.type", "array"],
        );
        refused(&with_prop("files", json!({"type": "string", "items": {}})), &["files", "items"]);
        refused(&with_prop("note", json!({"type": "string", "format": "textarea"})), &["format"]);
        refused(&with_prop("sub", json!({"type": "object", "properties": {}})), &["sub", "object"]);
        refused(&with_prop("x", json!({"$ref": "#/defs/x"})), &["$ref"]);
        refused(&with_prop("x", json!({"type": "string", "oneOf": []})), &["oneOf"]);
        refused(&with_prop("x", json!({"type": "string", "minLength": 1})), &["minLength"]);
        refused(
            &with_prop("x", json!({"type": "number", "exclusiveMinimum": 1})),
            &["exclusiveMinimum"],
        );
    }

    #[test]
    fn refuses_bad_enums() {
        refused(&with_prop("n", json!({"type": "integer", "enum": [1, 2]})), &["n", "enum"]);
        refused(&with_prop("s", json!({"type": "string", "enum": ["a", "a"]})), &["s", "enum"]);
        let many: Vec<String> = (0..51).map(|i| format!("o{i}")).collect();
        refused(&with_prop("s", json!({"type": "string", "enum": many})), &["s", "enum"]);
        refused(&with_prop("s", json!({"type": "string", "enum": []})), &["s", "enum"]);
        refused(&with_prop("s", json!({"type": "string", "enum": ["x".repeat(121)]})), &["enum"]);
    }

    #[test]
    fn refuses_bad_bounds_and_lengths() {
        refused(&with_prop("s", json!({"type": "string", "maxLength": 0})), &["s", "maxLength"]);
        refused(&with_prop("s", json!({"type": "string", "maxLength": 601})), &["s", "maxLength"]);
        refused(&with_prop("s", json!({"type": "string", "maxLength": 2.5})), &["maxLength"]);
        refused(
            &with_prop("n", json!({"type": "integer", "minimum": 5, "maximum": 1})),
            &["n", "minimum"],
        );
        refused(&with_prop("n", json!({"type": "number", "minimum": "1"})), &["n", "minimum"]);
        refused(&with_prop("b", json!({"type": "boolean", "maximum": 1})), &["b", "maximum"]);
        refused(&with_prop("n", json!({"type": "integer", "maxLength": 3})), &["n", "maxLength"]);
    }

    #[test]
    fn refuses_patterns_that_do_not_compile_or_carry_inline_groups() {
        refused(&with_prop("s", json!({"type": "string", "pattern": "("})), &["s", "pattern"]);
        refused(
            &with_prop("s", json!({"type": "string", "pattern": "(?i)abc"})),
            &["s", "pattern"],
        );
        refused(&with_prop("s", json!({"type": "string", "pattern": "(?<n>a)"})), &["pattern"]);
        refused(
            &with_prop("s", json!({"type": "string", "pattern": "a".repeat(201)})),
            &["pattern"],
        );
    }

    #[test]
    fn refuses_a_default_that_breaks_its_own_rules() {
        refused(
            &with_prop("n", json!({"type": "integer", "minimum": 1, "default": 0})),
            &["n", "default"],
        );
        refused(
            &with_prop("s", json!({"type": "string", "enum": ["a"], "default": "b"})),
            &["s", "default"],
        );
        refused(&with_prop("b", json!({"type": "boolean", "default": "yes"})), &["b", "default"]);
        refused(&with_prop("b", json!({"type": "boolean", "title": "x".repeat(121)})), &["title"]);
        refused(
            &with_prop("b", json!({"type": "boolean", "description": "x".repeat(301)})),
            &["description"],
        );
    }

    #[test]
    fn refuses_a_layout_outside_the_subset() {
        let lay = |el: Value| json!({"schema": schema(), "layout": {"type": "vertical", "elements": [el]}});
        refused(&lay(json!({"scope": "#/nope"})), &["layout", "nope"]);
        refused(
            &json!({"schema": schema(), "layout": {"type": "vertical", "elements": [
                {"scope": "#/target"}, {"scope": "#/target"}
            ]}}),
            &["layout", "target"],
        );
        for key in ["renderer", "override", "readonly", "options"] {
            refused(&lay(json!({"scope": "#/target", key: "textarea"})), &[key]);
        }
        for key in ["min", "max", "renderer"] {
            refused(&lay(json!({"scope": "#/count", "props": {key: 1}})), &[key]);
        }
        refused(&lay(json!({"scope": "#/target", "bogus": 1})), &["bogus"]);
        refused(&lay(json!({"scope": "#/target", "label": "x".repeat(121)})), &["label"]);
        refused(
            &json!({"schema": schema(), "layout": {"type": "horizontal", "elements": []}}),
            &["layout", "type"],
        );
    }

    // ── validate_values ──

    fn values_err(values: Value) -> String {
        validate_values(&response(), &values).expect_err(&format!("accepted {values}"))
    }

    #[test]
    fn accepts_values_that_satisfy_the_schema() {
        let ok = json!({"target": "main", "count": 3, "ship": true});
        assert_eq!(validate_values(&response(), &ok), Ok(()));
        // An absent optional stays absent: no default is required or filled.
        assert_eq!(validate_values(&response(), &json!({"target": "develop"})), Ok(()));
        // An integer may arrive as 3.0 (a JS number): it is still whole.
        assert_eq!(validate_values(&response(), &json!({"target": "main", "count": 3.0})), Ok(()));
    }

    #[test]
    fn refuses_unknown_keys_and_missing_required() {
        assert!(values_err(json!({"target": "main", "bogus": 1})).contains("values.bogus"));
        for v in [json!({}), json!({"target": null}), json!({"target": ""})] {
            let err = values_err(v);
            assert!(err.contains("values.target is required"), "{err}");
        }
        assert!(values_err(json!(["main"])).contains("values"));
    }

    #[test]
    fn refuses_the_wrong_type_per_type() {
        let err = values_err(json!({"target": "main", "count": 2.5}));
        assert!(err.contains("values.count") && err.contains("whole"), "{err}");
        let err = values_err(json!({"target": "main", "count": "3"}));
        assert!(err.contains("values.count"), "{err}");
        let err = values_err(json!({"target": "main", "ship": "true"}));
        assert!(err.contains("values.ship"), "{err}");
        let err = values_err(json!({"target": 7}));
        assert!(err.contains("values.target"), "{err}");
        let r = json!({"schema": {"type": "object", "properties": {"r": {"type": "number"}}}});
        let err = validate_values(&r, &json!({"r": "3"})).unwrap_err();
        assert!(err.contains("values.r"), "{err}");
        assert_eq!(validate_values(&r, &json!({"r": 2.5})), Ok(()));
    }

    #[test]
    fn refuses_enum_misses_and_bounds() {
        assert!(values_err(json!({"target": "prod"})).contains("values.target"));
        let err = values_err(json!({"target": "main", "count": 0}));
        assert!(err.contains("values.count") && err.contains("minimum"), "{err}");
        let err = values_err(json!({"target": "main", "count": 6}));
        assert!(err.contains("values.count") && err.contains("maximum"), "{err}");
    }

    #[test]
    fn refuses_text_over_its_length_or_off_its_pattern() {
        let r = json!({"schema": {"type": "object", "properties": {
            "path": {"type": "string", "maxLength": 10, "pattern": "src/"},
            "free": {"type": "string"}
        }}});
        assert_eq!(validate_values(&r, &json!({"path": "a/src/b"})), Ok(()), "unanchored");
        let err = validate_values(&r, &json!({"path": "src/".repeat(3)})).unwrap_err();
        assert!(err.contains("values.path") && err.contains("maxLength"), "{err}");
        let err = validate_values(&r, &json!({"path": "lib/x"})).unwrap_err();
        assert!(err.contains("values.path") && err.contains("pattern"), "{err}");
        // No maxLength: the 600-char cap still holds (counted in chars).
        assert_eq!(validate_values(&r, &json!({"free": "é".repeat(600)})), Ok(()));
        let err = validate_values(&r, &json!({"free": "x".repeat(601)})).unwrap_err();
        assert!(err.contains("values.free") && err.contains("600"), "{err}");
    }

    #[test]
    fn refuses_text_that_trips_the_leak_guard_naming_kinds_only() {
        let r = json!({"schema": {"type": "object", "properties": {"free": {"type": "string"}}}});
        let secret = format!("ghp_{}", "b".repeat(38));
        let err = validate_values(&r, &json!({"free": format!("token {secret}")})).unwrap_err();
        assert!(err.contains("values.free"), "{err}");
        assert!(!err.contains(&secret), "the value never echoes: {err}");
    }
}
