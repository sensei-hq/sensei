set search_path to sensei, extensions;

create type node_kind
    as enum (
        'file'
      , 'module', 'package'
      , 'class', 'interface', 'function', 'method'
      , 'property', 'field', 'parameter'
      , 'type', 'const', 'enum', 'enum_variant'
      , 'section'
      , 'rationale'
      , 'struct', 'component', 'hook', 'doc', 'extension'
        -- D12: `lib_symbol` / `lib_package` REMOVED. Kind says WHAT a node
        -- is; the fqn's `lib·` prefix says WHERE it came from. Collapsing the
        -- two destroyed the real kind on 18,240 rows and duplicated a fact the
        -- fqn already carried. External packages are `package`; external
        -- symbols take their real kind, or `unknown` when the use site does
        -- not reveal one.
        -- Stage 0 S5. Appended at the END: dbd diffs enums positionally, and
        -- Postgres has no DROP VALUE, so order here is permanent.
        --
        -- `trait`  — Rust traits. OPEN DECISION, settle before stage 4 emits
        --            either: does Rust `trait` map here, or onto `interface`
        --            (7,131 rows from other languages)? The DDL is the same
        --            either way; the walk's mapping is not.
        -- `static` — `static X: T`, distinct from `const` (4,307 rows).
        -- `macro`  — `macro_rules!` and proc macros; reach `macro` needs a
        --            declaration kind to point at.
        -- `unknown`— a stub minted before its declaration is seen. A NEW value,
        --            not `parameter` reused: `parameter` means something, and a
        --            value that means something else is not a value that means
        --            "not yet known" (§2.1, the withdrawn placeholder).
      , 'trait', 'static', 'macro', 'unknown'
    );
