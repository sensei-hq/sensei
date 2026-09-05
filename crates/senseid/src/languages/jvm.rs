//! Resolution rules shared by the JVM-family producers (java, kotlin).
//!
//! These lived as private fns inside `mod java_fqn`, so Kotlin could not call
//! them and the DRY rule forbids copying them. Copying is also what the code
//! they came from exists to prevent: `2aaf6a09` moved java's CALL path onto the
//! package-ROOT rule and left `resolve_supertype` on a JDK allowlist, so one
//! adapter answered two ways about `org.springframework` — a `lib·` node when
//! CALLED and a fabricated first-party `java·` node when EXTENDED (`b9818af5`,
//! 125 live edges). One definition, two callers, no drift.
//!
//! `lang` is a parameter rather than a constant because a Kotlin type and a Java
//! type in one package must be addressable alike — JVM projects mix them, and an
//! import resolves to whichever declared it.

use super::fqn;

/// The first two segments of a JVM package — its project ROOT.
///
/// `com.acme.svc` and `com.acme.core` share `com.acme` and are one project;
/// `org.mockito` does not. Two segments because the reverse-domain convention
/// puts the owning organisation there, which is exactly the boundary between
/// "our code" and "a dependency".
pub fn package_root(pkg: &str) -> &str {
    match pkg.match_indices('.').nth(1) {
        Some((i, _)) => &pkg[..i],
        None => pkg,
    }
}

/// Is `pkg` first-party relative to `own_package`? — they share a package ROOT.
///
/// The SINGLE test both the call path and the heritage path ask, so the two
/// cannot drift apart again.
///
/// An empty package on either side is not evidence of kinship, so it answers
/// false — an unknown owner is a dependency, never silently ours.
pub fn is_first_party(pkg: &str, own_package: &str) -> bool {
    !own_package.is_empty() && !pkg.is_empty() && package_root(pkg) == package_root(own_package)
}

/// Resolve a call on an imported/fully-qualified class `a.b.Foo` → `a.b.Foo.m`.
///
/// FIRST-PARTY when the target shares this file's package root; everything else
/// is a dependency and becomes a `lib` node.
pub fn resolve_type_call(
    lang: &str,
    fqcn: &str,
    method: &str,
    own_package: &str,
) -> (Option<String>, bool, String) {
    let (pkg, cls) = fqcn.rsplit_once('.').unwrap_or(("", fqcn));
    if !is_first_party(pkg, own_package) {
        let top = pkg.split('.').next().unwrap_or(pkg);
        (Some(fqn::lib(top, fqcn, method)), true, method.to_string())
    } else {
        (Some(fqn::method(lang, pkg, "", cls, method)), false, method.to_string())
    }
}

/// Resolve a supertype's simple name to `(fqn, is_lib)`.
///
/// Same two-way split as [`resolve_type_call`] and, sharing [`is_first_party`],
/// the same ANSWER — so a third-party supertype lands on the very key a
/// third-party call already writes.
///
/// An unimported name falls back to THIS file's package, which is how the JVM
/// languages resolve a same-package type — not a guess.
pub fn resolve_supertype(
    lang: &str,
    name: &str,
    imports: &std::collections::HashMap<String, String>,
    package: &str,
) -> Option<(String, bool)> {
    if name.is_empty() {
        return None;
    }
    match imports.get(name) {
        Some(fqcn) => {
            let (pkg, cls) = fqcn.rsplit_once('.').unwrap_or(("", fqcn.as_str()));
            if is_first_party(pkg, package) {
                Some((fqn::item(lang, pkg, "", cls), false))
            } else {
                let top = pkg.split('.').next().unwrap_or(pkg);
                Some((fqn::lib(top, pkg, cls), true))
            }
        }
        None => Some((fqn::item(lang, package, "", name), false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The rule is shared, so it must answer identically for BOTH languages —
    /// that is the whole reason these were lifted out of `mod java_fqn`.
    #[test]
    fn one_rule_answers_alike_for_java_and_kotlin() {
        for lang in ["java", "kotlin"] {
            let third = resolve_type_call(lang, "org.mockito.Mockito", "when", "com.acme.svc");
            assert!(third.1, "{lang}: org.mockito is third-party");
            assert_eq!(third.0.as_deref(), Some("lib·org·org.mockito.Mockito·when"));

            let own =
                resolve_type_call(lang, "com.acme.core.BaseService", "helper", "com.acme.svc");
            assert!(!own.1, "{lang}: com.acme.core shares the project root");
            assert_eq!(
                own.0.as_deref(),
                Some(&format!("{lang}·com.acme.core·BaseService·helper")[..])
            );
        }
    }

    #[test]
    fn package_root_takes_two_segments_and_tolerates_shorter_names() {
        assert_eq!(package_root("com.acme.svc"), "com.acme");
        assert_eq!(package_root("com.acme"), "com.acme");
        assert_eq!(package_root("java.util"), "java.util");
        assert_eq!(package_root("single"), "single");
    }

    /// An unknown owner is a dependency, never silently ours.
    #[test]
    fn an_empty_package_on_either_side_is_not_kinship() {
        assert!(!is_first_party("", "com.acme.svc"));
        assert!(!is_first_party("com.acme.svc", ""));
        assert!(is_first_party("com.acme.core", "com.acme.svc"));
        assert!(!is_first_party("org.springframework.web", "com.acme.svc"));
    }

    /// The heritage path splits the same way the call path does.
    #[test]
    fn a_supertype_splits_the_same_way_a_call_does() {
        let mut imports = HashMap::new();
        imports.insert("BaseService".to_string(), "com.acme.core.BaseService".to_string());
        imports.insert(
            "HandlerInterceptor".to_string(),
            "org.springframework.web.servlet.HandlerInterceptor".to_string(),
        );

        let own = resolve_supertype("kotlin", "BaseService", &imports, "com.acme.svc").unwrap();
        assert!(!own.1);
        assert_eq!(own.0, "kotlin·com.acme.core·BaseService");

        let third =
            resolve_supertype("kotlin", "HandlerInterceptor", &imports, "com.acme.svc").unwrap();
        assert!(third.1, "org.springframework is third-party on the heritage path too");
        assert!(third.0.starts_with("lib·"));

        // Unimported ⇒ same package, which is the language rule.
        let same = resolve_supertype("java", "Sibling", &imports, "com.acme.svc").unwrap();
        assert_eq!(same.0, "java·com.acme.svc·Sibling");
        assert!(!same.1);
    }
}
