//! A decision's response schema (#389): the closed subset a lane may ask with,
//! and the check an answer's values must pass before the daemon stores them.
//!
//! `response = {schema, layout?}`. `schema` is a flat JSON Schema subset that
//! `@rokkit/forms` draws as is; `layout` is an optional rokkit vertical layout.
//! Both are refused, by name, outside the subset (design D1). Values are checked
//! against the stored schema on every answer, whatever the client checked (D3).

use serde_json::{Map, Value};

/// At most this many properties in one form.
pub const MAX_PROPERTIES: usize = 12;
/// At most this many members in one `enum`.
pub const MAX_ENUM: usize = 50;
/// The text cap: a `maxLength` may not exceed it, and text without one is held to it.
pub const MAX_TEXT_CHARS: usize = crate::relay::MAX_SUMMARY_CHARS;
const MAX_LABEL_CHARS: usize = 120;
const MAX_DESCRIPTION_CHARS: usize = 300;
const MAX_PATTERN_CHARS: usize = 200;

/// The keywords a property may carry; which type allows which is checked after.
const PROPERTY_KEYS: &[&str] = &[
    "type",
    "enum",
    "maxLength",
    "pattern",
    "minimum",
    "maximum",
    "title",
    "description",
    "default",
];

/// At ask time: `response` is inside the subset, or the first keyword outside it.
pub fn validate_response(response: &Value) -> Result<(), String> {
    let obj = response
        .as_object()
        .ok_or_else(|| "response must be an object {schema, layout?}".to_string())?;
    for key in obj.keys() {
        if !matches!(key.as_str(), "schema" | "layout") {
            return Err(format!("response.{key} is not supported"));
        }
    }
    let schema = obj.get("schema").filter(|v| !v.is_null()).ok_or("response.schema is required")?;
    let properties = check_schema(schema)?;
    if let Some(layout) = obj.get("layout").filter(|v| !v.is_null()) {
        check_layout(layout, properties)?;
    }
    Ok(())
}

/// At answer time: `values` satisfy the stored `response.schema`, or the first
/// property and rule that fails. An absent optional property stays absent: a
/// `default` seeds the form only, and is never filled in here (D83).
pub fn validate_values(response: &Value, values: &Value) -> Result<(), String> {
    let values = values
        .as_object()
        .ok_or_else(|| "values must be an object keyed by property name".to_string())?;
    let schema = &response["schema"];
    let properties = schema["properties"]
        .as_object()
        .ok_or_else(|| "the decision's response has no schema properties".to_string())?;
    for key in values.keys() {
        if !properties.contains_key(key) {
            return Err(format!("values.{key} is not a property of the form"));
        }
    }
    for name in schema["required"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        match values.get(name) {
            None | Some(Value::Null) => return Err(format!("values.{name} is required")),
            Some(Value::String(s)) if s.is_empty() => {
                return Err(format!("values.{name} is required"));
            }
            _ => {}
        }
    }
    for (name, value) in values {
        if value.is_null() {
            continue;
        }
        check_value(&format!("values.{name}"), &properties[name], value)?;
    }
    Ok(())
}

/// The schema object: `type: "object"`, 1-12 properties, `required` naming them.
fn check_schema(schema: &Value) -> Result<&Map<String, Value>, String> {
    let obj = schema.as_object().ok_or("response.schema must be an object")?;
    for key in obj.keys() {
        if !matches!(key.as_str(), "type" | "properties" | "required") {
            return Err(format!("response.schema.{key} is not supported"));
        }
    }
    if obj.get("type").and_then(Value::as_str) != Some("object") {
        return Err("response.schema.type must be object".into());
    }
    let properties = obj
        .get("properties")
        .and_then(Value::as_object)
        .filter(|p| (1..=MAX_PROPERTIES).contains(&p.len()))
        .ok_or_else(|| {
            format!("response.schema.properties needs 1 to {MAX_PROPERTIES} properties")
        })?;
    for (name, prop) in properties {
        if !is_property_name(name) {
            return Err(format!(
                "response.schema.properties.{name}: a name must match ^[a-z][a-z0-9_]{{0,39}}$"
            ));
        }
        check_property(name, prop)?;
    }
    if let Some(required) = obj.get("required").filter(|v| !v.is_null()) {
        let required = required.as_array().ok_or("response.schema.required must be an array")?;
        let mut seen: Vec<&str> = Vec::with_capacity(required.len());
        for name in required {
            let name = name.as_str().ok_or("response.schema.required holds property names")?;
            if !properties.contains_key(name) {
                return Err(format!("response.schema.required names no property: {name}"));
            }
            if seen.contains(&name) {
                return Err(format!("response.schema.required names {name} twice"));
            }
            seen.push(name);
        }
    }
    Ok(properties)
}

fn is_property_name(name: &str) -> bool {
    let mut chars = name.chars();
    name.len() <= 40
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// One property: its type, then only the keywords that type allows.
fn check_property(name: &str, prop: &Value) -> Result<(), String> {
    let at = format!("response.schema.properties.{name}");
    let obj = prop.as_object().ok_or_else(|| format!("{at} must be an object"))?;
    let ty = obj.get("type").and_then(Value::as_str);
    if let Some(ty) = ty
        && !matches!(ty, "string" | "number" | "integer" | "boolean")
    {
        return Err(format!("{at}.type: {ty} is not supported"));
    }
    for key in obj.keys() {
        if !PROPERTY_KEYS.contains(&key.as_str()) {
            return Err(format!("{at}.{key} is not supported"));
        }
    }
    let ty = ty.ok_or_else(|| format!("{at}.type must be string, number, integer or boolean"))?;
    let only_on = |key: &str, types: &[&str]| -> Result<(), String> {
        if obj.contains_key(key) && !types.contains(&ty) {
            return Err(format!("{at}.{key} is not allowed on a {ty}"));
        }
        Ok(())
    };
    only_on("enum", &["string"])?;
    only_on("maxLength", &["string"])?;
    only_on("pattern", &["string"])?;
    only_on("minimum", &["number", "integer"])?;
    only_on("maximum", &["number", "integer"])?;
    if let Some(members) = obj.get("enum") {
        let members = members
            .as_array()
            .filter(|m| (1..=MAX_ENUM).contains(&m.len()))
            .ok_or_else(|| format!("{at}.enum needs 1 to {MAX_ENUM} members"))?;
        let mut seen: Vec<&str> = Vec::with_capacity(members.len());
        for member in members {
            let member = member
                .as_str()
                .filter(|m| m.chars().count() <= MAX_LABEL_CHARS)
                .ok_or_else(|| {
                    format!("{at}.enum members are strings of at most {MAX_LABEL_CHARS} characters")
                })?;
            if seen.contains(&member) {
                return Err(format!("{at}.enum lists {member} twice"));
            }
            seen.push(member);
        }
    }
    if let Some(max) = obj.get("maxLength") {
        let ok = max.as_u64().is_some_and(|n| (1..=MAX_TEXT_CHARS as u64).contains(&n));
        if !ok {
            return Err(format!(
                "{at}.maxLength must be a whole number from 1 to {MAX_TEXT_CHARS}"
            ));
        }
    }
    if let Some(pattern) = obj.get("pattern") {
        let pattern = pattern
            .as_str()
            .filter(|p| p.chars().count() <= MAX_PATTERN_CHARS)
            .ok_or_else(|| {
                format!("{at}.pattern must be a string of at most {MAX_PATTERN_CHARS} characters")
            })?;
        // Inline flags and named groups are spelled differently in the browser's RegExp.
        if pattern.contains("(?") {
            return Err(format!("{at}.pattern may not use (? groups or flags"));
        }
        regex::Regex::new(pattern).map_err(|_| format!("{at}.pattern does not compile"))?;
    }
    let bound = |key: &str| -> Result<Option<f64>, String> {
        match obj.get(key) {
            None => Ok(None),
            Some(v) => v
                .as_f64()
                .filter(|n| n.is_finite())
                .map(Some)
                .ok_or_else(|| format!("{at}.{key} must be a finite number")),
        }
    };
    if let (Some(min), Some(max)) = (bound("minimum")?, bound("maximum")?) {
        if min > max {
            return Err(format!("{at}.minimum must not exceed maximum"));
        }
        if ty == "integer" && min.ceil() > max.floor() {
            return Err(format!("{at}: no whole number lies between minimum and maximum"));
        }
    }
    // Each member must pass the property's own rules, or picking it is refused at answer time.
    for (i, member) in obj.get("enum").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let text = member.as_str().unwrap_or_default();
        if text.is_empty() {
            return Err(format!(
                "{at}.enum member {i} is empty, and an empty answer reads as none"
            ));
        }
        crate::relay::guard_markdown(text).map_err(|kinds| {
            format!("{at}.enum member {i} carries a secret ({})", kinds.join(", "))
        })?;
        check_value(&format!("{at}.enum member {text}"), prop, member)?;
    }
    for (key, cap) in [("title", MAX_LABEL_CHARS), ("description", MAX_DESCRIPTION_CHARS)] {
        if let Some(v) = obj.get(key) {
            check_text(&format!("{at}.{key}"), v, cap)?;
        }
    }
    if let Some(default) = obj.get("default") {
        check_value(&format!("{at}.default"), prop, default)?;
    }
    Ok(())
}

/// A short annotation: a string of at most `cap` characters.
fn check_text(at: &str, value: &Value, cap: usize) -> Result<(), String> {
    match value.as_str() {
        Some(s) if s.chars().count() <= cap => Ok(()),
        _ => Err(format!("{at} must be a string of at most {cap} characters")),
    }
}

/// The optional layout: a vertical list of elements naming each property once,
/// carrying only a label, help and a placeholder (nothing that picks a control).
fn check_layout(layout: &Value, properties: &Map<String, Value>) -> Result<(), String> {
    let obj = layout.as_object().ok_or("response.layout must be an object")?;
    for key in obj.keys() {
        if !matches!(key.as_str(), "type" | "elements") {
            return Err(format!("response.layout.{key} is not supported"));
        }
    }
    if obj.get("type").and_then(Value::as_str) != Some("vertical") {
        return Err("response.layout.type must be vertical".into());
    }
    let elements = obj
        .get("elements")
        .and_then(Value::as_array)
        .ok_or("response.layout.elements must be an array")?;
    let mut seen: Vec<&str> = Vec::with_capacity(elements.len());
    for (i, element) in elements.iter().enumerate() {
        let at = format!("response.layout.elements[{i}]");
        let el = element.as_object().ok_or_else(|| format!("{at} must be an object"))?;
        for key in el.keys() {
            if !matches!(key.as_str(), "scope" | "label" | "description" | "props") {
                return Err(format!("{at}.{key} is not supported"));
            }
        }
        let scope = el.get("scope").and_then(Value::as_str).unwrap_or_default();
        let name = scope.strip_prefix("#/").filter(|n| properties.contains_key(*n));
        let Some(name) = name else {
            return Err(format!("{at}.scope names no property: {scope}"));
        };
        if seen.contains(&name) {
            return Err(format!("{at}.scope names {name} twice"));
        }
        seen.push(name);
        if let Some(v) = el.get("label") {
            check_text(&format!("{at}.label"), v, MAX_LABEL_CHARS)?;
        }
        if let Some(v) = el.get("description") {
            check_text(&format!("{at}.description"), v, MAX_DESCRIPTION_CHARS)?;
        }
        if let Some(props) = el.get("props") {
            let props = props.as_object().ok_or_else(|| format!("{at}.props must be an object"))?;
            for (key, v) in props {
                if key != "placeholder" {
                    return Err(format!("{at}.props.{key} is not supported"));
                }
                check_text(&format!("{at}.props.placeholder"), v, MAX_LABEL_CHARS)?;
            }
        }
    }
    // rokkit draws only the layout's elements, so a property left out gets no control.
    if let Some(name) = properties.keys().find(|n| !seen.contains(&n.as_str())) {
        return Err(format!(
            "response.layout.elements must name every property; {name} is missing"
        ));
    }
    Ok(())
}

/// One value against its property's rules; `at` names it in the refusal.
fn check_value(at: &str, prop: &Value, value: &Value) -> Result<(), String> {
    match prop["type"].as_str() {
        Some("string") => {
            let text = value.as_str().ok_or_else(|| format!("{at} must be a string"))?;
            if let Some(members) = prop["enum"].as_array()
                && !members.iter().any(|m| m.as_str() == Some(text))
            {
                return Err(format!("{at} must be one of its enum members"));
            }
            let chars = text.chars().count();
            if let Some(max) = prop["maxLength"].as_u64()
                && chars as u64 > max
            {
                return Err(format!("{at} is over its maxLength {max}"));
            }
            if chars > MAX_TEXT_CHARS {
                return Err(format!("{at} is over {MAX_TEXT_CHARS} characters"));
            }
            if let Some(pattern) = prop["pattern"].as_str() {
                let re = regex::Regex::new(pattern)
                    .map_err(|_| format!("{at}: its pattern does not compile"))?;
                if !re.is_match(text) {
                    return Err(format!("{at} does not match its pattern"));
                }
            }
            crate::relay::guard_markdown(text)
                .map_err(|kinds| format!("{at} carries a secret ({})", kinds.join(", ")))?;
        }
        Some(ty @ ("number" | "integer")) => {
            let n = value
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or_else(|| format!("{at} must be a number"))?;
            if ty == "integer" && n.fract() != 0.0 {
                return Err(format!("{at} must be a whole number"));
            }
            if let Some(min) = prop["minimum"].as_f64()
                && n < min
            {
                return Err(format!("{at} is below its minimum {min}"));
            }
            if let Some(max) = prop["maximum"].as_f64()
                && n > max
            {
                return Err(format!("{at} is above its maximum {max}"));
            }
        }
        Some("boolean") => {
            if !value.is_boolean() {
                return Err(format!("{at} must be true or false"));
            }
        }
        _ => return Err(format!("{at}: the schema gives it no supported type")),
    }
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

    #[test]
    fn refuses_a_layout_that_leaves_a_property_undrawn() {
        // rokkit draws only the layout's elements: a property it omits gets no control.
        let lay = |elements: Value| json!({"schema": schema(), "layout": {"type": "vertical", "elements": elements}});
        refused(&lay(json!([])), &["layout", "target"]);
        // The required property is missing.
        refused(&lay(json!([{"scope": "#/count"}, {"scope": "#/ship"}])), &["layout", "target"]);
        // An optional property is missing: still one control per property.
        refused(&lay(json!([{"scope": "#/target"}, {"scope": "#/count"}])), &["layout", "ship"]);
    }

    #[test]
    fn refuses_a_property_no_answer_can_satisfy() {
        // An enum member off its own pattern or over its own maxLength.
        refused(
            &with_prop("b", json!({"type": "string", "enum": ["main"], "pattern": "^x"})),
            &["b", "enum", "main"],
        );
        refused(
            &with_prop("b", json!({"type": "string", "enum": ["develop"], "maxLength": 3})),
            &["b", "enum", "develop"],
        );
        // An enum member that trips the leak guard, named by kind, never echoed.
        let secret = format!("ghp_{}", "b".repeat(38));
        let err = validate_response(&with_prop("b", json!({"type": "string", "enum": [secret]})))
            .expect_err("a secret member is refused");
        assert!(err.contains("properties.b.enum"), "{err}");
        assert!(!err.contains(&secret), "the member never echoes: {err}");
        // An empty member reads as missing, so a person could never pick it.
        refused(&with_prop("b", json!({"type": "string", "enum": ["", "x"]})), &["b", "enum"]);
        // Integer bounds with no whole number between them.
        refused(
            &with_prop("n", json!({"type": "integer", "minimum": 0.2, "maximum": 0.8})),
            &["n", "whole number"],
        );
        // Fractional bounds that still hold a whole number stay accepted.
        let r = with_prop("n", json!({"type": "integer", "minimum": 0.5, "maximum": 1.5}));
        assert_eq!(validate_response(&r), Ok(()));
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
