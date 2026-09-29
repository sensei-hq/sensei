//! What a declared return type names — the one piece of rust text-reading the
//! read path needs.
//!
//! Moved here from the retired second producer, which is the whole point: it is
//! rust's grammar, so it belongs beside rust's walk rather than in a parallel
//! tree that mints identities under a different scheme. `resolve_receiver_calls`
//! is its only consumer.

/// Normalise a crate name for comparison — a path spells it with `_`, a
/// manifest with `-`, and both name one crate.
fn norm_crate(s: &str) -> String {
    s.replace('-', "_")
}

/// `&Result<PgStore, E>` correctly named `PgStore`.
fn peel_refs(text: &str) -> &str {
    let mut t = text.trim();
    loop {
        if let Some(r) = t.strip_prefix('&') {
            t = r.trim();
            continue;
        }
        if let Some(r) = t.strip_prefix("mut ") {
            t = r.trim();
            continue;
        }
        if t.starts_with('\'') {
            // Lifetime token (e.g. `'a`) — drop it.
            t = t[1..].trim_start_matches(|c: char| c.is_alphanumeric() || c == '_').trim();
            continue;
        }
        break;
    }
    t
}

/// [`base_type_name`]'s answer with the MODULE PATH still attached, as written
/// (`&crate::a::Widget` → `["crate", "a", "Widget"]`). Never empty.
///
/// The path is what says WHICH `Widget` is meant — measured live, one folder
/// holds twenty nodes named `PgStore` — so the resolver that turns a return type
/// into a node needs it, and the leaf-only readers get it by taking the last
/// segment. One peel, one wrapper list, one validity rule for both.
fn base_type_path(text: &str) -> Option<Vec<String>> {
    let t = peel_refs(text);
    let t = if let Some(r) = t.strip_prefix("dyn ") {
        r.trim()
    } else if let Some(r) = t.strip_prefix("impl ") {
        r.trim()
    } else {
        t
    };
    // Smart-pointer unwrap, matched on the path's LAST SEGMENT. Matching the
    // whole text (`t.strip_prefix("Arc")`) missed every path-qualified wrapper:
    // `std::sync::Arc<PgStore>` reduced to `Arc` and `Arc<tokio::sync::Mutex<T>>`
    // to `Mutex`, so a receiver typed that way named the WRAPPER — a confidently
    // wrong type rather than an unresolved one.
    if let Some(open) = t.find('<')
        && let Some(inner) = t.strip_suffix('>')
    {
        // No peel on `head`: `t` is already peeled above, so nothing here can
        // carry a `&`/`mut`/lifetime prefix.
        let head = t[..open].trim();
        let leaf = head.rsplit("::").next().unwrap_or(head).trim();
        if ["Box", "Rc", "Arc", "RefCell", "Cell", "Mutex", "RwLock"].contains(&leaf) {
            return base_type_path(inner[open + 1..].trim());
        }
    }
    let base = t.split('<').next().unwrap_or(t).trim();
    let mut segs: Vec<&str> = base.split("::").map(str::trim).collect();
    let leaf = segs.pop()?.trim_end_matches(|c: char| !is_ident_char(c));
    // An identifier, whole. Accepting anything that merely STARTS alphabetic let
    // a function-pointer type through as a type name (`fn(u8) -> u8`), which is
    // a string no lookup can mean.
    if !leaf.chars().next().is_some_and(char::is_alphabetic) || !leaf.chars().all(is_ident_char) {
        return None;
    }
    let mut path: Vec<String> =
        segs.into_iter().filter(|s| !s.is_empty()).map(str::to_string).collect();
    path.push(leaf.to_string());
    Some(path)
}

/// What a declared return type names, keeping everything the text said about
/// WHERE the type lives. A bare leaf is not the same answer as a qualified path
/// and the resolver must not treat them alike: reducing
/// `reqwest::blocking::Client` to `Client` turns a dependency's type into a
/// lookup among first-party nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReceiverType {
    /// `Self` — whatever type the returning method is defined on. The resolver
    /// reads that off the node's parent, which is a fact the graph holds and
    /// this text does not.
    SelfType,
    /// A path-qualified name, module chain crate-relative (empty = crate root).
    ///
    /// Always the CURRENT crate's module chain, because `crate::`, the crate's
    /// own name and a local module are the only internal path roots
    /// (`classify_segments` owns that rule). A path rooted at a DEPENDENCY lands
    /// here too and that is deliberate: it names a module the crate does not
    /// have, so the lookup finds nothing, which is the correct answer for a type
    /// with no definition in this graph.
    Qualified { module: String, name: String },
    /// A bare name with no path at all. Says nothing about which type of that
    /// name is meant, so it is usable only where the name is unambiguous.
    Bare(String),
}

/// What a declared RETURN TYPE says the receiver of a chained call is —
/// `ctx.pg().count_edges()` needs `pg`'s return type to know which
/// `count_edges` is meant. `None` when it names no concrete type, which leaves
/// the call unresolved.
///
/// Layered ON TOP of [`base_type_path`] rather than folded into it. That helper
/// has three owners (the IR walk's parameter binding, the fqn producer, and
/// this) and deliberately answers a different question: it strips `dyn `/`impl `
/// and returns the TRAIT name, which `rust_dyn_receiver_stays_unqualified`
/// depends on. Only in RETURN position does an opaque type mean "no receiver".
///
/// The rules this adds:
///
/// * **Opaque and generic types are a miss.** `-> impl Trait`, `-> Box<dyn T>`
///   and a bare parameter `T` name no type a member can be looked up under.
/// * **`Result<T, E>` / `Option<T>` unwrap to `T`.** Not deref-transparent the
///   way `Arc`/`Box` are, so this is a rule about the *chain*: the producer
///   only records a receiver hint for a DIRECT call receiver and never for a
///   `?`/`.await` one, and the member still has to exist on `T` for the lookup
///   to succeed — a `Result` method that `T` does not have simply misses.
/// * **`Self` is deferred, not substituted.** Only the graph knows what type the
///   returning method sits on.
/// * **`self::`/`super::` are a miss.** They are relative to the file's own
///   module, which the resolver reading this cannot recover from an fqn (an
///   empty module segment is dropped, so segment counts are ambiguous). Measured
///   over this repo's 3,932 declared return types, ZERO are written that way, so
///   the honest miss costs nothing that exists.
pub(crate) fn concrete_receiver_type(return_type: &str, package: &str) -> Option<ReceiverType> {
    let t = return_type.trim();
    if t.is_empty() || mentions_opaque_type(t) {
        return None;
    }
    let mut path = base_type_path(strip_fallible(t))?;
    let name = path.pop()?;
    if name == "Self" {
        return Some(ReceiverType::SelfType);
    }
    // A single uppercase letter (optionally numbered) is the universal spelling
    // of a generic parameter. It is a hole in the signature, not a type.
    let mut chars = name.chars();
    if chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_digit()) {
        return None;
    }
    let Some(root) = path.first() else {
        return Some(ReceiverType::Bare(name));
    };
    match root.as_str() {
        "crate" => Some(ReceiverType::Qualified { module: path[1..].join("::"), name }),
        "self" | "super" => None,
        // The crate's own name is `crate::` spelled out, and rustc accepts both.
        r if norm_crate(r) == norm_crate(package) => {
            Some(ReceiverType::Qualified { module: path[1..].join("::"), name })
        }
        _ => Some(ReceiverType::Qualified { module: path.join("::"), name }),
    }
}

/// True when the type expression mentions `impl` or `dyn` as a TOKEN anywhere.
///
/// Checked on the raw text rather than after peeling, because the keyword can
/// sit at any depth (`Result<Box<dyn Store>, E>`) and a peel that reached it
/// would be a second copy of [`base_type_name`]'s loop.
fn mentions_opaque_type(text: &str) -> bool {
    ["impl", "dyn"].iter().any(|kw| {
        text.match_indices(kw).any(|(i, _)| {
            let before_ok = i == 0 || !is_ident_char(text[..i].chars().next_back().unwrap());
            let after_ok =
                text[i + kw.len()..].chars().next().is_none_or(|c| !is_ident_char(c) && c != '<');
            before_ok && after_ok
        })
    })
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Peel `Result<T, E>` / `Option<T>` down to `T`, repeatedly.
///
/// Matches the path's LAST SEGMENT so `anyhow::Result<T>` and `std::io::Result<T>`
/// peel too, and splits the argument list at depth zero so the comma inside
/// `Result<HashMap<String, PgStore>, E>` is not mistaken for the separator.
fn strip_fallible(text: &str) -> &str {
    let mut t = text.trim();
    while let (Some(open), true) = (t.find('<'), t.ends_with('>')) {
        let head = t[..open].trim();
        let leaf = peel_refs(head).rsplit("::").next().unwrap_or(head).trim();
        if leaf != "Result" && leaf != "Option" {
            break;
        }
        t = first_type_arg(t[open + 1..t.len() - 1].trim());
    }
    t
}

/// The first argument of a generic argument list, split at nesting depth zero.
///
/// The `>` of a `->` closes nothing. Counting it as a closer drove the depth
/// negative, so the depth-zero comma after a function-pointer argument was never
/// seen and the whole list came back as one argument
/// (`Result<fn(u8) -> u8, E>` → `fn(u8) -> u8, E`).
fn first_type_arg(args: &str) -> &str {
    let mut depth = 0i32;
    let mut prev = ' ';
    for (i, c) in args.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' if prev == '-' => {}
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => return args[..i].trim(),
            _ => {}
        }
        prev = c;
    }
    args.trim()
}
/// The module a `crate::` / `self::` / `super::` path names, and its leaf.
///
/// `None` when the path has no root marker — it names nothing this arithmetic
/// can place. Each leading `super` consumes one level, so
/// `super::super::executor` climbs twice; consuming only the first would mint
/// `tasks::handlers::super::executor`, a module that never existed.
pub(crate) fn internal_use_module(current_module: &str, segs: &[&str]) -> Option<(String, String)> {
    // AN EMPTY PATH IS `None`, not an empty leaf. Defaulting the missing
    // segment to a blank string would hand a caller a name indistinguishable
    // from one the source actually wrote — the substitution R4 forbids, and
    // what `indexer/`'s own guard caught the moment this code moved under it.
    // (The guard matches on source TEXT, so this comment must not spell the
    // pattern either.)
    let (root, leaf) = match (segs.first(), segs.last()) {
        (Some(root), Some(leaf)) => (*root, (*leaf).to_string()),
        _ => return None,
    };
    // Modules strictly between the root marker and the leaf.
    let mid = |from: usize| -> String {
        if segs.len() <= from + 1 { String::new() } else { segs[from..segs.len() - 1].join("::") }
    };
    let module = match root {
        "crate" => mid(1),
        "self" => join_mod(current_module, &mid(1)),
        "super" => {
            let ups = segs.iter().take_while(|s| **s == "super").count();
            let base = (0..ups).fold(current_module.to_string(), |m, _| parent_mod(&m));
            join_mod(&base, &mid(ups))
        }
        _ => return None,
    };
    Some((module, leaf))
}
/// Join two `::`-path fragments, tolerating either being empty.
fn join_mod(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_string(),
        (_, true) => a.to_string(),
        _ => format!("{a}::{b}"),
    }
}
/// Parent of a `::`-path (`a::b::c` → `a::b`; `a` → "").
fn parent_mod(m: &str) -> String {
    match m.rsplit_once("::") {
        Some((head, _)) => head.to_string(),
        None => String::new(),
    }
}
