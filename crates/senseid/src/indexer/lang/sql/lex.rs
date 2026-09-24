//! The SQL tokeniser — batches, comments, quoting, and nothing else.
//!
//! # Why a lexer rather than a grammar
//!
//! Two parsers were measured against this corpus and neither can declare a
//! STORED PROCEDURE, which is the object a T-SQL codebase is mostly made of:
//!
//! - `tree-sitter-sequel` has no `create_procedure` node. Its `grammar.js`
//!   says `// TODO: procedure` in as many words. Measured over 331 files it
//!   recovered a reference in 330 and a declaration in 27, and the names it did
//!   produce were mangled — `dbo].[fnIssues_BSC` — because it does not
//!   understand bracket quoting either.
//! - `sqlparser`'s `MsSqlDialect` fails on `CREATE PROCEDURE @p int AS`, the
//!   parenless parameter form that is the T-SQL norm. 28% of files parsed
//!   whole.
//!
//! So the reader is this crate's own, and it is deliberately a LEXER and a
//! statement-head reader rather than a parser. That is sized to what the
//! corpus is: a T-SQL change script declares one object, names it on one line,
//! and refers to tables by name. The nested scopes, overloads and generics
//! that make a real parser necessary for Rust or C# are not present.
//!
//! ONE ENGINE, and that is a rule rather than a convenience. Wiring
//! "tree-sitter, and the lexer when tree-sitter misses" would make the
//! provenance of any given fact unknowable and turn a loud absence into a
//! silent inconsistency.
//!
//! # `GO` is not SQL
//!
//! It is a client directive — sqlcmd and SSMS split a file on it and send each
//! batch separately. A parser handed a whole file chokes on the first one, so
//! the batches are separated here before anything reads them.

/// One token. Deliberately small: the reader needs to know WHICH WORD is here
/// and where a statement ends, and nothing about expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok<'a> {
    /// A bare identifier or a keyword. Comparison is case-insensitive; the
    /// text is as the source wrote it.
    Word(&'a str),
    /// An identifier the source QUOTED — `[Order Details]`, `"Order"`. Kept
    /// apart from [`Tok::Word`] because a quoted identifier is never a keyword,
    /// however it is spelled.
    Quoted(String),
    /// `@p`, `@@ROWCOUNT`. Never an object name, so the reader skips them
    /// rather than mistaking a parameter for a table.
    Var(&'a str),
    /// A string or numeric literal. The content is not a name, so it is not
    /// kept.
    Literal,
    Punct(char),
}

impl Tok<'_> {
    /// The identifier text, for the two token kinds that carry one.
    pub fn name(&self) -> Option<&str> {
        match self {
            Tok::Word(w) => Some(w),
            Tok::Quoted(q) => Some(q.as_str()),
            _ => None,
        }
    }

    /// Whether this token is the given keyword, ignoring case.
    ///
    /// A QUOTED identifier never matches: `[TABLE]` is a table called `TABLE`,
    /// which is exactly why the source bracketed it.
    pub fn is(&self, keyword: &str) -> bool {
        matches!(self, Tok::Word(w) if w.eq_ignore_ascii_case(keyword))
    }
}

/// Split a file into the batches a client would send separately.
///
/// Returns each batch with the LINE it starts on, because a span has to point
/// at the file the reader is looking at rather than at an offset into a
/// fragment.
pub fn batches(text: &str) -> Vec<(u32, &str)> {
    let mut out = Vec::new();
    let mut start_line = 1u32;
    let mut start = 0usize;
    let mut at = 0usize;
    for (line, raw) in (1u32..).zip(text.split_inclusive('\n')) {
        let trimmed = raw.trim();
        // `GO` may carry a repeat count (`GO 5`) and a trailing comment. What
        // it may never do is appear inside a statement, so a line that BEGINS
        // with it and holds nothing else of substance is the separator.
        // BY CHARS, not by byte slice. `trimmed[..2]` panics the moment a
        // line starts with a multi-byte character, and a corpus exported from
        // SSMS is full of them.
        let mut chars = trimmed.chars();
        let is_go = matches!(chars.next(), Some('g' | 'G'))
            && matches!(chars.next(), Some('o' | 'O'))
            && chars.as_str().trim_start().chars().next().is_none_or(|c| c.is_ascii_digit());
        if is_go {
            if !text[start..at].trim().is_empty() {
                out.push((start_line, &text[start..at]));
            }
            start = at + raw.len();
            start_line = line + 1;
        }
        at += raw.len();
    }
    if !text[start..].trim().is_empty() {
        out.push((start_line, &text[start..]));
    }
    out
}

/// Read a quoted identifier's body, doubling `closer` as the escape.
///
/// SLICED FROM THE SOURCE rather than pushed byte by byte: `b[i] as char`
/// turns every non-ASCII byte into a separate Latin-1 character, so a name with
/// an accent in it comes out mojibake — and this corpus is exported from SSMS,
/// which is exactly where such names live.
fn quoted(sql: &str, i: &mut usize, closer: u8) -> String {
    let b = sql.as_bytes();
    let mut name = String::new();
    let mut start = *i;
    while *i < b.len() {
        if b[*i] == closer {
            name.push_str(&sql[start..*i]);
            if b.get(*i + 1) == Some(&closer) {
                name.push(closer as char);
                *i += 2;
                start = *i;
                continue;
            }
            *i += 1;
            return name;
        }
        *i += 1;
    }
    // UNTERMINATED. The name is what was read, which is the honest answer for
    // a truncated file — and the loop has consumed to the end, so the caller
    // cannot spin.
    name.push_str(&sql[start..]);
    name
}

/// Tokenise one batch.
///
/// Comments and literals are CONSUMED rather than emitted: a table name inside
/// a comment is not a reference, and the corpus is full of commented-out SQL.
pub fn tokens(sql: &str) -> Vec<Tok<'_>> {
    let b = sql.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        match c {
            _ if c.is_ascii_whitespace() => i += 1,
            // `-- line comment`
            b'-' if b.get(i + 1) == Some(&b'-') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            // `/* block */`, which T-SQL allows to NEST.
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut depth = 1usize;
                i += 2;
                while i < b.len() && depth > 0 {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            // `'literal'`, with `''` as the escape. `N'…'` is the same thing
            // with a unicode prefix, and the prefix lexes as a word first.
            b'\'' => {
                i += 1;
                while i < b.len() {
                    if b[i] == b'\'' {
                        if b.get(i + 1) == Some(&b'\'') {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                out.push(Tok::Literal);
            }
            // `[Order Details]`, with `]]` as the escape.
            b'[' => {
                i += 1;
                out.push(Tok::Quoted(quoted(sql, &mut i, b']')));
            }
            // `"Order"` under QUOTED_IDENTIFIER ON, which is the default.
            b'"' => {
                i += 1;
                out.push(Tok::Quoted(quoted(sql, &mut i, b'"')));
            }
            // `@p`, `@@ROWCOUNT`, and `#temp` / `##global` — none of them a
            // name this reader records, but all of them must be consumed whole
            // so their tail is not read as a bare identifier.
            b'@' | b'#' => {
                let start = i;
                i += 1;
                while i < b.len() && (b[i] == b'@' || b[i] == b'#') {
                    i += 1;
                }
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                out.push(Tok::Var(&sql[start..i]));
            }
            _ if c.is_ascii_digit() => {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'.') {
                    i += 1;
                }
                out.push(Tok::Literal);
            }
            _ if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$')
                {
                    i += 1;
                }
                out.push(Tok::Word(&sql[start..i]));
            }
            _ => {
                out.push(Tok::Punct(c as char));
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(sql: &str) -> Vec<String> {
        tokens(sql).iter().filter_map(|t| t.name().map(str::to_string)).collect()
    }

    /// **A COMMENT IS NOT SQL**, and this corpus is full of commented-out
    /// statements.
    ///
    /// MUTATION: stop consuming `--` or `/* */` — every table named in a
    /// commented-out query becomes a reference the file does not make.
    #[test]
    fn comments_are_consumed_including_nested_blocks() {
        assert_eq!(words("SELECT a -- FROM Secret\nFROM Real"), ["SELECT", "a", "FROM", "Real"]);
        assert_eq!(words("/* FROM Secret */ FROM Real"), ["FROM", "Real"]);
        // T-SQL NESTS block comments, so the first `*/` does not end the outer
        // one. Reading it as the end resumes lexing inside a comment.
        assert_eq!(words("/* a /* FROM Secret */ b */ FROM Real"), ["FROM", "Real"]);
    }

    /// A literal holds no names.
    #[test]
    fn a_string_literal_is_consumed_whole_escapes_and_all() {
        assert_eq!(words("WHERE x = 'FROM Secret'"), ["WHERE", "x"]);
        assert_eq!(words("WHERE x = 'it''s FROM Secret' AND y = 1"), ["WHERE", "x", "AND", "y"]);
        // `N'…'` is a unicode literal; the `N` lexes as a word of its own,
        // which the reader skips because it is never in a name position.
        assert_eq!(words("VALUES (N'FROM Secret')"), ["VALUES", "N"]);
    }

    /// **BRACKET QUOTING IS T-SQL'S, AND GETTING IT WRONG MANGLES EVERY NAME.**
    ///
    /// tree-sitter-sequel produced `dbo].[fnIssues_BSC` for `[dbo].[fnIssues_BSC]`
    /// — a name no use site can mint.
    ///
    /// MUTATION: lex `[` as punctuation — the brackets leak into the name.
    #[test]
    fn a_quoted_identifier_keeps_its_name_and_loses_its_quotes() {
        assert_eq!(words("[dbo].[sp_X]"), ["dbo", "sp_X"]);
        assert_eq!(words("\"dbo\".\"sp_X\""), ["dbo", "sp_X"]);
        // A SPACE in a name is the whole reason brackets exist.
        assert_eq!(words("[Order Details]"), ["Order Details"]);
        // `]]` is the escape for a literal `]`.
        assert_eq!(words("[we]]ird]"), ["we]ird"]);
        // NON-ASCII survives. Pushing bytes as chars turned every accented
        // name into mojibake, and an SSMS-exported corpus is full of them.
        assert_eq!(words("[Tabla_Año]"), ["Tabla_Año"]);
        // An UNTERMINATED quote yields what was read rather than spinning.
        assert_eq!(words("[truncated"), ["truncated"]);
    }

    /// A quoted identifier is never a keyword, however it is spelled.
    ///
    /// MUTATION: have `is()` accept `Tok::Quoted` — a table called `[TABLE]`
    /// starts being read as the `TABLE` keyword.
    #[test]
    fn a_quoted_identifier_is_never_a_keyword() {
        let t = tokens("[TABLE]");
        assert_eq!(t.len(), 1);
        assert!(!t[0].is("TABLE"), "the source bracketed it precisely so it is a name");
        assert_eq!(t[0].name(), Some("TABLE"));
        assert!(tokens("TABLE")[0].is("table"), "an unquoted keyword matches, ignoring case");
    }

    /// A parameter is not a name.
    ///
    /// MUTATION: lex `@` as punctuation — `@Issues` yields a bare `Issues`
    /// that reads as a table reference.
    #[test]
    fn a_variable_is_consumed_whole_so_its_tail_is_not_a_name() {
        assert_eq!(words("SELECT @Issues, @@ROWCOUNT FROM Real"), ["SELECT", "FROM", "Real"]);
        // A `#temp` table is local to the batch and is deliberately not a name
        // this reader records.
        assert_eq!(words("INSERT INTO #tmp SELECT 1"), ["INSERT", "INTO", "SELECT"]);
    }

    /// **`GO` IS A CLIENT DIRECTIVE**, not SQL — it separates batches.
    ///
    /// MUTATION: stop splitting on it — a parser sees `GO` mid-file and every
    /// statement after the first is lost.
    #[test]
    fn go_separates_batches_and_each_one_knows_its_line() {
        let bs = batches("CREATE TABLE a (x int)\nGO\nCREATE TABLE b (y int)\n");
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0].0, 1);
        assert!(bs[0].1.contains("TABLE a"));
        assert_eq!(bs[1].0, 3, "the second batch starts on line 3");
        assert!(bs[1].1.contains("TABLE b"));
    }

    /// `GO` takes an optional repeat count, and a file need not end with one.
    #[test]
    fn a_batch_separator_tolerates_a_repeat_count_and_a_missing_trailer() {
        assert_eq!(batches("SELECT 1\nGO 5\nSELECT 2").len(), 2);
        assert_eq!(batches("SELECT 1").len(), 1, "a file with no GO is one batch");
        assert_eq!(batches("\n\n  \n").len(), 0, "an empty file has no batches");
        // `GOTO` is not `GO`, and neither is a column called `go`.
        assert_eq!(batches("SELECT go FROM t").len(), 1);
        assert_eq!(batches("GOTO done").len(), 1);
    }
}
