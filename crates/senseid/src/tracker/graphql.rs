//! GraphQL as a guarded read (spec tracker-github §H2): a query document that a
//! validator has proved is exactly one named `query`, and the mapping of a
//! GraphQL response envelope onto data or a [`TrackerError`].
//!
//! The document is a `&'static str` constant, so nothing a user typed is ever
//! interpolated into it: owner, number, cursor and field name travel only as
//! variables.

use crate::tracker::TrackerError;

/// A GraphQL document proved to hold exactly one named `query` operation, with no
/// `mutation`, `subscription`, anonymous `{ }` shorthand or second operation.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphqlQuery {
    document: &'static str,
    operation: &'static str,
    variables: serde_json::Value,
    /// Error `path`s whose `NOT_FOUND` means "absent", not a failure (§H2.3).
    absent_on_not_found: &'static [&'static [&'static str]],
}

impl GraphqlQuery {
    /// Validate `document`; `BadRequest` for anything but one named `query`.
    pub fn new(document: &'static str, variables: serde_json::Value) -> Result<Self, TrackerError> {
        let operation = single_query_name(document).ok_or(TrackerError::BadRequest)?;
        Ok(GraphqlQuery { document, operation, variables, absent_on_not_found: &[] })
    }

    /// A `NOT_FOUND` error at exactly one of `paths` reads as absence: the data
    /// keeps GitHub's `null` there and no error is raised (§H2.3, `Q_ISSUE` only).
    pub fn absent_on_not_found(mut self, paths: &'static [&'static [&'static str]]) -> Self {
        self.absent_on_not_found = paths;
        self
    }

    /// The request body: `{"query", "operationName", "variables"}`.
    pub fn body(&self) -> serde_json::Value {
        serde_json::json!({
            "query": self.document,
            "operationName": self.operation,
            "variables": self.variables,
        })
    }

    /// The operation's name, sent as `operationName`.
    #[cfg(test)]
    pub fn operation(&self) -> &'static str {
        self.operation
    }

    /// The variables, for a recorded transport to look its answer up by.
    #[cfg(test)]
    pub fn variables(&self) -> &serde_json::Value {
        &self.variables
    }

    /// The response envelope's `data`, or the error it carries (§H2.3).
    pub fn read(&self, envelope: serde_json::Value) -> Result<serde_json::Value, TrackerError> {
        read_envelope(envelope, self.absent_on_not_found)
    }
}

/// One lexical token of a GraphQL document, as far as the validator needs.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token<'a> {
    Name(&'a str),
    Open,
    Close,
    Other,
}

/// The document's tokens, skipping whitespace, commas, `#` comments, strings and
/// block strings. `None` for an unterminated string. Works on bytes, so a
/// non-ASCII byte is simply `Other`.
fn tokens(doc: &str) -> Option<Vec<Token<'_>>> {
    let b = doc.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\n' | b'\r' | b',' => i += 1,
            b'#' => {
                while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                    i += 1;
                }
            }
            b'"' if b[i..].starts_with(b"\"\"\"") => {
                i += 3;
                loop {
                    if i >= b.len() {
                        return None;
                    }
                    if b[i..].starts_with(b"\\\"\"\"") {
                        i += 4;
                    } else if b[i..].starts_with(b"\"\"\"") {
                        i += 3;
                        break;
                    } else {
                        i += 1;
                    }
                }
                out.push(Token::Other);
            }
            b'"' => {
                i += 1;
                loop {
                    match b.get(i) {
                        None | Some(b'\n') | Some(b'\r') => return None,
                        Some(b'\\') => i += 2,
                        Some(b'"') => {
                            i += 1;
                            break;
                        }
                        Some(_) => i += 1,
                    }
                }
                out.push(Token::Other);
            }
            b'{' => {
                out.push(Token::Open);
                i += 1;
            }
            b'}' => {
                out.push(Token::Close);
                i += 1;
            }
            c if c == b'_' || c.is_ascii_alphabetic() => {
                let start = i;
                while i < b.len() && (b[i] == b'_' || b[i].is_ascii_alphanumeric()) {
                    i += 1;
                }
                out.push(Token::Name(&doc[start..i]));
            }
            _ => {
                out.push(Token::Other);
                i += 1;
            }
        }
    }
    Some(out)
}

/// The name of the document's only operation when it is exactly one named `query`
/// (fragments allowed); `None` otherwise. Each top-level definition is a keyword,
/// a header up to its `{`, and a balanced body.
fn single_query_name(doc: &'static str) -> Option<&'static str> {
    let mut toks = tokens(doc)?.into_iter();
    let mut name = None;
    while let Some(first) = toks.next() {
        match first {
            Token::Name("query") => {
                if name.is_some() {
                    return None;
                }
                match toks.next()? {
                    Token::Name(n) => name = Some(n),
                    _ => return None,
                }
            }
            Token::Name("fragment") => {}
            _ => return None,
        }
        loop {
            match toks.next()? {
                Token::Open => break,
                Token::Close => return None,
                _ => {}
            }
        }
        let mut depth = 1usize;
        while depth > 0 {
            match toks.next()? {
                Token::Open => depth += 1,
                Token::Close => depth -= 1,
                _ => {}
            }
        }
    }
    name
}

/// `data` from a response envelope. Any `errors` entry is a failure, even beside
/// partial `data`, except when every error is a `NOT_FOUND` at one of `absent`.
/// The first error's `type` decides which failure.
pub fn read_envelope(
    envelope: serde_json::Value,
    absent: &[&[&str]],
) -> Result<serde_json::Value, TrackerError> {
    use serde_json::Value;
    let errors: &[Value] = match envelope.get("errors") {
        None | Some(Value::Null) => &[],
        Some(Value::Array(a)) => a,
        Some(_) => return Err(TrackerError::Decode),
    };
    let data = || match envelope.get("data") {
        Some(d) if !d.is_null() => Ok(d.clone()),
        _ => Err(TrackerError::Decode),
    };
    let Some(first) = errors.first() else {
        return data();
    };
    let at_absent_path = |e: &Value| {
        e["type"] == "NOT_FOUND"
            && e["path"].as_array().is_some_and(|path| {
                absent.iter().any(|p| {
                    path.len() == p.len()
                        && path.iter().zip(p.iter()).all(|(x, y)| x.as_str() == Some(*y))
                })
            })
    };
    if !absent.is_empty() && errors.iter().all(at_absent_path) {
        return data();
    }
    Err(match first["type"].as_str() {
        Some("NOT_FOUND") => TrackerError::NotFound,
        Some("FORBIDDEN" | "INSUFFICIENT_SCOPES") => TrackerError::Forbidden,
        Some("RATE_LIMITED") => TrackerError::RateLimited,
        _ => TrackerError::HttpError,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_one_named_query_is_a_query() {
        for refused in [
            "mutation M { x }",
            "{ x }",
            "query A { x } mutation B { y }",
            "# c\nmutation M { x }",
            "query A { x } query B { y }",
            "subscription S { x }",
            "query { x }",
            "query A { x",
            "query A { x(s: \"unterminated) }",
            "",
        ] {
            assert_eq!(
                GraphqlQuery::new(refused, json!({})),
                Err(TrackerError::BadRequest),
                "{refused:?}"
            );
        }
        for (accepted, name) in [
            ("query A { x(s: \"mutation\") }", "A"),
            ("query A { x } fragment F on T { y }", "A"),
            ("fragment F on T { y }\nquery Board($id: ID!) { node(id: $id) { ...F } }", "Board"),
            ("# mutation in a comment\nquery A { x }", "A"),
            ("query A { x(s: \"\"\"block } mutation { \"\"\") }", "A"),
            ("query A($a: [String!]!, $b: Int = 3) @dir { x { y { z } } }", "A"),
        ] {
            let q = GraphqlQuery::new(accepted, json!({})).expect(accepted);
            assert_eq!(q.operation(), name);
        }
    }

    #[test]
    fn the_body_names_the_operation_and_carries_the_variables() {
        let q = GraphqlQuery::new(
            "query Items($id: ID!) { node(id: $id) { id } }",
            json!({"id": "PVT_1"}),
        )
        .unwrap();
        assert_eq!(
            q.body(),
            json!({
                "query": "query Items($id: ID!) { node(id: $id) { id } }",
                "operationName": "Items",
                "variables": {"id": "PVT_1"},
            })
        );
    }

    #[test]
    fn graphql_errors_are_failures_even_beside_partial_data() {
        let err = |t: &str| json!({"data": {"a": 1}, "errors": [{"type": t, "path": ["a"]}]});
        assert_eq!(read_envelope(err("NOT_FOUND"), &[]), Err(TrackerError::NotFound));
        assert_eq!(read_envelope(err("FORBIDDEN"), &[]), Err(TrackerError::Forbidden));
        assert_eq!(read_envelope(err("INSUFFICIENT_SCOPES"), &[]), Err(TrackerError::Forbidden));
        assert_eq!(read_envelope(err("RATE_LIMITED"), &[]), Err(TrackerError::RateLimited));
        assert_eq!(read_envelope(err("SOMETHING_ELSE"), &[]), Err(TrackerError::HttpError));
        assert_eq!(
            read_envelope(json!({"errors": [{"message": "x"}]}), &[]),
            Err(TrackerError::HttpError)
        );
        assert_eq!(read_envelope(json!({}), &[]), Err(TrackerError::Decode));
        assert_eq!(
            read_envelope(json!({"data": {"a": 1}, "errors": []}), &[]),
            Ok(json!({"a": 1}))
        );
        assert_eq!(read_envelope(json!({"data": {"a": 1}}), &[]), Ok(json!({"a": 1})));
    }

    #[test]
    fn not_found_at_the_issue_paths_is_absence_and_elsewhere_an_error() {
        const ISSUE: &[&[&str]] = &[&["repository"], &["repository", "issue"]];
        let at = |path: serde_json::Value| json!({"data": {"repository": null}, "errors": [{"type": "NOT_FOUND", "path": path}]});
        assert_eq!(
            read_envelope(at(json!(["repository"])), ISSUE),
            Ok(json!({"repository": null}))
        );
        assert_eq!(
            read_envelope(at(json!(["repository", "issue"])), ISSUE),
            Ok(json!({"repository": null}))
        );
        assert_eq!(
            read_envelope(at(json!(["repository", "issue", "projectItems"])), ISSUE),
            Err(TrackerError::NotFound)
        );
        assert_eq!(read_envelope(at(json!(["node"])), ISSUE), Err(TrackerError::NotFound));
        // Every error must qualify: one other error beside it is a failure.
        let mixed = json!({"data": null, "errors": [
            {"type": "NOT_FOUND", "path": ["repository"]},
            {"type": "FORBIDDEN", "path": ["repository"]},
        ]});
        assert_eq!(read_envelope(mixed, ISSUE), Err(TrackerError::NotFound));
        // Without the exception the same NOT_FOUND is an error.
        assert_eq!(read_envelope(at(json!(["repository"])), &[]), Err(TrackerError::NotFound));
    }
}
