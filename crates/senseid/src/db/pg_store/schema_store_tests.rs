//! The schema store's tables (#251 step 2): what the DDL itself guarantees,
//! before any writer exists.
//!
//! Tables, views and routines are nodes; their columns, indexes and column-level
//! foreign keys live in `schema_entities` / `schema_columns` / `schema_indexes` /
//! `schema_refs`, keyed by `node_id`. These tests hold the DDL to the spec
//! (`docs/spec/indexer/20-schema-entities.md`): a node's removal takes its detail
//! with it, a column name is unique within its object, and the closed
//! vocabularies are exactly the spec's.
use super::graph_seed::SeedGraph;
use super::*;

async fn table_node(pg: &PgStore, name: &str) -> (uuid::Uuid, uuid::Uuid) {
    let tag = uuid::Uuid::new_v4();
    let root = format!("/_test/schema-store/{tag}");
    let root_id = pg.add_watch_root(&root, "schema-store", &serde_json::json!([])).await.unwrap();
    let folder = pg.upsert_repo(&root_id, &format!("ss-{tag}"), &root).await.unwrap();
    let def = FqnDef {
        file_path: "ddl/table/app/orders.ddl",
        signature: None,
        line_start: Some(1),
        line_end: Some(9),
        is_exported: true,
        parent_id: None,
    };
    let node = pg
        .seed_node_by_fqn(
            &folder,
            &format!("sql·ss{tag}·app·{name}·item"),
            "struct",
            name,
            Some("sql"),
            Some(def),
        )
        .await
        .unwrap();
    (root_id, node)
}

async fn count(pg: &PgStore, table: &str, node: uuid::Uuid) -> i64 {
    let col = if table == "schema_refs" { "from_node_id" } else { "node_id" };
    let (n,): (i64,) = sqlx_core::query_as::query_as(&format!(
        "SELECT count(*) FROM sensei.{table} WHERE {col} = $1"
    ))
    .bind(node)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    n
}

/// Mutations that must break this: drop `on delete cascade` from any of the
/// four foreign keys to the node.
#[tokio::test]
async fn removing_a_node_removes_its_schema_detail() {
    let pg = PgStore::test_store().await;
    let (root, orders) = table_node(&pg, "orders").await;
    let (_, customers) = table_node(&pg, "customers").await;
    for n in [orders, customers] {
        sqlx_core::query::query(
            "INSERT INTO sensei.schema_entities (node_id, kind, db_schema, source, naming)
             VALUES ($1, 'table', 'app', 'dbd', 'stated')",
        )
        .bind(n)
        .execute(pg.pool())
        .await
        .unwrap();
    }
    sqlx_core::query::query(
        "INSERT INTO sensei.schema_columns (node_id, ordinal, name, sql_type, primary_key, not_null)
         VALUES ($1, 1, 'id', 'uuid', true, true), ($1, 2, 'customer_id', 'uuid', false, true)",
    )
    .bind(orders)
    .execute(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.schema_indexes (node_id, name, definition, unique_index)
         VALUES ($1, 'orders_customer_idx', '(customer_id)', false)",
    )
    .bind(orders)
    .execute(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.schema_refs (from_node_id, from_column, to_node_id, to_column, on_delete)
         VALUES ($1, 'customer_id', $2, 'id', 'cascade')",
    )
    .bind(orders)
    .bind(customers)
    .execute(pg.pool())
    .await
    .unwrap();
    for t in ["schema_entities", "schema_columns", "schema_indexes", "schema_refs"] {
        assert!(count(&pg, t, orders).await > 0, "{t} seeded");
    }

    sqlx_core::query::query("DELETE FROM sensei.nodes WHERE id = $1")
        .bind(orders)
        .execute(pg.pool())
        .await
        .unwrap();
    for t in ["schema_entities", "schema_columns", "schema_indexes", "schema_refs"] {
        assert_eq!(count(&pg, t, orders).await, 0, "{t} follows its node out");
    }

    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root)
        .execute(pg.pool())
        .await
        .ok();
}

/// A foreign key is to the REFERENCED object too: removing `customers` removes
/// the ref that pointed at it, rather than leaving a line to nothing.
///
/// Mutation that must break this: drop the cascade on `schema_refs.to_node_id`.
#[tokio::test]
async fn removing_the_referenced_object_removes_the_ref() {
    let pg = PgStore::test_store().await;
    let (root, orders) = table_node(&pg, "orders").await;
    let (_, customers) = table_node(&pg, "customers").await;
    for n in [orders, customers] {
        sqlx_core::query::query(
            "INSERT INTO sensei.schema_entities (node_id, kind, db_schema, source, naming) VALUES ($1, 'table', 'app', 'dbd', 'stated')",
        )
        .bind(n)
        .execute(pg.pool())
        .await
        .unwrap();
    }
    sqlx_core::query::query(
        "INSERT INTO sensei.schema_refs (from_node_id, from_column, to_node_id, to_column) VALUES ($1, 'customer_id', $2, 'id')",
    )
    .bind(orders)
    .bind(customers)
    .execute(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.nodes WHERE id = $1")
        .bind(customers)
        .execute(pg.pool())
        .await
        .unwrap();
    assert_eq!(count(&pg, "schema_refs", orders).await, 0);
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root)
        .execute(pg.pool())
        .await
        .ok();
}

/// One column name per object, and detail only for an object the store knows.
///
/// Mutations that must break this: drop the (node_id, name) unique, or point
/// `schema_columns.node_id` at `nodes` instead of `schema_entities`.
#[tokio::test]
async fn a_column_is_unique_within_its_object_and_needs_the_object() {
    let pg = PgStore::test_store().await;
    let (root, orders) = table_node(&pg, "orders").await;
    let col = "INSERT INTO sensei.schema_columns (node_id, ordinal, name, sql_type) VALUES ($1, $2, 'id', 'uuid')";
    assert!(
        sqlx_core::query::query(col).bind(orders).bind(1).execute(pg.pool()).await.is_err(),
        "a column needs its object's schema_entities row first"
    );
    sqlx_core::query::query(
        "INSERT INTO sensei.schema_entities (node_id, kind, db_schema, source, naming) VALUES ($1, 'table', 'app', 'dbd', 'stated')",
    )
    .bind(orders)
    .execute(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query(col).bind(orders).bind(1).execute(pg.pool()).await.unwrap();
    assert!(
        sqlx_core::query::query(col).bind(orders).bind(2).execute(pg.pool()).await.is_err(),
        "a second `id` on one object is refused"
    );
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root)
        .execute(pg.pool())
        .await
        .ok();
}

/// The closed vocabularies are exactly the spec's — a sixth object kind or an
/// unlisted ORM spelling is refused by the type, not discovered by a reader.
#[tokio::test]
async fn the_vocabularies_are_the_specs() {
    let pg = PgStore::test_store().await;
    let labels = |ty: &'static str| {
        let pg = pg.clone();
        async move {
            let (v,): (Vec<String>,) = sqlx_core::query_as::query_as(&format!(
                "SELECT enum_range(NULL::sensei.{ty})::text[]"
            ))
            .fetch_one(pg.pool())
            .await
            .unwrap();
            v
        }
    };
    assert_eq!(
        labels("schema_object_kind").await,
        ["table", "view", "materialized_view", "function", "procedure"]
    );
    assert_eq!(labels("schema_naming").await, ["stated", "convention"]);
    assert_eq!(
        labels("schema_source").await,
        [
            "dbd",
            "ef_core",
            "dapper",
            "sqlalchemy",
            "django_orm",
            "jpa",
            "prisma",
            "drizzle",
            "typeorm",
            "active_record",
            "gorm",
            "other"
        ]
    );
    assert_eq!(
        labels("fk_action").await,
        ["cascade", "restrict", "set_null", "set_default", "no_action"]
    );
}

/// The Schema diagram is a WORKSPACE's: one workspace's read never carries
/// another's objects, and a workspace's folders are exactly the ones placed in it.
///
/// Mutations that must break this: drop the workspace filter from
/// `schema_folders_of_workspace`; read `schema_detail_in` without its folder
/// filter.
#[tokio::test]
async fn a_workspace_reads_only_its_own_schema() {
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let mut placed = vec![];
    for name in ["mine", "theirs"] {
        let root = format!("/_test/schema-scope/{tag}/{name}");
        let root_id = pg.add_watch_root(&root, name, &serde_json::json!([])).await.unwrap();
        let folder = pg.upsert_repo(&root_id, &format!("{name}-{tag}"), &root).await.unwrap();
        let workspace =
            pg.create_workspace(&format!("_test:schema:{name}:{tag}"), None, None).await.unwrap();
        crate::tasks::test_support::place_folder_in_workspace(
            &pg,
            &folder,
            &workspace,
            &format!("{name}-{tag}"),
        )
        .await
        .unwrap();
        let node = pg
            .seed_node_by_fqn(
                &folder,
                &format!("sql·{name}·app·{name}_table·item"),
                "struct",
                &format!("{name}_table"),
                Some("sql"),
                None,
            )
            .await
            .unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.schema_entities (node_id, kind, db_schema, source, naming)
             VALUES ($1, 'table', 'app', 'dbd', 'stated')",
        )
        .bind(node)
        .execute(pg.pool())
        .await
        .unwrap();
        placed.push((root_id, folder, workspace));
    }
    let (_, mine, my_workspace) = placed[0];

    let folders = pg.schema_folders_of_workspace(&my_workspace).await.unwrap();
    assert_eq!(folders, vec![mine]);

    // THE STORE'S OWN VERSION, for the diagram cache: the import writes after
    // the file tasks without touching `files`, so the graph's version alone
    // cannot see it. Every detail write moves it.
    let before = pg.schema_version_of(&folders).await.unwrap();
    assert!(before.is_some(), "a described object has a version");
    let none: Vec<uuid::Uuid> = vec![];
    assert_eq!(pg.schema_version_of(&none).await.unwrap(), None, "nothing described, no version");
    sqlx_core::query::query(
        "UPDATE sensei.schema_entities se SET modified_at = now() + interval '1 second'
           FROM sensei.nodes n WHERE n.id = se.node_id AND n.folder_id = $1",
    )
    .bind(mine)
    .execute(pg.pool())
    .await
    .unwrap();
    assert!(pg.schema_version_of(&folders).await.unwrap() > before, "a rewrite moves the version");
    let names: Vec<String> =
        pg.schema_detail_in(&folders).await.unwrap().into_iter().map(|e| e.name).collect();
    assert_eq!(names, ["mine_table"]);

    for (root, ..) in placed {
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
            .bind(root)
            .execute(pg.pool())
            .await
            .ok();
    }
}

/// **A NODE THE SCHEMA STORE DESCRIBES IS NOT AN ORPHAN.** An ORM's table has
/// no file of its own — no walk declares it — and often no edge yet, which is
/// exactly what `prune_orphan_stubs_scoped` collects. Seen live (#251 step 7d):
/// the EF Core derivation wrote 9 tables and the next barrier's GC deleted all
/// 9, their detail with them by cascade. The store describing a node is a
/// reference to it; a stub the store no longer describes is still collected.
///
/// Mutation that must break this: drop the schema_entities clause from the GC.
#[tokio::test]
async fn a_described_stub_survives_the_orphan_gc() {
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let root = format!("/_test/schema-gc/{tag}");
    let root_id = pg.add_watch_root(&root, "schema-gc", &serde_json::json!([])).await.unwrap();
    let folder = pg.upsert_repo(&root_id, &format!("gc-{tag}"), &root).await.unwrap();
    let stub = |name: &'static str| {
        let pg = pg.clone();
        async move {
            pg.upsert_node_by_fqn(
                &folder,
                &format!("sql·Gc{tag}·dbo·{name}·item"),
                "unknown",
                name,
                Some("sql"),
                None,
            )
            .await
            .unwrap()
        }
    };
    let (described, bare) = (stub("Orders").await, stub("Gone").await);
    sqlx_core::query::query(
        "INSERT INTO sensei.schema_entities (node_id, kind, db_schema, source, naming)
         VALUES ($1, 'table', 'dbo', 'ef_core', 'convention')",
    )
    .bind(described)
    .execute(pg.pool())
    .await
    .unwrap();

    pg.prune_orphan_stubs_scoped(&[folder]).await.unwrap();

    let alive = |id: uuid::Uuid| {
        let pg = pg.clone();
        async move {
            let (n,): (i64,) =
                sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.nodes WHERE id = $1")
                    .bind(id)
                    .fetch_one(pg.pool())
                    .await
                    .unwrap();
            n == 1
        }
    };
    assert!(alive(described).await, "the store describes it, so it is referenced");
    assert!(!alive(bare).await, "an undescribed stub is still an orphan");
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(pg.pool())
        .await
        .ok();
}
