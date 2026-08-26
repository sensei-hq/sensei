//! Signals derived from tool ARGUMENTS rather than from the call itself.
//!
//! Three of the four questions a retrospective needs — what languages you work
//! in, how often you commit, how fast you reply — are already sitting in the
//! transcript as tool arguments. None of them needs a model to read.
//!
//! Every ACP names its arguments differently (`path` in Copilot, `file_path` in
//! Claude Code) but the SHAPES are the same, so the extraction lives here once
//! and each adapter passes its own key.

/// The language a file path implies, or `None` when the extension says nothing.
///
/// Deliberately a fixed table rather than a guess: an unknown extension returns
/// `None` and is left out of the tally, instead of being bucketed as "other"
/// and inflating a language nobody works in.
pub fn language_of(path: &str) -> Option<&'static str> {
    // Take the extension off the last segment, so a dotted DIRECTORY
    // (`~/.claude/projects/foo`) cannot be read as an extension.
    let file = path.rsplit(['/', '\\']).next()?;
    let ext = file.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => "Rust",
        "ts" | "tsx" | "mts" | "cts" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "svelte" => "Svelte",
        "vue" => "Vue",
        "py" => "Python",
        // This repo writes schema as `.ddl`; treating it as unknown would hide
        // most of the database work from the tally.
        "sql" | "ddl" => "SQL",
        "md" | "mdx" | "markdown" => "Markdown",
        "json" | "jsonc" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "cs" => "C#",
        "java" => "Java",
        "go" => "Go",
        "rb" => "Ruby",
        "php" => "PHP",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "sh" | "bash" | "zsh" => "Shell",
        "ps1" | "psm1" => "PowerShell",
        "xml" => "XML",
        _ => return None,
    })
}

/// How many commits and pushes a shell command performs.
///
/// Counted per SEGMENT, because one call routinely chains several
/// (`cd repo && git add -A && git commit -m x && git push`), and a whole-string
/// search would score that as one.
///
/// A segment counts only when its FIRST token is `git`. Searching anywhere in
/// the segment would score `echo "run git commit first"` as a commit, and commit
/// messages talk about committing constantly.
pub fn git_actions(command: &str) -> (usize, usize) {
    let (mut commits, mut pushes) = (0usize, 0usize);
    for segment in command.split(['\n', ';', '|', '&']) {
        let mut tokens = segment.split_whitespace();
        if tokens.next() != Some("git") {
            continue;
        }
        // Skip the options that sit between `git` and its subcommand, along
        // with the values of the ones that take a separate argument.
        let mut skip_value = false;
        for t in tokens {
            if skip_value {
                skip_value = false;
                continue;
            }
            if t.starts_with('-') {
                skip_value = matches!(t, "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace");
                continue;
            }
            match t {
                "commit" => commits += 1,
                "push" => pushes += 1,
                _ => {}
            }
            break;
        }
    }
    (commits, pushes)
}

/// The file path a tool call addresses, if it addresses exactly one.
///
/// Search tools are deliberately excluded by the caller passing no key for
/// them: `grep`'s `path` is a search ROOT, not a file worked on, and counting
/// a repo-root grep as work in every language under it would swamp the tally.
pub fn path_argument<'a>(args: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| args.get(*k).and_then(|v| v.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    // Owned and tested by the shared crate; imported here only to assert how
    // THIS crate maps a lifted file link onto a language.
    use sensei_transcript_formats::paths::file_uris;

    #[test]
    fn language_reads_the_extension() {
        assert_eq!(language_of("crates/senseid/src/lib.rs"), Some("Rust"));
        assert_eq!(language_of("app/src/routes/+page.svelte"), Some("Svelte"));
        assert_eq!(language_of("database/ddl/table/sensei/metrics.ddl"), Some("SQL"));
        assert_eq!(language_of(r"D:\Development\sample-ui\src\app.ts"), Some("TypeScript"));
    }

    #[test]
    fn unknown_extensions_are_left_out_rather_than_bucketed() {
        assert_eq!(language_of("target/debug/senseid"), None);
        assert_eq!(language_of("notes.qqq"), None);
    }

    /// The LAST dot in the file name decides, not the first — `app.config.ts`
    /// is TypeScript, and a dotted directory in the path changes nothing.
    #[test]
    fn the_last_dot_in_the_file_name_decides() {
        assert_eq!(language_of("src/app.config.ts"), Some("TypeScript"));
        assert_eq!(language_of("/Users/j/.claude/hooks/pre.commit.sh"), Some("Shell"));
        assert_eq!(language_of("/Users/j/.claude/projects/some-slug"), None);
    }

    #[test]
    fn counts_each_chained_git_action_separately() {
        assert_eq!(git_actions("git add -A && git commit -m x && git push"), (1, 1));
        assert_eq!(git_actions("cd repo; git commit -m 'a'; git commit -m 'b'"), (2, 0));
    }

    #[test]
    fn options_between_git_and_its_subcommand_are_skipped() {
        assert_eq!(git_actions("git --no-pager commit -m x"), (1, 0));
        assert_eq!(git_actions("git -C /tmp/repo push origin main"), (0, 1));
    }

    /// Commit messages talk about committing and pushing constantly. Only a
    /// segment that actually STARTS with `git` may count.
    #[test]
    fn talking_about_a_commit_is_not_a_commit() {
        // Deliberately unquoted: a quote glued to the token would make this
        // pass for the wrong reason, hiding a match-anywhere implementation.
        assert_eq!(git_actions("echo remember to git commit"), (0, 0));
        assert_eq!(git_actions("# next step: git push origin main"), (0, 0));
        assert_eq!(git_actions("git commit -m 'fix: push to remote on failure'"), (1, 0));
    }

    #[test]
    fn reading_the_log_is_neither() {
        assert_eq!(git_actions("git --no-pager log --oneline -5"), (0, 0));
        assert_eq!(git_actions("git status --short"), (0, 0));
    }

    /// A file lifted out of a rendered message must map to a language — the
    /// extraction itself is the shared crate's; this is the mapping.
    #[test]
    fn a_lifted_file_link_maps_to_its_language() {
        let msg = "Reading [](file:///c%3A/Users/r/app/src/main.ts) and done";
        assert_eq!(language_of(&file_uris(msg)[0]), Some("TypeScript"));
    }

    /// A directory or a glob has no recognisable extension, so it drops out on
    /// the language lookup — searches never count as work in a language.
    #[test]
    fn directories_and_globs_do_not_become_languages() {
        let dir = "Reading [](file:///c%3A/Users/r/app/src/assets/i18n)";
        assert_eq!(language_of(&file_uris(dir)[0]), None);
        assert!(file_uris("Searching for files matching `**/src/app/**`").is_empty());
    }

    #[test]
    fn path_argument_takes_the_first_key_present() {
        let a = serde_json::json!({"file_path": "a.rs"});
        assert_eq!(path_argument(&a, &["path", "file_path"]), Some("a.rs"));
        let b = serde_json::json!({"pattern": "foo"});
        assert_eq!(path_argument(&b, &["path", "file_path"]), None);
    }
}
