//! Compiles the database schema into the binary (see `src/schema.rs`).
//!
//! Only GIT-TRACKED files under `database/` are embedded. A developer's
//! checkout also holds gitignored personal capture (session payloads staged in
//! `database/import/staging/`); walking the directory would put that into a
//! binary. `git ls-files` cannot see it, so it can never ship.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

include!("src/schema_select.rs");

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().unwrap_or_else(|e| {
        panic!("sensei-bootstrap build needs git to list the schema files: {e}")
    });
    if !out.status.success() {
        panic!(
            "`git {}` failed — the schema is embedded from the git index, so the build must run in a git checkout: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out.stdout
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().expect("workspace root");

    // Adding or removing a tracked file changes the index.
    let index = String::from_utf8(git(
        &root,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    ))
    .unwrap();
    println!("cargo:rerun-if-changed={}", index.trim());

    let listing = git(&root, &["ls-files", "-z", "database"]);
    let mut code = String::from("pub static EMBEDDED_SCHEMA: &[(&str, &[u8])] = &[\n");
    let mut count = 0;
    for path in listing.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = std::str::from_utf8(path).expect("utf-8 schema path");
        let Some(rel) = path.strip_prefix("database/") else { continue };
        let abs = root.join(path);
        // A tracked file deleted in the working tree is not part of this build.
        if !is_shipped(rel) || !abs.is_file() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", abs.display());
        let abs = abs.to_str().expect("utf-8 schema path");
        writeln!(code, "    ({rel:?}, include_bytes!({abs:?})),").unwrap();
        count += 1;
    }
    code.push_str("];\n");
    assert!(
        count > 0,
        "no schema files found under database/ — refusing to build a binary that cannot create its database"
    );

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded_schema.rs");
    std::fs::write(out, code).unwrap();
}
