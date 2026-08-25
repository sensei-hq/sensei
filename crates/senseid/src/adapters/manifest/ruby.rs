//! `RubyManifestAdapter` — `Gemfile` (#89).
//!
//! Bundler's Gemfile is a Ruby DSL; dependencies are declared via `gem`
//! calls with an optional version string and a `:group => :name` key.
//! Real-world variety this v1 covers:
//! - `gem 'rails'`
//! - `gem 'rails', '~> 7.0'`
//! - `gem 'byebug', group: :development`
//! - `gem "puma", "~> 6.0", require: false`
//! - `gem 'rspec', groups: [:test, :development]`
//!
//! Not covered in v1:
//! - `git:`/`github:`/`path:` deps — local-source deps (needs a
//!   `DepVersion.local_source` mapping like npm/cargo have).
//! - `gemspec` files — separate manifest with different rules.
//! - The `group do ... end` block form for grouping multiple gems.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use regex::Regex;
use std::sync::OnceLock;

pub struct RubyManifestAdapter;

impl ManifestAdapter for RubyManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["Gemfile"]
    }

    fn ecosystem(&self) -> &'static str {
        "rubygems"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let stripped = strip_ruby_comments(content);
        let mut out = Vec::new();
        for cap in gem_re().captures_iter(&stripped) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let version =
                cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_else(|| "*".to_string());
            let rest = cap.get(3).map(|m| m.as_str()).unwrap_or("");
            let dev = is_dev_group(rest);
            out.push(DepVersion {
                lib_name: name,
                version: version.clone(),
                raw_version: version,
                source: "Gemfile".into(),
                dev,
                local_source: None,
            });
        }
        out
    }

    fn is_workspace_root(&self, _content: &str) -> bool {
        false
    }

    fn parse_manifest(&self, _content: &str) -> ParsedManifest {
        ParsedManifest::default()
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["ruby"]
    }

    /// Conventional Ruby entry points. `rake test` covers the standard test
    /// runner; `bundle install` is universally the setup step; `bundle
    /// exec …` is the runtime wrapper. Rake task discovery from Rakefile
    /// itself is a follow-up (Ruby DSL parse is out of scope for this cut).
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands(
            "bundle",
            &[("exec rake test", "test"), ("exec rake", "run"), ("install", "run")],
        )
    }
}

/// Match a `gem` call. Captures:
///   1: name (quoted string, single or double)
///   2: optional version (second positional quoted argument)
///   3: everything after the second argument (for group detection)
fn gem_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // (?m) enables ^ multiline. \b keeps `gem` a word so `polygem` doesn't
        // match.
        Regex::new(r#"(?m)^\s*gem\s+['"]([^'"]+)['"](?:\s*,\s*['"]([^'"]+)['"])?([^\n]*)"#).unwrap()
    })
}

fn strip_ruby_comments(content: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"#[^\n]*").unwrap());
    re.replace_all(content, "").into_owned()
}

/// True when the trailing `gem` args mention a dev/test group. Matches:
/// - `group: :development` / `group: :test`
/// - `groups: [:test, :development]`
fn is_dev_group(rest: &str) -> bool {
    static GROUP_ONE: OnceLock<Regex> = OnceLock::new();
    static GROUP_MANY: OnceLock<Regex> = OnceLock::new();
    let one = GROUP_ONE.get_or_init(|| Regex::new(r"group\s*:\s*:(development|test)\b").unwrap());
    let many = GROUP_MANY
        .get_or_init(|| Regex::new(r"groups?\s*:\s*\[[^\]]*(:development|:test)[^\]]*\]").unwrap());
    one.is_match(rest) || many.is_match(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        let a = RubyManifestAdapter;
        assert_eq!(a.ecosystem(), "rubygems");
        assert_eq!(a.manifest_filenames(), &["Gemfile"]);
    }

    #[test]
    fn parse_dependencies_reads_bare_and_versioned_gems() {
        let src = r#"
            source 'https://rubygems.org'
            gem 'rails', '~> 7.0'
            gem 'puma'
            gem "byebug", "~> 11.1", require: false
        "#;
        let deps = RubyManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 3);
        let by = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert_eq!(by("rails").version, "~> 7.0");
        assert_eq!(by("puma").version, "*", "unversioned gem defaults to '*'");
        assert_eq!(by("byebug").version, "~> 11.1");
    }

    #[test]
    fn parse_dependencies_flags_dev_and_test_groups() {
        let src = r#"
            gem 'rspec', group: :test
            gem 'pry', group: :development
            gem 'yard', groups: [:development, :test]
            gem 'rack', group: :production
        "#;
        let deps = RubyManifestAdapter.parse_dependencies(src);
        let by = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert!(by("rspec").dev);
        assert!(by("pry").dev);
        assert!(by("yard").dev);
        assert!(!by("rack").dev);
    }

    #[test]
    fn parse_dependencies_skips_commented_lines() {
        let src = r#"
            # gem 'hidden', '1.0'
            gem 'real', '2.0'
        "#;
        let deps = RubyManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "real");
    }
}
