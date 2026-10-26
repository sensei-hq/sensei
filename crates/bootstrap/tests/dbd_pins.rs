//! One dbd version across the tree. The daemon embeds `dbd-core` to deploy its
//! own schema, CI provisions its test database with the `dbd` CLI, and the
//! release deploys the dōjō's schema with it — three readers of one DDL tree.
//! When they drift, CI and the release apply a schema differently from an
//! install; it drifted to four versions once (#197), and again to two (0.12.1).

use std::path::Path;

const PINNED_IN: [&str; 5] = [
    "crates/bootstrap/Cargo.toml",
    "crates/senseid/Cargo.toml",
    ".github/workflows/rust.yml",
    ".github/workflows/release.yml",
    ".github/workflows/dojo-db.yml",
];

/// Every `tag` given for a `sensei-hq/dbd` git source in `text`.
fn dbd_tags(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.contains("github.com/sensei-hq/dbd"))
        .filter_map(|line| {
            let rest = &line[line.find("v0.").or_else(|| line.find("v1."))?..];
            let end =
                rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '.')).unwrap_or(rest.len());
            Some(rest[..end].to_string())
        })
        .collect()
}

#[test]
fn every_dbd_pin_names_one_version() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut pins = Vec::new();
    for file in PINNED_IN {
        let text =
            std::fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        let tags = dbd_tags(&text);
        assert!(!tags.is_empty(), "{file} pins no dbd version — the list above is stale");
        pins.extend(tags.into_iter().map(|tag| (file, tag)));
    }
    let first = &pins[0].1;
    let apart: Vec<_> = pins.iter().filter(|(_, tag)| tag != first).collect();
    assert!(apart.is_empty(), "dbd pins disagree: {pins:#?}");
}

#[test]
fn a_tag_is_read_off_a_cargo_dependency_and_a_cargo_install() {
    assert_eq!(
        dbd_tags(r#"dbd-core = { git = "https://github.com/sensei-hq/dbd", tag = "v0.26.0" }"#),
        ["v0.26.0"]
    );
    assert_eq!(
        dbd_tags(
            "run: cargo install --git https://github.com/sensei-hq/dbd --tag v0.19.0 dbd-cli --locked"
        ),
        ["v0.19.0"]
    );
}
