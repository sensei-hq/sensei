//! Which builds and tests a shell command ran, and how each ended (spec S3).
//!
//! The catalogue is data (`sensei.command_patterns`, `sensei.command_markers`);
//! this module only applies it. A run's outcome comes from its own output first
//! — most agent commands are piped, so the shell's exit status belongs to `tail`
//! or `grep` — and a marker decides an outcome only when exactly one stage of the
//! command can have printed it.

/// What a classified run checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Build,
    Typecheck,
    Test,
    E2e,
}

impl Category {
    pub fn parse(s: &str) -> Option<Category> {
        match s {
            "build" => Some(Category::Build),
            "typecheck" => Some(Category::Typecheck),
            "test" => Some(Category::Test),
            "e2e" => Some(Category::E2e),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Category::Build => "build",
            Category::Typecheck => "typecheck",
            Category::Test => "test",
            Category::E2e => "e2e",
        }
    }

    /// Build-family runs (`build`, `typecheck`) become `build_*` signals, the
    /// test family (`test`, `e2e`) `test_*` (spec S2).
    pub fn is_build_family(self) -> bool {
        matches!(self, Category::Build | Category::Typecheck)
    }
}

/// Which hook event closed the call: the command's exit, as Claude Code reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// `PostToolUse`: the shell exited 0.
    Ok,
    /// `PostToolUseFailure`: the shell exited non-zero.
    Fail,
}

/// One run the command made, and what is known of its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunSignal {
    Ok(Category),
    Fail(Category),
    Unknown(Category),
    /// The command failed and ran nothing the catalogue classifies.
    UnclassifiedFail,
}

/// A `sensei.command_patterns` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternRow {
    pub toolchain: String,
    pub pattern: String,
    pub category: Option<String>,
    pub wrapper: bool,
    pub compiles: bool,
    /// For a wrapper: the toolchains whose markers it may claim; `None` = any.
    pub owns: Option<Vec<String>>,
    pub priority: i32,
}

/// A `sensei.command_markers` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerRow {
    pub toolchain: String,
    pub category: String,
    pub kind: String,
    pub marker: String,
}

/// A `sensei.folder_commands` command of the session's repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderCommand {
    pub command_line: String,
    pub category: Option<String>,
}

/// The compiled catalogue, ready to classify with.
#[derive(Debug)]
pub struct Catalogue {
    /// `md5` over both tables (spec S3): a change re-derives every session.
    pub hash: String,
    /// By priority, then pattern length, both descending: the first match wins.
    patterns: Vec<Pattern>,
    markers: Vec<Marker>,
}

#[derive(Debug)]
struct Pattern {
    toolchain: String,
    regex: regex::Regex,
    category: Option<String>,
    wrapper: bool,
    compiles: bool,
    owns: Option<Vec<String>>,
}

#[derive(Debug)]
struct Marker {
    toolchain: String,
    category: Category,
    fail: bool,
    matcher: Matcher,
}

#[derive(Debug)]
enum Matcher {
    Literal(String),
    Regex(regex::Regex),
}

impl Matcher {
    fn is_in(&self, text: &str) -> bool {
        match self {
            Matcher::Literal(s) => text.contains(s.as_str()),
            Matcher::Regex(r) => r.is_match(text),
        }
    }
}

impl Catalogue {
    /// Compile the rows. A pattern or regex marker that does not compile is an
    /// error naming the row, never a silently skipped one.
    pub fn compile(
        patterns: Vec<PatternRow>,
        markers: Vec<MarkerRow>,
        hash: String,
    ) -> Result<Catalogue, String> {
        let mut rows = patterns;
        rows.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then(b.pattern.len().cmp(&a.pattern.len()))
        });
        let patterns = rows
            .into_iter()
            .map(|r| {
                let regex = regex::Regex::new(&r.pattern).map_err(|e| {
                    format!("command_patterns ({}, {}): {e}", r.toolchain, r.pattern)
                })?;
                Ok(Pattern {
                    toolchain: r.toolchain,
                    regex,
                    category: r.category,
                    wrapper: r.wrapper,
                    compiles: r.compiles,
                    owns: r.owns,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let markers = markers
            .into_iter()
            .map(|r| {
                let named = || {
                    format!(
                        "command_markers ({}, {}, {}, {})",
                        r.toolchain, r.category, r.kind, r.marker
                    )
                };
                let category = Category::parse(&r.category)
                    .ok_or_else(|| format!("{}: unknown category", named()))?;
                let (fail, matcher) = match r.kind.as_str() {
                    "fail" => (true, Matcher::Literal(r.marker.clone())),
                    "ok" => (false, Matcher::Literal(r.marker.clone())),
                    "fail_re" | "ok_re" => (
                        r.kind == "fail_re",
                        Matcher::Regex(
                            regex::Regex::new(&r.marker)
                                .map_err(|e| format!("{}: {e}", named()))?,
                        ),
                    ),
                    _ => return Err(format!("{}: unknown kind", named())),
                };
                Ok(Marker { toolchain: r.toolchain, category, fail, matcher })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Catalogue { hash, patterns, markers })
    }
}

/// How a list element was entered: the operator before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Start,
    And,
    Or,
    Then,
}

/// One list element: a pipeline of stages, each a command's words.
#[derive(Debug, Default)]
struct Element {
    entry: Option<Entry>,
    stages: Vec<Vec<String>>,
    /// Ended by `&`: it runs in the background, so the shell's exit is never its.
    background: bool,
}

/// Split a command into list elements and pipeline stages with shell quoting
/// rules: quoted text and `$(…)` / backticks are single words, heredoc bodies and
/// comments are skipped (spec S3.1).
fn tokenize(command: &str) -> Vec<Element> {
    let chars: Vec<char> = command.chars().collect();
    let mut elements =
        vec![Element { entry: Some(Entry::Start), stages: vec![Vec::new()], background: false }];
    let mut word = String::new();
    let mut in_word = false;
    let mut heredocs: Vec<(String, bool)> = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;

    fn flush(word: &mut String, in_word: &mut bool, elements: &mut [Element]) {
        if *in_word {
            let el = elements.last_mut().expect("one element");
            el.stages.last_mut().expect("one stage").push(std::mem::take(word));
            *in_word = false;
        }
    }
    fn next_element(entry: Entry, elements: &mut Vec<Element>) {
        elements.push(Element { entry: Some(entry), stages: vec![Vec::new()], background: false });
    }

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if i + 1 < chars.len() => {
                if chars[i + 1] != '\n' {
                    word.push(chars[i + 1]);
                    in_word = true;
                }
                i += 2;
            }
            '\'' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    word.push(chars[i]);
                    i += 1;
                }
                i += 1;
            }
            '"' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        word.push(chars[i + 1]);
                        i += 2;
                    } else if chars[i] == '$' && chars.get(i + 1) == Some(&'(') {
                        i = skip_subst(&chars, i + 1, &mut word);
                    } else if chars[i] == '`' {
                        i = skip_backtick(&chars, i, &mut word);
                    } else {
                        word.push(chars[i]);
                        i += 1;
                    }
                }
                i += 1;
            }
            '$' if chars.get(i + 1) == Some(&'(') => {
                in_word = true;
                i = skip_subst(&chars, i + 1, &mut word);
            }
            '`' => {
                in_word = true;
                i = skip_backtick(&chars, i, &mut word);
            }
            '#' if !in_word => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            // `<<<` is a here-string, a redirection: not a heredoc.
            '<' if chars.get(i + 1) == Some(&'<') && chars.get(i + 2) == Some(&'<') => {
                word.push_str("<<<");
                in_word = true;
                i += 3;
            }
            '<' if chars.get(i + 1) == Some(&'<') => {
                flush(&mut word, &mut in_word, &mut elements);
                i += 2;
                let strip = chars.get(i) == Some(&'-');
                if strip {
                    i += 1;
                }
                while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                    i += 1;
                }
                let mut delim = String::new();
                while i < chars.len() && !" \t\n;&|<>()".contains(chars[i]) {
                    if chars[i] != '\'' && chars[i] != '"' && chars[i] != '\\' {
                        delim.push(chars[i]);
                    }
                    i += 1;
                }
                if !delim.is_empty() {
                    heredocs.push((delim, strip));
                }
            }
            '\n' => {
                flush(&mut word, &mut in_word, &mut elements);
                i += 1;
                for (delim, strip) in std::mem::take(&mut heredocs) {
                    loop {
                        let start = i;
                        while i < chars.len() && chars[i] != '\n' {
                            i += 1;
                        }
                        let line: String = chars[start..i].iter().collect();
                        i = (i + 1).min(chars.len());
                        let line =
                            if strip { line.trim_start_matches('\t') } else { line.as_str() };
                        if line == delim || i >= chars.len() {
                            break;
                        }
                    }
                }
                next_element(Entry::Then, &mut elements);
            }
            '&' if chars.get(i + 1) == Some(&'&') => {
                flush(&mut word, &mut in_word, &mut elements);
                next_element(Entry::And, &mut elements);
                i += 2;
            }
            '|' if chars.get(i + 1) == Some(&'|') => {
                flush(&mut word, &mut in_word, &mut elements);
                next_element(Entry::Or, &mut elements);
                i += 2;
            }
            '|' => {
                flush(&mut word, &mut in_word, &mut elements);
                elements.last_mut().expect("one element").stages.push(Vec::new());
                i += if chars.get(i + 1) == Some(&'&') { 2 } else { 1 };
            }
            // `2>&1`, `>&2`, `&>file`: redirections, part of the word.
            '&' if word.ends_with('>') || word.ends_with('<') || chars.get(i + 1) == Some(&'>') => {
                word.push(c);
                in_word = true;
                i += 1;
            }
            ';' | '&' => {
                flush(&mut word, &mut in_word, &mut elements);
                if c == '&' {
                    elements.last_mut().expect("one element").background = true;
                }
                next_element(Entry::Then, &mut elements);
                i += if c == ';' && chars.get(i + 1) == Some(&';') { 2 } else { 1 };
            }
            // An unmatched `)` closes a `case` arm's pattern: the command follows it.
            ')' if depth == 0 => {
                flush(&mut word, &mut in_word, &mut elements);
                next_element(Entry::Then, &mut elements);
                i += 1;
            }
            '(' | ')' => {
                flush(&mut word, &mut in_word, &mut elements);
                if c == '(' {
                    depth += 1;
                } else {
                    depth -= 1;
                }
                elements
                    .last_mut()
                    .expect("one element")
                    .stages
                    .last_mut()
                    .expect("one stage")
                    .push(c.to_string());
                i += 1;
            }
            ' ' | '\t' | '\r' => {
                flush(&mut word, &mut in_word, &mut elements);
                i += 1;
            }
            _ => {
                word.push(c);
                in_word = true;
                i += 1;
            }
        }
    }
    flush(&mut word, &mut in_word, &mut elements);
    elements
}

/// Copy a `$(…)` (from the `(`) into `word` as one opaque word; returns the
/// index after its closing `)`.
fn skip_subst(chars: &[char], open: usize, word: &mut String) -> usize {
    let mut depth = 0;
    let mut i = open;
    while i < chars.len() {
        match chars[i] {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            '\'' => {
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
            }
            _ => {}
        }
        word.push('_');
        i += 1;
    }
    i
}

fn skip_backtick(chars: &[char], open: usize, word: &mut String) -> usize {
    let mut i = open + 1;
    while i < chars.len() && chars[i] != '`' {
        word.push('_');
        i += 1;
    }
    i + 1
}

/// Words that precede the command itself (spec S3.1).
const PREFIX_WORDS: &[&str] =
    &["!", "{", "(", "do", "then", "else", "elif", "if", "while", "until"];
/// Commands that run the command after them, with the options of theirs that take
/// a separate argument (`timeout -k 5 10 cargo test`).
const PREFIX_COMMANDS: &[(&str, &[&str])] = &[
    ("timeout", &["-k", "-s", "--kill-after", "--signal"]),
    ("nice", &["-n", "--adjustment"]),
    ("sudo", &["-u", "-g", "-C", "-D", "-h", "-p", "-r", "-t", "-U"]),
    ("time", &["-f", "-o"]),
    ("exec", &["-a"]),
    ("command", &[]),
];
/// A stage starting with one of these is not a command.
const NO_OP_WORDS: &[&str] = &[
    "cd", "pushd", "popd", "for", "case", "select", "function", "done", "fi", "esac", "}", ")",
    "in",
];

/// The command a stage runs, with its prefixes stripped; `None` for a no-op stage.
fn command_of(words: &[String]) -> Option<String> {
    let mut rest = words;
    loop {
        let first = rest.first()?;
        let w = first.as_str();
        if PREFIX_WORDS.contains(&w) || is_env_assignment(w) {
            rest = &rest[1..];
        } else if let Some((_, takes)) = PREFIX_COMMANDS.iter().find(|(c, _)| *c == w) {
            rest = &rest[1..];
            while let Some(opt) = rest.first().filter(|o| o.starts_with('-')) {
                let skip = if takes.contains(&opt.as_str()) { 2 } else { 1 };
                rest = rest.get(skip..).unwrap_or(&[]);
            }
            if w == "timeout" {
                rest = rest.get(1..).unwrap_or(&[]);
            }
        } else if NO_OP_WORDS.contains(&w) {
            return None;
        } else {
            return Some(rest.join(" "));
        }
    }
}

fn is_env_assignment(w: &str) -> bool {
    let Some((name, _)) = w.split_once('=') else { return false };
    !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A stage the catalogue recognised.
#[derive(Debug)]
struct Stage {
    /// `None` for a repository folder command: its toolchain is unknown.
    toolchain: Option<String>,
    /// `None` for a recognised command that is not a run (`none`, lint, bench…).
    run: Option<Category>,
    wrapper: bool,
    compiles: bool,
    owns: Option<Vec<String>>,
    element: usize,
    last_in_element: bool,
}

/// What the catalogue says a stage is: toolchain, run category, wrapper, compiles,
/// and the toolchains a wrapper may claim.
type Recognised = (Option<String>, Option<Category>, bool, bool, Option<Vec<String>>);

/// What the catalogue says a stage is (spec S3.2); `None` when it knows nothing.
///
/// An explicit catalogue row keeps its toolchain and category, so its output
/// markers stay its own. A repository folder command supplies the category only
/// for a wrapper (`bun run <name>`), or for a command the catalogue does not know.
fn recognise(
    command: &str,
    catalogue: &Catalogue,
    folder_commands: &[FolderCommand],
) -> Option<Recognised> {
    let folder = folder_commands
        .iter()
        .filter(|f| f.category.is_some() && prefixes(&f.command_line, command))
        .max_by_key(|f| f.command_line.len())
        .map(|f| f.category.as_deref().and_then(Category::parse));
    let Some(p) = catalogue.patterns.iter().find(|p| p.regex.is_match(command)) else {
        return folder.map(|run| (None, run, true, false, None));
    };
    let derived = p.wrapper || p.category.is_none();
    let run = match (&p.category, folder) {
        (_, Some(run)) if derived => run,
        (Some(c), _) => Category::parse(c),
        (None, _) => p
            .regex
            .captures(command)
            .and_then(|c| c.get(1))
            .and_then(|name| crate::adapters::manifest::command_category::categorise(name.as_str()))
            .and_then(Category::parse),
    };
    Some((Some(p.toolchain.clone()), run, p.wrapper, p.compiles, p.owns.clone()))
}

/// `line` is `command` or a word-boundary prefix of it.
fn prefixes(line: &str, command: &str) -> bool {
    let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
    command == line || command.starts_with(&format!("{line} "))
}

/// The commands `command` runs, one per stage, with prefixes stripped and no-op
/// stages dropped (spec S3.1) — for recognising a command by its first word.
pub fn stage_commands(command: &str) -> Vec<String> {
    tokenize(command).iter().flat_map(|e| e.stages.iter().filter_map(|w| command_of(w))).collect()
}

/// The runs `command` would make, with no outcome: for a call that never
/// reported one (spec S2 `unmatched_call`).
pub fn run_categories(
    command: &str,
    catalogue: &Catalogue,
    folder_commands: &[FolderCommand],
) -> Vec<Category> {
    stage_commands(command)
        .iter()
        .filter_map(|cmd| recognise(cmd, catalogue, folder_commands))
        .filter_map(|(_, run, ..)| run)
        .collect()
}

/// The runs `command` made and their outcomes, in the order they appear.
pub fn classify(
    command: &str,
    output: &str,
    exit: Exit,
    catalogue: &Catalogue,
    folder_commands: &[FolderCommand],
) -> Vec<RunSignal> {
    let elements = tokenize(command);
    let mut stages: Vec<Stage> = Vec::new();
    let mut commands = 0;
    for (e, element) in elements.iter().enumerate() {
        let n = element.stages.len();
        for (k, words) in element.stages.iter().enumerate() {
            let Some(cmd) = command_of(words) else { continue };
            commands += 1;
            if let Some((toolchain, run, wrapper, compiles, owns)) =
                recognise(&cmd, catalogue, folder_commands)
            {
                stages.push(Stage {
                    toolchain,
                    run,
                    wrapper,
                    compiles,
                    owns,
                    element: e,
                    last_in_element: k + 1 == n,
                });
            }
        }
    }
    if !stages.iter().any(|s| s.run.is_some()) {
        return if exit == Exit::Fail { vec![RunSignal::UnclassifiedFail] } else { Vec::new() };
    }

    // Which stages own each marker the output carries (spec S3.3).
    let mut attributed: Vec<Vec<(Category, bool)>> = vec![Vec::new(); stages.len()];
    let mut ambiguous: Vec<(bool, bool)> = vec![(false, false); stages.len()];
    let mut seen: Vec<(&str, Category, bool)> = Vec::new();
    for m in catalogue.markers.iter().filter(|m| m.matcher.is_in(output)) {
        let key = (m.toolchain.as_str(), m.category, m.fail);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let named = stages.iter().any(|s| s.toolchain.as_deref() == Some(m.toolchain.as_str()));
        let owners: Vec<usize> = (0..stages.len())
            .filter(|&i| {
                let s = &stages[i];
                let own = s.toolchain.as_deref() == Some(m.toolchain.as_str());
                (own && s.run == Some(m.category))
                    || (own && m.fail && m.category == Category::Build && s.compiles)
                    || (!named
                        && s.wrapper
                        && s.run == Some(m.category)
                        && s.owns.as_ref().is_none_or(|o| o.contains(&m.toolchain)))
            })
            .collect();
        match owners.as_slice() {
            [one] => attributed[*one].push((m.category, m.fail)),
            many => {
                for &i in many {
                    if m.fail {
                        ambiguous[i].0 = true;
                    } else {
                        ambiguous[i].1 = true;
                    }
                }
            }
        }
    }

    let lone = commands == 1;
    let mut out = Vec::new();
    for (i, s) in stages.iter().enumerate() {
        let Some(cat) = s.run else { continue };
        let got = &attributed[i];
        let signal = if s.compiles && got.contains(&(Category::Build, true)) {
            RunSignal::Fail(Category::Build)
        } else if got.contains(&(cat, true)) {
            RunSignal::Fail(cat)
        } else if ambiguous[i].0 {
            RunSignal::Unknown(cat)
        } else if exit == Exit::Ok && got.contains(&(cat, false)) {
            RunSignal::Ok(cat)
        } else if ambiguous[i].1 {
            RunSignal::Unknown(cat)
        } else {
            by_exit(s, cat, exit, lone, &elements)
        };
        out.push(signal);
    }
    out
}

/// The outcome from the shell's exit, only when it is this stage's (spec S3.4).
fn by_exit(s: &Stage, cat: Category, exit: Exit, lone: bool, elements: &[Element]) -> RunSignal {
    match exit {
        Exit::Fail if lone => RunSignal::Fail(cat),
        Exit::Ok
            if s.last_in_element
                && !elements[s.element].background
                && elements[s.element].entry != Some(Entry::Or)
                && elements[s.element + 1..]
                    .iter()
                    .filter(|e| e.stages.iter().any(|w| !w.is_empty()))
                    .all(|e| e.entry == Some(Entry::And)) =>
        {
            RunSignal::Ok(cat)
        }
        _ => RunSignal::Unknown(cat),
    }
}

/// The seeded catalogue, for other modules' tests.
#[cfg(test)]
pub(crate) mod tests_support {
    pub(crate) fn seed() -> super::Catalogue {
        super::tests::seed()
    }
}

#[cfg(test)]
mod tests {
    use super::Category::{Build, E2e, Test, Typecheck};
    use super::RunSignal::{Fail, Ok as Pass, UnclassifiedFail, Unknown};
    use super::*;

    /// The catalogue exactly as seeded, so every case also checks the datafiles.
    pub(super) fn seed() -> Catalogue {
        fn rows(file: &str) -> Vec<serde_json::Value> {
            file.lines().map(|l| serde_json::from_str(l).expect("seed line is JSON")).collect()
        }
        let s = |v: &serde_json::Value, k: &str| v[k].as_str().map(str::to_string);
        let patterns =
            rows(include_str!("../../../../database/import/staging/command_patterns.jsonl"))
                .iter()
                .map(|v| PatternRow {
                    toolchain: s(v, "toolchain").unwrap(),
                    pattern: s(v, "pattern").unwrap(),
                    category: s(v, "category"),
                    wrapper: v["wrapper"].as_bool().unwrap_or(false),
                    compiles: v["compiles"].as_bool().unwrap_or(false),
                    owns: v["owns"]
                        .as_array()
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()),
                    priority: v["priority"].as_i64().unwrap_or(0) as i32,
                })
                .collect();
        let markers =
            rows(include_str!("../../../../database/import/staging/command_markers.jsonl"))
                .iter()
                .map(|v| MarkerRow {
                    toolchain: s(v, "toolchain").unwrap(),
                    category: s(v, "category").unwrap(),
                    kind: s(v, "kind").unwrap(),
                    marker: s(v, "marker").unwrap(),
                })
                .collect();
        Catalogue::compile(patterns, markers, "seed".into()).expect("the seed compiles")
    }

    fn run(cmd: &str, out: &str, exit: Exit) -> Vec<RunSignal> {
        classify(cmd, out, exit, &seed(), &[])
    }

    const CARGO_FAILED: &str = "running 3 tests\ntest result: FAILED. 2 passed; 1 failed\n";
    const CARGO_OK: &str = "test result: ok. 66 passed; 0 failed\n";
    const FINISHED: &str = "    Finished `test` profile [unoptimized] target(s) in 3.1s\n";
    const NO_COMPILE: &str = "error[E0425]: cannot find value `x`\nerror: could not compile `senseid` (bin \"senseid\" test) due to 1 previous error\n";

    #[test]
    fn piped_test_reads_its_marker_or_is_unknown() {
        assert_eq!(run("cargo test 2>&1 | tail -3", CARGO_FAILED, Exit::Ok), [Fail(Test)]);
        assert_eq!(run("cargo test 2>&1 | tail -3", "", Exit::Ok), [Unknown(Test)]);
    }

    #[test]
    fn a_compile_error_inside_cargo_test_is_a_build_fail() {
        assert_eq!(run("cargo test 2>&1 | tail -3", NO_COMPILE, Exit::Ok), [Fail(Build)]);
    }

    #[test]
    fn an_exit_masked_by_a_later_command_is_unknown() {
        assert_eq!(run("cargo test > l 2>&1; echo $?", "0", Exit::Ok), [Unknown(Test)]);
        assert_eq!(run("cargo build || true", "", Exit::Ok), [Unknown(Build)]);
    }

    #[test]
    fn a_failure_is_attributed_only_to_a_lone_stage() {
        assert_eq!(run("cargo build", "", Exit::Fail), [Fail(Build)]);
        assert_eq!(run("cd crates && cargo build", "", Exit::Fail), [Fail(Build)]);
        assert_eq!(run("cargo fmt --check && cargo test", "", Exit::Fail), [Unknown(Test)]);
        assert_eq!(
            run("cargo build && cargo test", "", Exit::Fail),
            [Unknown(Build), Unknown(Test)]
        );
    }

    #[test]
    fn success_under_and_means_every_step_succeeded() {
        assert_eq!(run("cargo build && cargo test", "", Exit::Ok), [Pass(Build), Pass(Test)]);
    }

    #[test]
    fn each_marker_belongs_to_the_stage_that_printed_it() {
        let both = format!("{FINISHED}{CARGO_FAILED}");
        assert_eq!(
            run("cargo build 2>&1 | tail -1; cargo test 2>&1 | grep result", &both, Exit::Ok),
            [Pass(Build), Fail(Test)]
        );
        assert_eq!(
            run(
                "cargo build 2>&1 | tail -1; cargo test 2>&1 | grep result",
                CARGO_FAILED,
                Exit::Ok
            ),
            [Unknown(Build), Fail(Test)]
        );
    }

    #[test]
    fn a_marker_two_stages_could_have_printed_is_no_ones() {
        // Row 1735: two cargo test runs, one passing and one failing.
        let out = format!("{CARGO_OK}{CARGO_FAILED}");
        assert_eq!(
            run(
                "cargo test -p sensei-cli 2>&1 | grep '^test result'; cargo test -p senseid 2>&1 | grep '^test result'",
                &out,
                Exit::Ok
            ),
            [Unknown(Test), Unknown(Test)]
        );
    }

    #[test]
    fn a_wrapper_owns_markers_of_toolchains_no_stage_names() {
        // Row 3508: cargo's marker is cargo's; vitest's is the bun script's.
        let out = format!(
            "{CARGO_OK} Test Files  1 failed | 80 passed (81)\n      Tests  1 failed | 1219 passed (1220)\n"
        );
        assert_eq!(
            run(
                "cargo test -p sensei-cli 2>&1 | tail -2; bun run test:unit 2>&1 | tail -3",
                &out,
                Exit::Ok
            ),
            [Pass(Test), Fail(Test)]
        );
    }

    #[test]
    fn a_test_file_that_fails_to_load_fails_the_run() {
        // Row 6849: every test that loaded passed, but one file did not load.
        let out = " Test Files  1 failed | 193 passed (194)\n      Tests  2349 passed (2349)\n";
        assert_eq!(
            run("bun run test:unit 2>&1 | grep -E 'Test Files|Tests ' | head", out, Exit::Ok),
            [Fail(Test)]
        );
    }

    #[test]
    fn loop_bodies_are_classified() {
        assert_eq!(
            run(
                "for i in 1 2; do cargo test 2>&1 | grep \"test result\"; done",
                CARGO_FAILED,
                Exit::Ok
            ),
            [Fail(Test)]
        );
    }

    #[test]
    fn a_known_non_run_is_not_a_test() {
        // Row 10729.
        assert_eq!(
            run(
                "make test-template 2>&1 | grep stamped; cargo test -p senseid 2>&1 | grep result",
                &format!("stamped sensei_test: fdd8\n{CARGO_FAILED}"),
                Exit::Ok
            ),
            [Fail(Test)]
        );
    }

    #[test]
    fn a_compile_marker_shared_with_clippy_is_ambiguous() {
        let out = format!(
            "{FINISHED}error: used `assert_eq!` with a literal bool\nerror: could not compile `senseid` due to 1 previous error\n"
        );
        assert_eq!(
            run(
                "cargo build 2>&1 | tail -1; cargo clippy -- -D warnings 2>&1 | grep error",
                &out,
                Exit::Ok
            ),
            [Unknown(Build)]
        );
    }

    #[test]
    fn check_scripts_are_typechecks() {
        assert_eq!(
            run(
                "bun run check 2>&1 | tail -1",
                "svelte-check found 2 errors and 0 warnings\n",
                Exit::Ok
            ),
            [Fail(Typecheck)]
        );
        assert_eq!(
            run(
                "bun run check 2>&1 | tail -1",
                "svelte-check found 0 errors and 0 warnings\n",
                Exit::Ok
            ),
            [Pass(Typecheck)]
        );
    }

    #[test]
    fn workspace_options_before_the_verb() {
        assert_eq!(
            run("pnpm --filter a run typecheck 2>&1 | tail -3", "", Exit::Ok),
            [Unknown(Typecheck)]
        );
        assert_eq!(run("pnpm -F @x/web test", "", Exit::Ok), [Pass(Test)]);
    }

    #[test]
    fn quoted_and_heredoc_text_is_not_a_run() {
        assert_eq!(run("rg 'cargo test|make test' f.yml | head", "", Exit::Ok), []);
        let pr = "gh pr create --title x --body \"$(cat <<'EOF'\n## Test plan\ncargo test; svelte-check 0 errors\nEOF\n)\"";
        assert_eq!(run(pr, "", Exit::Ok), []);
        assert_eq!(run("cat <<EOF > notes.md\ncargo test\nEOF", "", Exit::Ok), []);
        assert_eq!(run("git commit -m 'fix: make cargo test pass'", "", Exit::Ok), []);
    }

    #[test]
    fn an_ok_marker_never_passes_a_failed_call() {
        assert_eq!(run("cargo test", FINISHED, Exit::Fail), [Fail(Test)]);
        assert_eq!(run("cargo test 2>&1 | tail -2", FINISHED, Exit::Fail), [Unknown(Test)]);
    }

    #[test]
    fn non_runs_emit_nothing_and_a_failure_of_one_is_unclassified() {
        assert_eq!(run("cargo bench", "", Exit::Ok), []);
        assert_eq!(run("npm run bench", "", Exit::Ok), []);
        assert_eq!(run("cargo bench", "", Exit::Fail), [UnclassifiedFail]);
        assert_eq!(run("ls missing", "", Exit::Fail), [UnclassifiedFail]);
        assert_eq!(run("ls", "", Exit::Ok), []);
    }

    #[test]
    fn folder_commands_come_first_and_a_null_category_falls_through() {
        let typed = [FolderCommand {
            command_line: "bun run verify".into(),
            category: Some("typecheck".into()),
        }];
        assert_eq!(classify("bun run verify", "", Exit::Ok, &seed(), &typed), [Pass(Typecheck)]);
        let untyped = [FolderCommand { command_line: "bun run test:unit".into(), category: None }];
        assert_eq!(classify("bun run test:unit", "", Exit::Ok, &seed(), &untyped), [Pass(Test)]);
    }

    #[test]
    fn one_case_per_seeded_toolchain() {
        let cases: &[(&str, RunSignal)] = &[
            ("cargo build", Pass(Build)),
            ("cargo check", Pass(Build)),
            ("cargo nextest run", Pass(Test)),
            ("go build ./...", Pass(Build)),
            ("go test ./...", Pass(Test)),
            ("dotnet build", Pass(Build)),
            ("dotnet test", Pass(Test)),
            ("./mvnw -q package", Pass(Build)),
            ("mvn test", Pass(Test)),
            ("./gradlew assemble", Pass(Build)),
            ("./gradlew test", Pass(Test)),
            ("npm run build", Pass(Build)),
            ("yarn test", Pass(Test)),
            ("bun test", Pass(Test)),
            ("npx tsc --noEmit", Pass(Typecheck)),
            ("bunx svelte-check", Pass(Typecheck)),
            ("npx vitest run", Pass(Test)),
            ("npx jest", Pass(Test)),
            ("npx playwright test", Pass(E2e)),
            ("make test", Pass(Test)),
            ("make", Pass(Build)),
            ("just e2e", Pass(E2e)),
            ("python -m pytest -q", Pass(Test)),
            ("uv run pytest", Pass(Test)),
            ("tox -e py312", Pass(Test)),
            ("mypy src", Pass(Typecheck)),
            ("npx pyright", Pass(Typecheck)),
            ("cmake --build build", Pass(Build)),
            ("ninja -C build", Pass(Build)),
            ("msbuild app.sln", Pass(Build)),
            ("xcodebuild -scheme App test", Pass(Test)),
            ("xcodebuild -scheme App", Pass(Build)),
            ("swift build", Pass(Build)),
            ("swift test", Pass(Test)),
            ("mix compile", Pass(Build)),
            ("mix test", Pass(Test)),
            ("bundle exec rspec", Pass(Test)),
            ("RUST_LOG=debug timeout 900 cargo test -p x", Pass(Test)),
        ];
        let cat = seed();
        for (cmd, want) in cases {
            assert_eq!(classify(cmd, "", Exit::Ok, &cat, &[]), std::slice::from_ref(want), "{cmd}");
        }
    }

    #[test]
    fn a_bad_pattern_is_an_error_naming_it() {
        let bad = PatternRow {
            toolchain: "x".into(),
            pattern: "(".into(),
            category: Some("build".into()),
            wrapper: false,
            compiles: false,
            owns: None,
            priority: 0,
        };
        let err = Catalogue::compile(vec![bad], vec![], "h".into()).unwrap_err();
        assert!(err.contains("x") && err.contains("("), "{err}");
    }

    // ---- review round 1 (F1 post-implementation swarm) ----

    #[test]
    fn rule_4_never_passes_on_a_failed_exit() {
        assert_eq!(run("cargo test", CARGO_OK, Exit::Fail), [Fail(Test)]);
        assert_eq!(run("cargo test 2>&1 | tail -2", CARGO_OK, Exit::Fail), [Unknown(Test)]);
        assert_eq!(
            run("bun run check", "svelte-check found 0 errors and 0 warnings\n", Exit::Fail),
            [Fail(Typecheck)]
        );
    }

    #[test]
    fn an_ok_marker_two_stages_share_is_unknown() {
        assert_eq!(
            run(
                "cargo test -p a 2>&1 | tail -1; cargo test -p b 2>&1 | tail -1",
                CARGO_OK,
                Exit::Ok
            ),
            [Unknown(Test), Unknown(Test)]
        );
    }

    #[test]
    fn a_compile_error_outranks_a_test_failure() {
        let out = format!("{NO_COMPILE}{CARGO_FAILED}");
        assert_eq!(run("cargo test 2>&1 | tail -5", &out, Exit::Ok), [Fail(Build)]);
    }

    #[test]
    fn a_stage_entered_by_or_never_takes_the_exit() {
        assert_eq!(run("false || cargo test", "", Exit::Ok), [Unknown(Test)]);
        assert_eq!(run("cd x || cargo build", "", Exit::Ok), [Unknown(Build)]);
    }

    #[test]
    fn a_backgrounded_run_never_takes_the_exit() {
        assert_eq!(run("cargo test &", "", Exit::Ok), [Unknown(Test)]);
        assert_eq!(run("cargo build & wait", "", Exit::Ok), [Unknown(Build)]);
    }

    #[test]
    fn heredoc_forms_are_skipped() {
        assert_eq!(run("cat <<-'EOF'\n\tcargo test\n\tEOF", "", Exit::Ok), []);
        assert_eq!(run("cat <<'EOF' | sh\ncargo test\nEOF", "", Exit::Ok), []);
        assert_eq!(run("cat << EOF\ncargo test\nEOF", "", Exit::Ok), []);
        assert_eq!(run("cat <<EOF\nx\nEOF\ncargo test", "", Exit::Ok), [Pass(Test)]);
    }

    #[test]
    fn a_here_string_is_not_a_heredoc() {
        assert_eq!(
            run("grep -q x <<< \"$v\"\ncargo test 2>&1 | tail -2", CARGO_FAILED, Exit::Ok),
            [Fail(Test)]
        );
    }

    #[test]
    fn operators_inside_quotes_and_substitutions_do_not_split() {
        assert_eq!(run("echo \"a; cargo test && make test\"", "", Exit::Ok), []);
        assert_eq!(run("echo 'x' && echo $(cargo test; make)", "", Exit::Ok), []);
        assert_eq!(run("echo `cargo test`", "", Exit::Ok), []);
    }

    #[test]
    fn redirections_comments_and_case_arms() {
        assert_eq!(run("cargo test &> log", "", Exit::Ok), [Pass(Test)]);
        assert_eq!(run("cargo test |& tail -1", CARGO_OK, Exit::Ok), [Pass(Test)]);
        assert_eq!(run("# cargo test\ncargo build", "", Exit::Ok), [Pass(Build)]);
        assert_eq!(run("case x in a) cargo test;; esac", "", Exit::Ok), [Unknown(Test)]);
    }

    #[test]
    fn prefixes_are_stripped() {
        for cmd in [
            "timeout -k 5 10 cargo test",
            "timeout --foreground 10 cargo test",
            "timeout -s KILL 60 cargo test",
            "nice -n 10 cargo test",
            "sudo -E cargo test",
            "time cargo test",
            "exec cargo test",
            "{ cargo test; }",
            "( cargo test )",
            "cd x && (cargo test)",
        ] {
            let got = run(cmd, CARGO_OK, Exit::Ok);
            assert_eq!(got, [Pass(Test)], "{cmd}");
        }
        assert_eq!(run("if cargo test; then echo ok; fi", CARGO_OK, Exit::Ok), [Pass(Test)]);
        assert_eq!(run("! cargo test 2>&1 | grep -q FAILED", CARGO_OK, Exit::Ok), [Pass(Test)]);
    }

    #[test]
    fn script_names_are_categorised_whole() {
        assert_eq!(run("npm run type-check", "", Exit::Ok), [Pass(Typecheck)]);
        assert_eq!(run("bun run test:e2e", "", Exit::Ok), [Pass(E2e)]);
        assert_eq!(run("make test-fast", "", Exit::Ok), [Pass(Test)]);
        assert_eq!(run("bun run check-links", "", Exit::Ok), []);
        assert_eq!(run("make check-ddl-grants", "", Exit::Ok), []);
        assert_eq!(run("make _e2e-cold-pre", "", Exit::Ok), []);
        assert_eq!(run("make reset-e2e-db", "", Exit::Ok), []);
    }

    #[test]
    fn option_arguments_are_not_names() {
        assert_eq!(run("make -C app test", "", Exit::Ok), [Pass(Test)]);
        assert_eq!(run("make -j 2 test-fast", "", Exit::Ok), [Pass(Test)]);
        assert_eq!(run("bun run --cwd app test:unit", "", Exit::Ok), [Pass(Test)]);
        assert_eq!(run("cargo +nightly -q test", "", Exit::Ok), [Pass(Test)]);
    }

    #[test]
    fn cargo_check_reads_build_markers() {
        assert_eq!(run("cargo check 2>&1 | tail -3", NO_COMPILE, Exit::Ok), [Fail(Build)]);
        assert_eq!(run("cargo check 2>&1 | tail -1", FINISHED, Exit::Ok), [Pass(Build)]);
    }

    #[test]
    fn a_catalogue_toolchain_keeps_its_markers_under_a_folder_command() {
        let typed = [FolderCommand {
            command_line: "cargo check".into(),
            category: Some("typecheck".into()),
        }];
        assert_eq!(
            classify("cargo check 2>&1 | tail -3", NO_COMPILE, Exit::Ok, &seed(), &typed),
            [Fail(Build)]
        );
    }

    #[test]
    fn folder_command_prefix_rules() {
        let cmds = [
            FolderCommand { command_line: "bun run verify".into(), category: Some("build".into()) },
            FolderCommand {
                command_line: "bun run verify:types".into(),
                category: Some("typecheck".into()),
            },
        ];
        let cat = seed();
        assert_eq!(
            classify("bun run verify:types", "", Exit::Ok, &cat, &cmds),
            [Pass(Typecheck)],
            "longest wins"
        );
        assert_eq!(classify("bun run verify2", "", Exit::Ok, &cat, &cmds), [], "word boundary");
    }

    #[test]
    fn markers_per_toolchain() {
        // (command, output, want): one failing and one passing output per marker row.
        let cases: &[(&str, &str, RunSignal)] = &[
            ("go test ./... | tail", "--- FAIL: TestX (0.00s)\nFAIL\tpkg\t0.1s\n", Fail(Test)),
            ("go test ./... | tail", "ok  \tpkg\t0.1s\n", Pass(Test)),
            (
                "go test ./... | grep -c FAIL; echo ok; git status -s",
                "3\nok\n M a.go\n",
                Unknown(Test),
            ),
            ("pytest | tail -1", "===== 1 failed, 5 passed in 0.42s =====\n", Fail(Test)),
            ("pytest | tail -1", "===== 5 passed, 1 error in 0.42s =====\n", Fail(Test)),
            ("pytest -q | tail -1", "1 failed, 5 passed in 0.42s\n", Fail(Test)),
            ("pytest | tail -1", "===== 5 passed, 2 skipped in 0.42s =====\n", Pass(Test)),
            ("pytest -q | tail -1", "5 passed in 0.42s\n", Pass(Test)),
            ("npx jest | tail", "Tests:       1 failed, 3 passed, 4 total\n", Fail(Test)),
            ("npx jest | tail", "Tests:       2 skipped, 5 passed, 7 total\n", Pass(Test)),
            ("npx vitest run | tail -3", "      Tests  40 passed | 2 skipped (42)\n", Pass(Test)),
            ("bun test | tail", " 3 pass\n 1 fail\n", Fail(Test)),
            ("bun test | tail", " 3 pass\n 0 fail\n", Pass(Test)),
            (
                "npx tsc --noEmit | head",
                "src/a.ts(1,1): error TS2304: Cannot find name 'x'.\n",
                Fail(Typecheck),
            ),
            ("npx playwright test | tail", "  1 failed\n  4 passed (3.1s)\n", Fail(E2e)),
            ("npx playwright test | tail", "  4 passed (3.1s)\n", Pass(E2e)),
            (
                "mypy src | tail -1",
                "Found 2 errors in 1 file (checked 3 source files)\n",
                Fail(Typecheck),
            ),
            ("mypy src | tail -1", "Success: no issues found in 3 source files\n", Pass(Typecheck)),
            ("npx pyright | tail -1", "2 errors, 0 warnings, 0 informations\n", Fail(Typecheck)),
            ("npx pyright | tail -1", "0 errors, 0 warnings, 0 informations\n", Pass(Typecheck)),
            ("dotnet build | tail", "Build FAILED.\n", Fail(Build)),
            ("dotnet build | tail", "Build succeeded.\n", Pass(Build)),
            ("dotnet test | tail", "Failed!  - Failed:     1, Passed:     9\n", Fail(Test)),
            ("dotnet test | tail", "Passed!  - Failed:     0, Passed:    10\n", Pass(Test)),
            ("mvn -q package | tail", "[ERROR] COMPILATION ERROR :\n", Fail(Build)),
            ("mvn -q package | tail", "[INFO] BUILD SUCCESS\n", Pass(Build)),
            ("mvn test | tail", "Tests run: 10, Failures: 1, Errors: 0, Skipped: 0\n", Fail(Test)),
            ("mvn test | tail", "Tests run: 10, Failures: 0, Errors: 0, Skipped: 0\n", Pass(Test)),
            (
                "./gradlew build | tail",
                "> Compilation failed; see the compiler error output\n",
                Fail(Build),
            ),
            ("./gradlew assemble | tail", "BUILD SUCCESSFUL in 3s\n", Pass(Build)),
            ("./gradlew test | tail", "10 tests completed, 2 failed\n", Fail(Test)),
            ("swift test | tail", "Executed 10 tests, with 1 failure (0 unexpected)\n", Fail(Test)),
            (
                "swift test | tail",
                "Executed 10 tests, with 0 failures (0 unexpected)\n",
                Pass(Test),
            ),
            (
                "swift build | tail",
                "error: emit-module command failed with exit code 1\n",
                Fail(Build),
            ),
            ("mix test | tail", "10 tests, 1 failure\n", Fail(Test)),
            ("mix test | tail", "10 tests, 0 failures\n", Pass(Test)),
            ("bundle exec rspec | tail", "10 examples, 2 failures\n", Fail(Test)),
            ("bundle exec rspec | tail", "10 examples, 0 failures\n", Pass(Test)),
        ];
        let cat = seed();
        for (cmd, out, want) in cases {
            assert_eq!(
                classify(cmd, out, Exit::Ok, &cat, &[]),
                std::slice::from_ref(want),
                "{cmd} :: {out:?}"
            );
        }
    }

    #[test]
    fn more_seeded_forms() {
        assert_eq!(run("cargo clippy -- -D warnings", "", Exit::Ok), []);
        assert_eq!(run("cargo fmt", "", Exit::Ok), []);
        assert_eq!(run("rake test", "", Exit::Ok), [Pass(Test)]);
        assert_eq!(run("tsc -p .", "", Exit::Ok), [Pass(Typecheck)]);
    }

    #[test]
    fn a_wrapper_claims_only_toolchains_it_can_run() {
        // Rows 4810 and 10275: cargo's summary (from another stage, or grepped
        // from a log) is never a bun script's result, nor a pytest marker.
        let cargo = "test result: ok. 146 passed; 0 failed; 0 ignored; finished in 3.24s\n";
        assert_eq!(
            run(
                "grep '^test result' log.txt | head -1; bun run test:unit >/dev/null 2>&1; echo $?",
                cargo,
                Exit::Ok
            ),
            [Unknown(Test)]
        );
        let both =
            format!("{cargo} Test Files  194 passed (194)\n      Tests  2339 passed (2339)\n");
        assert_eq!(
            run(
                "cargo test -p x 2>&1 | grep result; bun run test:unit 2>&1 | grep Tests",
                &both,
                Exit::Ok
            ),
            [Pass(Test), Pass(Test)]
        );
        assert_eq!(
            run("make test 2>&1 | tail -1", cargo, Exit::Ok),
            [Pass(Test)],
            "make may run anything"
        );
    }

    #[test]
    fn node_test_runner_and_foreign_zero_error_lines() {
        // Row 5424: node's runner passed; "Diagnostics 0 errors" is no pyright result.
        let out = "ℹ tests 13\nℹ pass 13\nℹ fail 0\nDiagnostics   0 errors, 0 warnings\n";
        assert_eq!(
            run(
                "pnpm run test 2>&1 | grep -E 'ℹ'; pnpm run typecheck 2>&1 | tail -5; pnpm run info | head",
                out,
                Exit::Ok
            ),
            [Pass(Test), Unknown(Typecheck)]
        );
        assert_eq!(
            run("pnpm run test 2>&1 | tail -3", "ℹ pass 12\nℹ fail 1\n", Exit::Ok),
            [Fail(Test)]
        );
    }

    #[test]
    fn stage_commands_and_run_categories() {
        assert_eq!(
            stage_commands("cd x && RUST_LOG=1 codex review 2>&1 | tail"),
            ["codex review 2>&1", "tail"]
        );
        assert_eq!(
            run_categories("cargo build && bun run test:unit | tail", &seed(), &[]),
            [Build, Test]
        );
        assert!(run_categories("ls", &seed(), &[]).is_empty());
    }
}
