//! The Rust adapter's compiler step: rust-analyzer's answers, read through
//! [`crate::indexer::lang::compiler`], turned into the identities this indexer
//! mints (docs/plan/2026-10-13-indexer-full-call-placement.md).
//!
//! A library target (the language's own `core`/`std`/`alloc`, or a dependency)
//! is named, never opened: `lib·<crate>·<path>`, the path written the way Rust
//! itself qualifies a name — `option::Option::Some`, `<T as Into<U>>::into`.

use scip::symbol::parse_symbol;
use scip::types::descriptor::Suffix;

use crate::indexer::facts::Fqn;
use crate::indexer::fqn::{self, Form};

/// The `lib·…` identity of a symbol rust-analyzer resolved into a library, or
/// `None` when the symbol is not one this can name (a local, a malformed
/// symbol, or a descriptor shape no Rust path spells).
pub fn library_target(symbol: &str) -> Option<Fqn> {
    let parsed = parse_symbol(symbol).ok()?;
    let package = parsed.package.as_ref()?;
    let krate = package.name.as_str();
    let path = rust_path(&parsed.descriptors)?;
    fqn::refer(&Form::Lib { package: krate, member: &path }).ok()
}

/// Render descriptors as a Rust path. `impl#[Self]` and `impl#[Self][Trait]`
/// become `Self` and `<Self as Trait>`, the forms Rust writes a method of an
/// impl block with.
fn rust_path(descriptors: &[scip::types::Descriptor]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < descriptors.len() {
        let d = &descriptors[i];
        let suffix = d.suffix.enum_value().ok()?;
        match suffix {
            Suffix::Type if d.name == "impl" => {
                let mut params = Vec::new();
                while let Some(next) = descriptors.get(i + 1) {
                    if next.suffix.enum_value().ok()? != Suffix::TypeParameter {
                        break;
                    }
                    params.push(next.name.clone());
                    i += 1;
                }
                match params.as_slice() {
                    [ty] => parts.push(ty.clone()),
                    [ty, tr] => parts.push(format!("<{ty} as {tr}>")),
                    _ => return None,
                }
            }
            Suffix::Namespace | Suffix::Package | Suffix::Type | Suffix::Term | Suffix::Method => {
                parts.push(d.name.clone())
            }
            Suffix::Macro => parts.push(format!("{}!", d.name)),
            // A type parameter outside an impl header, a parameter, a local or
            // meta: no Rust path names these.
            _ => return None,
        }
        i += 1;
    }
    (!parts.is_empty()).then(|| parts.join("::"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(symbol: &str) -> Option<String> {
        library_target(symbol).map(|f| f.to_string())
    }

    const CORE: &str = "rust-analyzer cargo core https://github.com/rust-lang/rust/library/core";
    const STD: &str = "rust-analyzer cargo std https://github.com/rust-lang/rust/library/std";
    const ALLOC: &str = "rust-analyzer cargo alloc https://github.com/rust-lang/rust/library/alloc";

    #[test]
    fn a_free_function_is_its_module_path() {
        assert_eq!(lib(&format!("{STD} env/var().")).as_deref(), Some("lib·std·env::var"));
        assert_eq!(
            lib("rust-analyzer cargo serde_json 1.0.149 de/from_str().").as_deref(),
            Some("lib·serde_json·de::from_str")
        );
    }

    #[test]
    fn a_macro_keeps_its_bang_so_it_never_merges_with_a_function() {
        assert_eq!(
            lib(&format!("{ALLOC} macros/format!")).as_deref(),
            Some("lib·alloc·macros::format!")
        );
        assert_eq!(
            lib(&format!("{CORE} macros/assert_eq!")).as_deref(),
            Some("lib·core·macros::assert_eq!")
        );
    }

    #[test]
    fn a_variant_and_a_trait_method_are_their_type_paths() {
        assert_eq!(
            lib(&format!("{CORE} option/Option#Some#")).as_deref(),
            Some("lib·core·option::Option::Some")
        );
        assert_eq!(
            lib(&format!("{CORE} iter/traits/iterator/Iterator#collect().")).as_deref(),
            Some("lib·core·iter::traits::iterator::Iterator::collect")
        );
    }

    #[test]
    fn an_impl_method_is_qualified_the_way_rust_writes_it() {
        assert_eq!(
            lib(&format!("{CORE} slice/impl#[`[T]`]iter().")).as_deref(),
            Some("lib·core·slice::[T]::iter")
        );
        assert_eq!(
            lib(&format!("{CORE} convert/impl#[T][`Into<U>`]into().")).as_deref(),
            Some("lib·core·convert::<T as Into<U>>::into")
        );
        assert_eq!(
            lib(&format!("{CORE} result/impl#[`Result<T, E>`]unwrap().")).as_deref(),
            Some("lib·core·result::Result<T, E>::unwrap")
        );
    }

    #[test]
    fn a_local_or_a_malformed_symbol_names_nothing() {
        assert_eq!(lib("local 12"), None);
        assert_eq!(lib("not a symbol"), None);
    }
}
