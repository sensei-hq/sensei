//! Shared XML helpers for `ManifestAdapter` impls whose manifest is XML —
//! Maven's `pom.xml` (#86) and .NET's `.csproj` / `.fsproj` (#88).
//!
//! Streams the document via `quick-xml::Reader` (no allocation of a full
//! parse tree). Callers pull out only the tags they need. This keeps the
//! adapters robust against real-world XML quirks — comments, namespaces,
//! CDATA, self-closing tags, attributes on tags we care about — that any
//! regex approach handles incorrectly.

use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;

/// The path of open element names from document root down to the current
/// event, used by callers to distinguish e.g. `<project><groupId>` from
/// `<project><parent><groupId>`.
///
/// Names are the *local* part of the tag — namespace prefixes are stripped
/// so a pom with `xmlns="http://maven.apache.org/POM/4.0.0"` behaves the
/// same as one without.
pub struct XmlPath<'a>(pub &'a [String]);

impl XmlPath<'_> {
    /// True when the current path matches exactly `expected`, top-down.
    pub fn is(&self, expected: &[&str]) -> bool {
        self.0.len() == expected.len() && self.0.iter().zip(expected).all(|(a, b)| a == b)
    }

    /// True when the current path ends with `suffix`, top-down.
    #[allow(dead_code)]
    pub fn ends_with(&self, suffix: &[&str]) -> bool {
        self.0.len() >= suffix.len()
            && self.0[self.0.len() - suffix.len()..].iter().zip(suffix).all(|(a, b)| a == b)
    }
}

/// One tagged event a `walk` visitor receives. `Leaf` fires exactly once per
/// element that has text (or is empty/self-closing); `Enter`/`Exit` bracket
/// every element so callers can partition repeated children — e.g. keep
/// per-`<dependency>` state — without carrying the previous-path themselves.
/// `Enter` also carries the element's attributes as a `(name, value)` slice —
/// .NET `.csproj` puts dependency identity in attributes
/// (`<PackageReference Include="Foo" Version="1.0" />`) rather than text.
pub enum XmlEvent<'a> {
    Enter(&'a [(String, String)]),
    Leaf(&'a str),
    Exit,
}

/// Streaming walk of `content`. The visitor is invoked with the current
/// element-name path and one of three event variants. `Enter` fires when an
/// element opens; `Exit` fires when it closes; `Leaf` fires exactly once per
/// element that carried text — or once with `""` for a self-closing element.
///
/// Returns `Err` on malformed XML — quick-xml is fairly forgiving (a missing
/// close tag returns EOF rather than an error), so this is mostly a hedge.
pub fn walk<F>(content: &str, mut visit: F) -> Result<(), String>
where
    F: FnMut(&XmlPath<'_>, XmlEvent<'_>),
{
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf: Vec<u8> = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let mut current_text: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(format!("xml: {e}")),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                current_text = None;
                path.push(local_name(e.name()));
                let attrs = collect_attrs(&e);
                visit(&XmlPath(&path), XmlEvent::Enter(&attrs));
            }
            Ok(Event::End(_)) => {
                if let Some(text) = current_text.take() {
                    visit(&XmlPath(&path), XmlEvent::Leaf(text.trim()));
                }
                visit(&XmlPath(&path), XmlEvent::Exit);
                path.pop();
            }
            Ok(Event::Empty(e)) => {
                path.push(local_name(e.name()));
                let attrs = collect_attrs(&e);
                visit(&XmlPath(&path), XmlEvent::Enter(&attrs));
                visit(&XmlPath(&path), XmlEvent::Leaf(""));
                visit(&XmlPath(&path), XmlEvent::Exit);
                path.pop();
            }
            Ok(Event::Text(e)) => {
                if let Ok(txt) = e.unescape() {
                    let s = txt.into_owned();
                    if !s.trim().is_empty() {
                        current_text = Some(match current_text.take() {
                            Some(mut prev) => {
                                prev.push_str(&s);
                                prev
                            }
                            None => s,
                        });
                    }
                }
            }
            Ok(Event::CData(e)) => {
                if let Ok(raw) = std::str::from_utf8(e.as_ref()) {
                    let s = raw.to_string();
                    current_text = Some(match current_text.take() {
                        Some(mut prev) => {
                            prev.push_str(&s);
                            prev
                        }
                        None => s,
                    });
                }
            }
            Ok(_) => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Convenience: only surface leaves. Callers that don't need enter/exit
/// (identity extraction, `is_workspace_root` checks) can use this to
/// keep the visitor closure tight.
pub fn walk_leaves<F: FnMut(&XmlPath<'_>, &str)>(
    content: &str,
    mut visit: F,
) -> Result<(), String> {
    walk(content, |path, evt| {
        if let XmlEvent::Leaf(text) = evt {
            visit(path, text);
        }
    })
}

/// Local (unprefixed) element name — the last `:`-separated segment.
fn local_name(name: QName<'_>) -> String {
    let raw = std::str::from_utf8(name.as_ref()).unwrap_or("");
    match raw.rfind(':') {
        Some(idx) => raw[idx + 1..].to_string(),
        None => raw.to_string(),
    }
}

/// Collect `(name, value)` attribute pairs off an element start event.
/// Malformed / non-utf8 attributes are dropped rather than surfaced — this
/// path is best-effort for real-world manifests, not a validating parser.
fn collect_attrs(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
    e.attributes()
        .flatten()
        .filter_map(|a| {
            let name = local_name(a.key);
            let value = a.unescape_value().ok()?.into_owned();
            Some((name, value))
        })
        .collect()
}

/// Look up an attribute value by (local) name from the `Enter` payload.
/// Returns `None` when the attribute is absent.
pub fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_leaves_reports_leaf_text_with_full_path() {
        let mut seen: Vec<(Vec<String>, String)> = Vec::new();
        walk_leaves(
            r#"<root>
                <a>1</a>
                <b><c>2</c></b>
            </root>"#,
            |p, t| seen.push((p.0.to_vec(), t.to_string())),
        )
        .unwrap();
        assert_eq!(
            seen,
            vec![
                (vec!["root".into(), "a".into()], "1".into()),
                (vec!["root".into(), "b".into(), "c".into()], "2".into()),
            ],
        );
    }

    #[test]
    fn walk_leaves_strips_namespace_prefix() {
        // Real pom.xml carries xmlns declarations that quick-xml exposes as
        // namespace-prefixed names. Locally-namespaced lookups must still
        // match by the unprefixed tag.
        let mut names: Vec<String> = Vec::new();
        walk_leaves(
            r#"<project xmlns="http://maven.apache.org/POM/4.0.0">
                <groupId>com.example</groupId>
            </project>"#,
            |p, _| names.push(p.0.last().cloned().unwrap()),
        )
        .unwrap();
        assert_eq!(names, vec!["groupId".to_string()]);
    }

    #[test]
    fn walk_leaves_handles_cdata() {
        let mut seen: Vec<String> = Vec::new();
        walk_leaves(r#"<r><d><![CDATA[hello <world>]]></d></r>"#, |_, t| seen.push(t.to_string()))
            .unwrap();
        assert_eq!(seen, vec!["hello <world>".to_string()]);
    }

    #[test]
    fn walk_enter_exit_brackets_every_element() {
        let mut ops: Vec<String> = Vec::new();
        walk(
            r#"<r>
                <a>1</a>
                <b><c>2</c></b>
            </r>"#,
            |p, evt| {
                let name = p.0.last().cloned().unwrap_or_default();
                match evt {
                    XmlEvent::Enter(_) => ops.push(format!("enter:{name}")),
                    XmlEvent::Leaf(_) => ops.push(format!("leaf:{name}")),
                    XmlEvent::Exit => ops.push(format!("exit:{name}")),
                }
            },
        )
        .unwrap();
        assert_eq!(
            ops,
            vec![
                "enter:r", "enter:a", "leaf:a", "exit:a", "enter:b", "enter:c", "leaf:c", "exit:c",
                "exit:b", "exit:r",
            ],
        );
    }

    #[test]
    fn is_matches_exact_path() {
        let owned = vec!["a".to_string(), "b".to_string()];
        assert!(XmlPath(&owned).is(&["a", "b"]));
        assert!(!XmlPath(&owned).is(&["a"]));
        assert!(!XmlPath(&owned).is(&["a", "b", "c"]));
    }

    #[test]
    fn walk_enter_carries_attributes() {
        let mut include: Option<String> = None;
        let mut version: Option<String> = None;
        walk(
            r#"<Project>
                <PackageReference Include="Newtonsoft.Json" Version="13.0.3" />
            </Project>"#,
            |p, evt| {
                if let (true, XmlEvent::Enter(attrs)) =
                    (p.is(&["Project", "PackageReference"]), evt)
                {
                    include = attr(attrs, "Include").map(String::from);
                    version = attr(attrs, "Version").map(String::from);
                }
            },
        )
        .unwrap();
        assert_eq!(include.as_deref(), Some("Newtonsoft.Json"));
        assert_eq!(version.as_deref(), Some("13.0.3"));
    }

    #[test]
    fn ends_with_matches_suffix() {
        let owned = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert!(XmlPath(&owned).ends_with(&["b", "c"]));
        assert!(XmlPath(&owned).ends_with(&["c"]));
        assert!(!XmlPath(&owned).ends_with(&["a", "c"]));
    }
}
