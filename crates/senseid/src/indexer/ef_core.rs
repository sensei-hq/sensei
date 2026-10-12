//! EF Core, derived (spec 20 §7c, #251): the database a `DbContext` maps,
//! read off what the C# walks already wrote — classes, their properties and
//! declared types, ownership, base types, and call sites.
//!
//! A DERIVATION, not a walk. The table a `DbSet<Order>` maps is renamed by a
//! `.ToTable("orders")` that usually sits in another file, and its columns are
//! another class's properties. One file cannot see either, so this runs per
//! repository after the file tasks — the place the dbd import runs.
//!
//! What this module claims and what it does not:
//! - A name the source STATES (`ToTable`, `HasColumnName`, `HasDefaultSchema`)
//!   wins, and the table is `naming = stated`. Otherwise EF's own convention —
//!   the `DbSet` property's name, else the class's — and `naming = convention`.
//! - A schema nothing states and no provider implies is not guessed: the entity
//!   is SKIPPED and named.
//! - `not_null` only where the CLR type says so (a non-nullable value type, or
//!   a key). A reference type's nullability is a project setting no file
//!   states, so it is left false.

use std::collections::{BTreeMap, HashMap};

use super::facts::{
    CallArg, DeclaredType, Naming, Receiver, SchemaColumn, SchemaFact, SchemaKind, SchemaRef,
    SchemaSource, Span, SymbolKind,
};
use super::persist::{BatchScope, CallSiteRow, SymbolRow};

/// The database provider a context is built for, read off its project's
/// package references. It decides the schema EF uses when none is stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    SqlServer,
    Npgsql,
}

impl Provider {
    pub fn default_schema(self) -> &'static str {
        match self {
            Provider::SqlServer => "dbo",
            Provider::Npgsql => "public",
        }
    }
}

/// What the derivation reads. Built from the store in production and from
/// walker output in tests — the same row types either way.
#[derive(Debug, Clone)]
pub struct Input {
    pub symbols: Vec<SymbolRow>,
    /// `(member fqn, owning type fqn)` — a property's or method's class.
    pub owners: Vec<(String, String)>,
    /// `(type fqn, base name as written, without type arguments)`.
    pub bases: Vec<(String, String)>,
    pub calls: Vec<CallSiteRow>,
    /// `(context fqn, provider)`; a context with none here has no known provider.
    pub providers: Vec<(String, Provider)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// One batch per package the contexts live in — the scope `write_schema`
    /// replaces.
    pub batches: Vec<(BatchScope, Vec<SchemaFact>)>,
    pub skipped: Vec<Skipped>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub entity: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// No `ToTable` schema, no `HasDefaultSchema`, and no known provider.
    NoSchema,
    /// The entity's class is not declared in this repository.
    NoClass,
    /// More than one class of that name, and none in the context's package.
    AmbiguousClass,
    /// Configured (`EntityTypeBuilder<T>`), but no single context claims it.
    NoContext,
}

/// The package names that state a provider, as NuGet spells them.
const PROVIDERS: &[(&str, Provider)] = &[
    ("Microsoft.EntityFrameworkCore.SqlServer", Provider::SqlServer),
    ("Npgsql.EntityFrameworkCore.PostgreSQL", Provider::Npgsql),
];

/// Read the derivation's input for one repository out of the store.
pub async fn load(
    store: &crate::db::pg_store::PgStore,
    folder_id: &uuid::Uuid,
    repo_abs: &str,
) -> Result<Input, String> {
    let language = super::facts::Language::CSharp;
    let symbols: Vec<SymbolRow> = super::persist::symbols_of(store, folder_id)
        .await?
        .into_iter()
        .filter(|s| s.language == language)
        .collect();
    let owners = store.member_owners(folder_id, language.as_str()).await?;
    let bases = store.type_bases(folder_id, language.as_str()).await?;
    let calls = super::persist::call_sites_of(store, folder_id).await?;
    let names: Vec<&str> = PROVIDERS.iter().map(|(n, _)| *n).collect();
    let referenced = store.libraries_under(repo_abs, &names).await?;

    // A type's provider is the one its NEAREST referencing folder states — the
    // project holding it. Two providers in that folder is no answer.
    let mut providers = Vec::new();
    for (fqn, _) in &bases {
        let Some(symbol) = symbols.iter().find(|s| &s.fqn == fqn) else { continue };
        let file = std::path::Path::new(repo_abs).join(&symbol.file_path);
        let nearest = referenced
            .iter()
            .filter(|(folder, _)| file.starts_with(folder))
            .map(|(folder, _)| folder.len())
            .max();
        let Some(depth) = nearest else { continue };
        let stated: Vec<Provider> = referenced
            .iter()
            .filter(|(folder, _)| folder.len() == depth && file.starts_with(folder))
            .filter_map(|(_, name)| PROVIDERS.iter().find(|(n, _)| n == name).map(|(_, p)| *p))
            .collect();
        if let [one] = stated.as_slice()
            && !providers.iter().any(|(f, _)| f == fqn)
        {
            providers.push((fqn.clone(), *one));
        }
    }
    Ok(Input { symbols, owners, bases, calls, providers })
}

pub fn derive(input: &Input) -> Result<Derived, String> {
    let ix = Ix::new(input);
    let contexts = ix.contexts();
    if contexts.is_empty() {
        return Ok(Derived { batches: Vec::new(), skipped: Vec::new() });
    }
    let config = ix.configuration(&contexts);

    // Which entities each context maps, in the order the source names them:
    // its DbSets, then `Entity<T>()` calls in its own methods.
    let mut claims: Vec<(&SymbolRow, Vec<Claim>)> = Vec::new();
    for &context in &contexts {
        let mut entities: Vec<Claim> = Vec::new();
        for member in ix.members(&context.fqn) {
            if let DeclaredType::Stated(ty) = &member.declared_type
                && let Some(t) = generic_arg(ty, "DbSet")
                && !entities.iter().any(|(e, _)| *e == t)
            {
                entities.push((t, Some(member.name.clone())));
            }
        }
        for t in config.entity_calls.get(context.fqn.as_str()).into_iter().flatten() {
            if !entities.iter().any(|(e, _)| e == t) {
                entities.push((t.clone(), None));
            }
        }
        claims.push((context, entities));
    }

    // A CONFIGURED-only entity (an `EntityTypeBuilder<T>` with no DbSet and no
    // `Entity<T>()`) joins the repository's context when there is exactly one;
    // with several, nothing in the source says which, so it is skipped.
    let mut skipped = Vec::new();
    for t in &config.configured {
        if claims.iter().any(|(_, es)| es.iter().any(|(e, _)| e == t)) {
            continue;
        }
        match claims.as_mut_slice() {
            [(_, only)] => only.push((t.clone(), None)),
            _ => skipped.push(Skipped { entity: t.clone(), reason: SkipReason::NoContext }),
        }
    }

    // PASS 1 — where each entity lives: its class, table, schema and key.
    let mut placed: Vec<Placed> = Vec::new();
    for (context, entities) in &claims {
        let package = package_of(&context.fqn)?;
        for (entity, dbset) in entities {
            let class = match ix.class_named(entity, &package) {
                Ok(c) => c,
                Err(reason) => {
                    skipped.push(Skipped { entity: entity.clone(), reason });
                    continue;
                }
            };
            let (table, naming) = match config.table.get(entity.as_str()) {
                Some(t) => (t.clone(), Naming::Stated),
                None => (dbset.clone().unwrap_or_else(|| entity.clone()), Naming::Convention),
            };
            let schema = config
                .schema
                .get(entity.as_str())
                .cloned()
                .or_else(|| config.default_schema.get(context.fqn.as_str()).cloned())
                .or_else(|| ix.provider(&context.fqn).map(|p| p.default_schema().to_string()));
            let Some(schema) = schema else {
                skipped.push(Skipped { entity: entity.clone(), reason: SkipReason::NoSchema });
                continue;
            };
            let fqn = super::lang::sql::facts::object_identity(&package, &schema, &table)
                .map_err(|e| format!("ef core: {schema}.{table}: {e:?}"))?;
            let properties = ix.properties(&class.fqn);
            let key = match config.key.get(entity.as_str()) {
                Some(stated) => stated.clone(),
                None => properties
                    .iter()
                    .map(|(name, _)| name)
                    .find(|n| {
                        n.eq_ignore_ascii_case("Id")
                            || n.eq_ignore_ascii_case(&format!("{entity}Id"))
                    })
                    .cloned(),
            };
            placed.push(Placed {
                entity: entity.clone(),
                package: package.clone(),
                schema,
                naming,
                fqn,
                properties,
                key,
            });
        }
    }

    // PASS 2 — columns and keys, now that every entity's table is known.
    let names: Vec<&str> = placed.iter().map(|p| p.entity.as_str()).collect();
    let mut batches: BTreeMap<String, Vec<SchemaFact>> = BTreeMap::new();
    for p in &placed {
        let column_name = |entity: &str, property: &str| {
            config
                .rename
                .get(&(entity.to_string(), property.to_string()))
                .cloned()
                .unwrap_or_else(|| property.to_string())
        };
        // Reference navigations: a property typed as another entity.
        let navigations: Vec<(&str, &str)> = p
            .properties
            .iter()
            .filter_map(|(name, ty)| {
                let base = ty.trim_end_matches('?');
                names.contains(&base).then_some((name.as_str(), base))
            })
            .collect();
        let mut refs = Vec::new();
        let mut foreign: Vec<&str> = Vec::new();
        for (navigation, target) in &navigations {
            let Some(other) = placed.iter().find(|o| o.entity == *target) else { continue };
            let Some(other_key) = &other.key else { continue };
            // EF's convention: the navigation's name, or the principal type's,
            // followed by the principal key's name.
            let candidates = [format!("{navigation}{other_key}"), format!("{target}{other_key}")];
            let Some((fk, _)) = p
                .properties
                .iter()
                .find(|(n, _)| candidates.iter().any(|c| c.eq_ignore_ascii_case(n)))
            else {
                continue;
            };
            foreign.push(fk.as_str());
            refs.push(SchemaRef {
                from_column: column_name(&p.entity, fk),
                to: other.fqn.clone(),
                to_column: column_name(&other.entity, other_key),
                on_delete: None,
            });
        }
        let columns = p
            .properties
            .iter()
            .filter(|(name, ty)| {
                !navigations.iter().any(|(n, _)| n == name) && !is_collection_of(ty, &names)
            })
            .map(|(name, ty)| {
                let primary_key = p.key.as_deref() == Some(name.as_str());
                SchemaColumn {
                    name: column_name(&p.entity, name),
                    sql_type: ty.clone(),
                    primary_key,
                    not_null: primary_key || is_value_type(ty),
                    unique_key: false,
                    foreign_key: foreign.contains(&name.as_str()),
                    enum_typed: false,
                    default_expr: None,
                    note: None,
                }
            })
            .collect();
        let facts = batches.entry(p.package.clone()).or_default();
        if facts.iter().any(|f| f.fqn == p.fqn) {
            continue;
        }
        facts.push(SchemaFact {
            fqn: p.fqn.clone(),
            kind: SchemaKind::Table,
            db_schema: p.schema.clone(),
            source: SchemaSource::EfCore,
            naming: p.naming,
            note: None,
            note_md: None,
            columns,
            indexes: Vec::new(),
            refs,
        });
    }

    Ok(Derived {
        batches: batches
            .into_iter()
            .map(|(package, facts)| (BatchScope { source: SchemaSource::EfCore, package }, facts))
            .collect(),
        skipped,
    })
}

/// An entity a context maps: its class's name, and the `DbSet` property that
/// names it when one does.
type Claim = (String, Option<String>);

/// One entity, placed: where its table lives and which property is its key.
struct Placed {
    entity: String,
    package: String,
    schema: String,
    naming: Naming,
    fqn: super::facts::Fqn,
    properties: Vec<(String, String)>,
    key: Option<String>,
}

/// What the call sites configure, per entity.
#[derive(Debug)]
struct Configuration {
    table: HashMap<String, String>,
    schema: HashMap<String, String>,
    /// `Some(p)` is a stated single key; `None` stated a key no single member
    /// names (a composite) — no column is marked, and no convention is applied.
    key: HashMap<String, Option<String>>,
    rename: HashMap<(String, String), String>,
    /// Context fqn → its `HasDefaultSchema`.
    default_schema: HashMap<String, String>,
    /// Context fqn → the `Entity<T>()` its own methods name, in order.
    entity_calls: HashMap<String, Vec<String>>,
    /// Every entity some call configures, in first-seen order.
    configured: Vec<String>,
}

/// Lookups over the input.
struct Ix<'i> {
    input: &'i Input,
    by_fqn: HashMap<&'i str, &'i SymbolRow>,
    owner: HashMap<&'i str, &'i str>,
    calls_at: HashMap<(&'i str, Span), &'i CallSiteRow>,
}

impl<'i> Ix<'i> {
    fn new(input: &'i Input) -> Self {
        Self {
            input,
            by_fqn: input.symbols.iter().map(|s| (s.fqn.as_str(), s)).collect(),
            owner: input.owners.iter().map(|(c, o)| (c.as_str(), o.as_str())).collect(),
            calls_at: input.calls.iter().map(|c| ((c.file_path.as_str(), c.at), c)).collect(),
        }
    }

    fn is_type(kind: SymbolKind) -> bool {
        matches!(kind, SymbolKind::Class | SymbolKind::Struct)
    }

    /// Classes deriving from `…DbContext`, directly or through a class of this
    /// repository that does.
    fn contexts(&self) -> Vec<&'i SymbolRow> {
        let bases_of = |fqn: &str| -> Vec<&'i str> {
            self.input.bases.iter().filter(|(c, _)| c == fqn).map(|(_, b)| b.as_str()).collect()
        };
        let mut out: Vec<&'i SymbolRow> = Vec::new();
        let types: Vec<&'i SymbolRow> =
            self.input.symbols.iter().filter(|s| Self::is_type(s.kind)).collect();
        // A fixed point over the base chain: a context's subclass is a context.
        loop {
            let before = out.len();
            for t in &types {
                if out.iter().any(|o| o.fqn == t.fqn) {
                    continue;
                }
                let is_context = bases_of(&t.fqn).iter().any(|b| {
                    let name = bare(b);
                    name.ends_with("DbContext") || out.iter().any(|o| o.name == name)
                });
                if is_context {
                    out.push(t);
                }
            }
            if out.len() == before {
                break;
            }
        }
        out.sort_by(|a, b| {
            (a.file_path.as_str(), a.span.start_line)
                .cmp(&(b.file_path.as_str(), b.span.start_line))
        });
        out
    }

    fn members(&self, type_fqn: &str) -> Vec<&'i SymbolRow> {
        let mut out: Vec<&'i SymbolRow> = self
            .input
            .owners
            .iter()
            .filter(|(_, o)| o == type_fqn)
            .filter_map(|(c, _)| self.by_fqn.get(c.as_str()).copied())
            .collect();
        out.sort_by_key(|s| (s.span.start_line, s.span.start_col));
        out
    }

    /// `(name, declared type)` of a type's properties, in declaration order.
    fn properties(&self, type_fqn: &str) -> Vec<(String, String)> {
        self.members(type_fqn)
            .into_iter()
            .filter(|m| m.kind == SymbolKind::Property)
            .filter_map(|m| match &m.declared_type {
                DeclaredType::Stated(ty) => Some((m.name.clone(), ty.clone())),
                DeclaredType::Unstated => None,
            })
            .collect()
    }

    fn class_named(&self, name: &str, package: &str) -> Result<&'i SymbolRow, SkipReason> {
        let found: Vec<&'i SymbolRow> =
            self.input.symbols.iter().filter(|s| Self::is_type(s.kind) && s.name == name).collect();
        match found.as_slice() {
            [] => Err(SkipReason::NoClass),
            [one] => Ok(one),
            many => {
                let local: Vec<&&SymbolRow> = many
                    .iter()
                    .filter(|s| package_of(&s.fqn).ok().as_deref() == Some(package))
                    .collect();
                match local.as_slice() {
                    [one] => Ok(one),
                    _ => Err(SkipReason::AmbiguousClass),
                }
            }
        }
    }

    fn provider(&self, context: &str) -> Option<Provider> {
        self.input.providers.iter().find(|(c, _)| c == context).map(|(_, p)| *p)
    }

    /// The type of a parameter of the declaration a call sits in.
    fn param_type(&self, from: &str, name: &str) -> Option<&'i str> {
        self.by_fqn.get(from)?.params.iter().find(|p| p.name == name).and_then(|p| {
            match &p.declared_type {
                DeclaredType::Stated(ty) => Some(ty.as_str()),
                DeclaredType::Unstated => None,
            }
        })
    }

    /// The call whose lambda argument binds `name` around `call` — the
    /// innermost, by span containment, in the same file.
    fn lambda_binder(&self, call: &CallSiteRow, name: &str) -> Option<&'i CallSiteRow> {
        self.input
            .calls
            .iter()
            .filter(|c| c.file_path == call.file_path && c.at != call.at)
            .filter_map(|c| {
                c.args.iter().find_map(|a| match a {
                    CallArg::Lambda { params, at, .. }
                        if params.iter().any(|p| p == name) && contains(*at, call.at) =>
                    {
                        Some((c, *at))
                    }
                    _ => None,
                })
            })
            .min_by_key(|(_, at)| {
                (at.end_line - at.start_line, at.end_col.wrapping_sub(at.start_col))
            })
            .map(|(c, _)| c)
    }

    /// The entity a call configures, followed back through its receiver.
    fn entity_of(&self, call: &CallSiteRow, depth: usize) -> Option<String> {
        if depth > 32 {
            return None;
        }
        if call.callee == "Entity" && call.type_args.len() == 1 {
            return Some(bare(&call.type_args[0]).to_string());
        }
        match &call.receiver {
            Receiver::Call(at) => {
                let previous = self.calls_at.get(&(call.file_path.as_str(), *at))?;
                self.entity_of(previous, depth + 1)
            }
            Receiver::Name(name) => match self.lambda_binder(call, name) {
                Some(binder) => self.entity_of(binder, depth + 1),
                None => generic_arg(self.param_type(&call.from, name)?, "EntityTypeBuilder"),
            },
            Receiver::None | Receiver::Other => None,
        }
    }

    fn configuration(&self, contexts: &[&SymbolRow]) -> Configuration {
        let mut c = Configuration {
            table: HashMap::new(),
            schema: HashMap::new(),
            key: HashMap::new(),
            rename: HashMap::new(),
            default_schema: HashMap::new(),
            entity_calls: HashMap::new(),
            configured: Vec::new(),
        };
        let str_arg = |call: &CallSiteRow, i: usize| match call.args.get(i) {
            Some(CallArg::Str(s)) => Some(s.clone()),
            _ => None,
        };
        let owning_context = |call: &CallSiteRow| -> Option<&str> {
            let owner = *self.owner.get(call.from.as_str())?;
            contexts.iter().any(|ctx| ctx.fqn == owner).then_some(owner)
        };
        for call in &self.input.calls {
            if call.callee == "HasDefaultSchema"
                && let (Some(ctx), Some(schema)) = (owning_context(call), str_arg(call, 0))
            {
                c.default_schema.insert(ctx.to_string(), schema);
                continue;
            }
            let Some(entity) = self.entity_of(call, 0) else { continue };
            if !c.configured.contains(&entity) {
                c.configured.push(entity.clone());
            }
            match call.callee.as_str() {
                "Entity" => {
                    if let Some(ctx) = owning_context(call) {
                        let calls = c.entity_calls.entry(ctx.to_string()).or_default();
                        if !calls.contains(&entity) {
                            calls.push(entity);
                        }
                    }
                }
                "ToTable" => {
                    if let Some(table) = str_arg(call, 0) {
                        c.table.insert(entity.clone(), table);
                    }
                    if let Some(schema) = str_arg(call, 1) {
                        c.schema.insert(entity, schema);
                    }
                }
                "HasKey" => {
                    let key = match call.args.first() {
                        Some(CallArg::Lambda { member, .. }) => member.clone(),
                        Some(CallArg::Str(s)) => Some(s.clone()),
                        _ => None,
                    };
                    c.key.insert(entity, key);
                }
                "HasColumnName" => {
                    let Some(column) = str_arg(call, 0) else { continue };
                    let Receiver::Call(at) = &call.receiver else { continue };
                    let Some(property) = self.calls_at.get(&(call.file_path.as_str(), *at)) else {
                        continue;
                    };
                    let member = match property.args.first() {
                        Some(CallArg::Lambda { member: Some(m), .. }) => m.clone(),
                        Some(CallArg::Str(s)) => s.clone(),
                        _ => continue,
                    };
                    if property.callee == "Property" {
                        c.rename.insert((entity, member), column);
                    }
                }
                _ => {}
            }
        }
        c
    }
}

/// `Shop.Order` → `Order`; `DbSet<Order>` → `DbSet`.
fn bare(name: &str) -> &str {
    let head = name.split('<').next().unwrap_or(name).trim();
    head.rsplit('.').next().unwrap_or(head)
}

/// The single type argument of `Outer<T>` when the outer type's bare name is
/// `outer`.
fn generic_arg(ty: &str, outer: &str) -> Option<String> {
    let ty = ty.trim().trim_end_matches('?');
    let (head, rest) = ty.split_once('<')?;
    let inner = rest.strip_suffix('>')?;
    (bare(head) == outer && !inner.contains(',')).then(|| bare(inner).to_string())
}

/// A collection of a mapped entity — a collection navigation, never a column.
fn is_collection_of(ty: &str, entities: &[&str]) -> bool {
    const COLLECTIONS: &[&str] = &[
        "ICollection",
        "IList",
        "List",
        "IEnumerable",
        "HashSet",
        "ISet",
        "IReadOnlyCollection",
        "IReadOnlyList",
        "Collection",
    ];
    COLLECTIONS.iter().any(|c| generic_arg(ty, c).is_some_and(|t| entities.contains(&t.as_str())))
}

/// A CLR type no row can hold NULL in: a non-nullable value type.
fn is_value_type(ty: &str) -> bool {
    const VALUES: &[&str] = &[
        "int",
        "long",
        "short",
        "byte",
        "sbyte",
        "uint",
        "ulong",
        "ushort",
        "bool",
        "decimal",
        "double",
        "float",
        "char",
        "DateTime",
        "DateTimeOffset",
        "DateOnly",
        "TimeOnly",
        "TimeSpan",
        "Guid",
        "Int16",
        "Int32",
        "Int64",
        "Boolean",
        "Decimal",
        "Double",
        "Single",
    ];
    !ty.trim().ends_with('?') && VALUES.contains(&bare(ty))
}

fn contains(outer: Span, inner: Span) -> bool {
    (outer.start_line, outer.start_col) <= (inner.start_line, inner.start_col)
        && (inner.end_line, inner.end_col) <= (outer.end_line, outer.end_col)
}

fn package_of(fqn: &str) -> Result<String, String> {
    super::fqn::parse(fqn)
        .map(|p| p.package.to_string())
        .map_err(|e| format!("ef core: {fqn}: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::{FileFacts, Language, RelationKind, Resolution};
    use crate::indexer::lang::sql::facts::object_identity;
    use crate::indexer::lang::{Source, TypeHomes};

    const CONTEXT: &str = r#"namespace Shop;
public class ShopContext : DbContext {
    public DbSet<Order> Orders { get; set; }
    public DbSet<Customer> Customers { get; set; }
    public DbSet<Note> Notes { get; set; }
    protected override void OnModelCreating(ModelBuilder modelBuilder) {
        modelBuilder.HasDefaultSchema("sales");
        modelBuilder.Entity<Order>().ToTable("orders", "ops");
        modelBuilder.Entity<Customer>(b => { b.HasKey(c => c.Code); });
    }
}
"#;

    const ENTITIES: &str = r#"namespace Shop;
public class Order {
    public int Id { get; set; }
    public string? Reference { get; set; }
    public Customer Customer { get; set; }
    public string CustomerCode { get; set; }
    public ICollection<Line> Lines { get; set; }
    public decimal Total { get; set; }
}
public class Customer { public string Code { get; set; } public string Name { get; set; } }
public class Line {
    public int LineId { get; set; }
    public int Qty { get; set; }
    public Order Order { get; set; }
    public int OrderId { get; set; }
}
public class Note { public int Id { get; set; } }
"#;

    const CONFIG: &str = r#"namespace Shop;
public class LineConfig : IEntityTypeConfiguration<Line> {
    public void Configure(EntityTypeBuilder<Line> builder) {
        builder.ToTable("order_lines");
        builder.Property(l => l.Qty).HasColumnName("quantity");
    }
}
"#;

    fn walk(path: &str, text: &str) -> FileFacts {
        let source = Source { package: "Shop", module: "", path, text };
        crate::indexer::lang::adapter_for(Language::CSharp)
            .read(&source, &TypeHomes::unknown())
            .expect("the fixture parses")
    }

    /// The input as the store would hand it over, built from walker output with
    /// the store's own row types.
    fn input_of(files: &[FileFacts], provider: Option<Provider>) -> Input {
        let mut input = Input {
            symbols: vec![],
            owners: vec![],
            bases: vec![],
            calls: vec![],
            providers: vec![],
        };
        for f in files {
            input.symbols.extend(f.symbols.iter().map(|s| SymbolRow::of(s, &f.path, f.language)));
            input.calls.extend(f.calls.iter().map(|c| CallSiteRow::of(c, &f.path)));
            for r in &f.relations {
                match (r.kind, &r.parent) {
                    (RelationKind::Owns, Resolution::Resolved { fqn, .. }) => {
                        input.owners.push((r.child.as_str().into(), fqn.as_str().into()))
                    }
                    (
                        RelationKind::Implements | RelationKind::Extends,
                        Resolution::Unresolved { evidence, .. },
                    ) => input.bases.push((r.child.as_str().into(), evidence.name.clone())),
                    _ => {}
                }
            }
        }
        if let Some(p) = provider {
            let context = input
                .symbols
                .iter()
                // The CLASS — the file's module carries the same name.
                .find(|s| s.name == "ShopContext" && s.kind == SymbolKind::Class)
                .map(|s| s.fqn.clone())
                .expect("the context is declared");
            input.providers.push((context, p));
        }
        input
    }

    fn shop(provider: Option<Provider>) -> Derived {
        let files = [
            walk("src/ShopContext.cs", CONTEXT),
            walk("src/Entities.cs", ENTITIES),
            walk("src/LineConfig.cs", CONFIG),
        ];
        derive(&input_of(&files, provider)).expect("the shop derives")
    }

    fn table<'d>(d: &'d Derived, name: &str) -> &'d SchemaFact {
        d.batches
            .iter()
            .flat_map(|(_, facts)| facts)
            .find(|f| f.fqn.as_str().contains(&format!("·{name}·")))
            .unwrap_or_else(|| panic!("no table {name}: {d:#?}"))
    }

    fn columns(f: &SchemaFact) -> Vec<(&str, &str, bool, bool, bool)> {
        f.columns
            .iter()
            .map(|c| {
                (c.name.as_str(), c.sql_type.as_str(), c.primary_key, c.not_null, c.foreign_key)
            })
            .collect()
    }

    /// **A STATED NAME WINS; A CONVENTION IS LABELLED ONE.** `ToTable("orders",
    /// "ops")` names table and schema; `Customers` is EF's convention (the
    /// DbSet's name) under the context's `HasDefaultSchema("sales")`.
    ///
    /// Mutations that must break this: ignore ToTable's schema; name a
    /// convention table after the class instead of the DbSet; label it stated.
    #[test]
    fn a_stated_name_wins_and_a_convention_is_labelled() {
        let d = shop(Some(Provider::SqlServer));
        assert_eq!(d.batches.len(), 1);
        assert_eq!(
            d.batches[0].0,
            BatchScope { source: SchemaSource::EfCore, package: "Shop".into() }
        );

        let orders = table(&d, "orders");
        assert_eq!(orders.fqn, object_identity("Shop", "ops", "orders").unwrap());
        assert_eq!((orders.db_schema.as_str(), orders.naming), ("ops", Naming::Stated));
        assert_eq!((orders.source, orders.kind), (SchemaSource::EfCore, SchemaKind::Table));

        let customers = table(&d, "Customers");
        assert_eq!(customers.fqn, object_identity("Shop", "sales", "Customers").unwrap());
        assert_eq!(customers.naming, Naming::Convention);
        assert_eq!(table(&d, "Notes").db_schema, "sales");
    }

    /// Columns are the entity's properties in order, MINUS its navigations; a
    /// key comes from `HasKey` or EF's `Id` / `<Type>Id`; `not_null` only where
    /// the CLR type says so.
    ///
    /// Mutations that must break this: keep a navigation as a column; mark a
    /// reference type not-null; ignore HasKey; drop the `<Type>Id` convention.
    #[test]
    fn columns_are_properties_minus_navigations() {
        let d = shop(Some(Provider::SqlServer));
        assert_eq!(
            columns(table(&d, "orders")),
            [
                ("Id", "int", true, true, false),
                ("Reference", "string?", false, false, false),
                ("CustomerCode", "string", false, false, true),
                ("Total", "decimal", false, true, false),
            ]
        );
        assert_eq!(
            columns(table(&d, "Customers")),
            [("Code", "string", true, true, false), ("Name", "string", false, false, false)],
            "HasKey(c => c.Code) is the key, and a key is not null"
        );
    }

    /// An `IEntityTypeConfiguration<T>` configures `T` through its builder
    /// parameter: `ToTable` names the table and `HasColumnName` renames a
    /// column. A configured-only entity joins the repository's one context.
    #[test]
    fn a_configuration_class_configures_its_builders_entity() {
        let d = shop(Some(Provider::SqlServer));
        let lines = table(&d, "order_lines");
        assert_eq!((lines.db_schema.as_str(), lines.naming), ("sales", Naming::Stated));
        assert_eq!(
            columns(lines),
            [
                ("LineId", "int", true, true, false),
                ("quantity", "int", false, true, false),
                ("OrderId", "int", false, true, true),
            ]
        );
    }

    /// A reference navigation with a matching key property is a foreign key to
    /// the navigated entity's KEY — whatever that key is called.
    #[test]
    fn a_reference_navigation_is_a_key_to_the_other_entitys_key() {
        let d = shop(Some(Provider::SqlServer));
        assert_eq!(
            table(&d, "orders").refs,
            [SchemaRef {
                from_column: "CustomerCode".into(),
                to: object_identity("Shop", "sales", "Customers").unwrap(),
                to_column: "Code".into(),
                on_delete: None,
            }]
        );
        assert_eq!(
            table(&d, "order_lines").refs[0].to,
            object_identity("Shop", "ops", "orders").unwrap()
        );
        assert_eq!(table(&d, "order_lines").refs[0].to_column, "Id");
    }

    /// No stated schema and no known provider: the entity is SKIPPED and named,
    /// never filed under a guessed schema. A stated schema still lands.
    #[test]
    fn no_schema_and_no_provider_is_skipped_not_guessed() {
        let files = [
            walk(
                "src/ShopContext.cs",
                &CONTEXT.replace(r#"modelBuilder.HasDefaultSchema("sales");"#, ""),
            ),
            walk("src/Entities.cs", ENTITIES),
            walk("src/LineConfig.cs", CONFIG),
        ];
        let d = derive(&input_of(&files, None)).unwrap();
        let skipped: Vec<(&str, SkipReason)> =
            d.skipped.iter().map(|s| (s.entity.as_str(), s.reason)).collect();
        assert_eq!(
            skipped,
            [
                ("Customer", SkipReason::NoSchema),
                ("Note", SkipReason::NoSchema),
                ("Line", SkipReason::NoSchema)
            ]
        );
        assert_eq!(table(&d, "orders").db_schema, "ops", "a stated schema needs no provider");

        // The provider's default fills the gap when nothing is stated.
        let d = derive(&input_of(&files, Some(Provider::Npgsql))).unwrap();
        assert!(d.skipped.is_empty(), "{:?}", d.skipped);
        assert_eq!(table(&d, "Customers").db_schema, "public");
    }

    /// An entity whose class the repository does not declare is skipped and
    /// named — no table of guessed columns.
    #[test]
    fn an_entity_with_no_class_is_skipped() {
        let files = [walk(
            "src/ShopContext.cs",
            &CONTEXT.replace("DbSet<Note> Notes", "DbSet<Ghost> Ghosts"),
        )];
        let d = derive(&input_of(&files, Some(Provider::SqlServer))).unwrap();
        assert!(
            d.skipped.contains(&Skipped { entity: "Ghost".into(), reason: SkipReason::NoClass })
        );
    }

    /// A repository with no DbContext derives nothing — and says so with an
    /// empty result, not an error.
    #[test]
    fn no_context_derives_nothing() {
        let d = derive(&input_of(&[walk("src/Entities.cs", ENTITIES)], None)).unwrap();
        assert!(d.batches.is_empty() && d.skipped.is_empty());
    }
}
