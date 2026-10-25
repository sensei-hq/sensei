//! An installed daemon upgrades its database with `database::deploy_to`. These
//! tests start from a database an OLDER release left behind and deploy this
//! release's design over it — the path every `brew upgrade` takes, which a
//! deploy onto an empty database never exercises.
//!
//! Each test makes its own scratch database on the `TEST_DATABASE_URL` server
//! (default `sensei_test`'s, the monorepo's throwaway) and drops it after. They
//! are `#[ignore]`d because bootstrap's suite runs without Postgres elsewhere
//! (the no-DB coverage job); `rust.yml` runs them by name with `--ignored`:
//! `cargo test -p sensei-bootstrap --test deploy_upgrade -- --ignored`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn server_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT)
    })
}

/// `server_url` with its database swapped for `db`.
fn url_for(db: &str) -> String {
    let url = server_url();
    let (base, _) = url.rsplit_once('/').expect("a database URL names its database");
    format!("{base}/{db}")
}

fn psql(db: &str, sql: &str) -> String {
    let out = Command::new("psql")
        .args([&url_for(db), "-v", "ON_ERROR_STOP=1", "-Atc", sql])
        .output()
        .expect("psql runs");
    assert!(out.status.success(), "psql failed on {sql}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A scratch database, dropped when the test ends.
struct Scratch(String);

impl Scratch {
    fn new(name: &str) -> Self {
        let db = format!("sensei_deploy_{name}_{}", std::process::id());
        psql("postgres", &format!("drop database if exists {db}"));
        psql("postgres", &format!("create database {db}"));
        Scratch(db)
    }
    fn url(&self) -> String {
        url_for(&self.0)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = Command::new("psql")
            .args([
                &url_for("postgres"),
                "-c",
                &format!("drop database if exists {} with (force)", self.0),
            ])
            .output();
    }
}

fn design_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

#[tokio::test]
#[ignore = "needs Postgres — rust.yml runs these with --ignored"]
async fn an_enum_an_older_release_created_gains_the_values_this_release_added() {
    let db = Scratch::new("enum");
    let backups = tempfile::tempdir().unwrap();
    // v0.11.0's model_capability: no `decision`, which this release's seed uses.
    psql(
        &db.0,
        "create schema sensei; \
         create type sensei.model_capability as enum \
           ('chat', 'reasoning', 'embed', 'classify', 'summarize', 'vision', 'audio', 'image')",
    );

    sensei_bootstrap::database::deploy_to(&db.url(), &design_dir(), backups.path())
        .await
        .expect("an upgrade deploys over the older database");

    let values = psql(&db.0, "select enum_range(null::sensei.model_capability)::text");
    assert!(values.contains("decision"), "model_capability after the upgrade: {values}");
}

#[tokio::test]
#[ignore = "needs Postgres — rust.yml runs these with --ignored"]
async fn a_fresh_install_gets_the_whole_schema_and_its_seed() {
    let db = Scratch::new("fresh");
    let backups = tempfile::tempdir().unwrap();

    sensei_bootstrap::database::deploy_to(&db.url(), &design_dir(), backups.path())
        .await
        .expect("a deploy onto an empty database succeeds");

    // A table, a view, a function and seeded rows: every kind of object the
    // install path has to create, not only the tables reconcile diffs.
    assert_eq!(psql(&db.0, "select to_regclass('sensei.folders') is not null"), "t");
    assert_eq!(psql(&db.0, "select to_regclass('sensei.graph_placement') is not null"), "t");
    assert_eq!(
        psql(&db.0, "select count(*) > 0 from pg_proc where proname = 'resolution_quality'"),
        "t"
    );
    assert_eq!(psql(&db.0, "select count(*) > 0 from sensei.reason_codes"), "t");
}

#[tokio::test]
#[ignore = "needs Postgres — rust.yml runs these with --ignored"]
async fn a_column_the_design_dropped_is_backed_up_before_it_goes() {
    let db = Scratch::new("drop");
    let backups = tempfile::tempdir().unwrap();
    let (url, design) = (db.url(), design_dir());
    let deploy = || sensei_bootstrap::database::deploy_to(&url, &design, backups.path());
    deploy().await.expect("the first deploy");
    // A column an older release had and this design no longer declares.
    psql(
        &db.0,
        "alter table sensei.reason_codes add column retired_note text; \
         update sensei.reason_codes set retired_note = 'kept-by-the-backup'",
    );

    deploy().await.expect("the upgrade drops the retired column");

    let column = psql(
        &db.0,
        "select count(*) from pg_attribute \
         where attrelid = 'sensei.reason_codes'::regclass and attname = 'retired_note' and not attisdropped",
    );
    assert_eq!(column, "0", "the retired column is gone");
    let saved: Vec<_> = walk(backups.path())
        .into_iter()
        .filter(|f| {
            std::fs::read(f)
                .is_ok_and(|b| String::from_utf8_lossy(&b).contains("kept-by-the-backup"))
        })
        .collect();
    assert_eq!(
        saved.len(),
        1,
        "one backup holds the dropped column's data: {:?}",
        walk(backups.path())
    );
}

#[tokio::test]
#[ignore = "needs Postgres — rust.yml runs these with --ignored"]
async fn a_deploy_that_drops_nothing_writes_no_backup() {
    let db = Scratch::new("nodrop");
    let backups = tempfile::tempdir().unwrap();
    sensei_bootstrap::database::deploy_to(&db.url(), &design_dir(), backups.path()).await.unwrap();
    sensei_bootstrap::database::deploy_to(&db.url(), &design_dir(), backups.path()).await.unwrap();
    assert!(walk(backups.path()).is_empty(), "backups: {:?}", walk(backups.path()));
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// A design of one table and one view over it, in `dir`: v1's table has
/// `legacy`, and its view reads it; v2 drops the column and the view no longer
/// reads it — the shape of `folders.project_id` between 0.11 and 0.12, which
/// thirteen of 0.11's views read.
fn small_design(dir: &Path, legacy: bool) {
    let ddl = dir.join("ddl");
    std::fs::create_dir_all(ddl.join("table/app")).unwrap();
    std::fs::create_dir_all(ddl.join("view/app")).unwrap();
    std::fs::write(
        dir.join("design.yaml"),
        "project:\n  name: upgrade_fixture\nsource:\n  dialect: postgresql\nschemas:\n  - app\n",
    )
    .unwrap();
    let (column, read) = if legacy { (", legacy text", ", legacy") } else { ("", "") };
    std::fs::write(
        ddl.join("table/app/items.ddl"),
        format!("create table if not exists app.items (\n  id int primary key\n{column}\n);\n"),
    )
    .unwrap();
    std::fs::write(
        ddl.join("view/app/item_list.ddl"),
        format!("create or replace view app.item_list as\nselect id{read} from app.items;\n"),
    )
    .unwrap();
}

#[tokio::test]
#[ignore = "needs Postgres — rust.yml runs these with --ignored"]
async fn a_column_an_older_release_s_views_read_is_dropped_once_they_are_replaced() {
    let db = Scratch::new("viewdep");
    let backups = tempfile::tempdir().unwrap();
    let (v1, v2) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    small_design(v1.path(), true);
    small_design(v2.path(), false);
    sensei_bootstrap::database::deploy_to(&db.url(), v1.path(), backups.path())
        .await
        .expect("v1 deploys");
    psql(&db.0, "insert into app.items values (1, 'from-v1')");

    sensei_bootstrap::database::deploy_to(&db.url(), v2.path(), backups.path())
        .await
        .expect("v2 replaces the view, then drops the column it no longer reads");

    assert_eq!(
        psql(
            &db.0,
            "select string_agg(attname, ',') from pg_attribute where attrelid = 'app.items'::regclass and attnum > 0 and not attisdropped"
        ),
        "id"
    );
    let saved = walk(backups.path());
    assert_eq!(saved.len(), 1, "{saved:?}");
    assert!(std::fs::read_to_string(&saved[0]).unwrap().contains("from-v1"));
}

/// A design of one table, in `dir`: v2 adds a column and comments on it, in the
/// same file — the shape of `sensei.trackers.level` in 0.20.0.
fn commented_design(dir: &Path, added: bool) {
    let ddl = dir.join("ddl");
    std::fs::create_dir_all(ddl.join("table/app")).unwrap();
    std::fs::write(
        dir.join("design.yaml"),
        "project:\n  name: upgrade_fixture\nsource:\n  dialect: postgresql\nschemas:\n  - app\n",
    )
    .unwrap();
    let (column, comment) = if added {
        (", added text", "\ncomment on column app.things.added is 'added by v2';\n")
    } else {
        ("", "")
    };
    std::fs::write(
        ddl.join("table/app/things.ddl"),
        format!(
            "create table if not exists app.things (\n  id int primary key\n{column}\n);\n{comment}"
        ),
    )
    .unwrap();
}

/// 0.20.0 shipped a daemon whose upgrade failed: `apply` re-ran the table's file
/// for a table that already existed, so its `comment on column` ran before
/// `reconcile` had added the column ("column \"level\" of relation \"trackers\"
/// does not exist"), and the daemon kept the previous schema.
#[tokio::test]
#[ignore = "needs Postgres — rust.yml runs these with --ignored"]
async fn a_column_this_release_adds_and_comments_on_is_added_before_its_comment_runs() {
    let db = Scratch::new("addcomment");
    let backups = tempfile::tempdir().unwrap();
    let (v1, v2) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    commented_design(v1.path(), false);
    commented_design(v2.path(), true);
    sensei_bootstrap::database::deploy_to(&db.url(), v1.path(), backups.path())
        .await
        .expect("v1 deploys");
    psql(&db.0, "insert into app.things values (1)");

    sensei_bootstrap::database::deploy_to(&db.url(), v2.path(), backups.path())
        .await
        .expect("v2 adds the column, then comments on it");

    assert_eq!(psql(&db.0, "select col_description('app.things'::regclass, 2)"), "added by v2");
    assert_eq!(psql(&db.0, "select count(*) from app.things"), "1", "the row stays");
}

/// A design of one table and a view over it, in `dir`: v2 adds two generated
/// columns, a CASE among them, and the view reads them — the shape of
/// `activity.assistant_events.tool_use_id` / `duration_ms` (#298).
fn generated_design(dir: &Path, added: bool) {
    let ddl = dir.join("ddl");
    std::fs::create_dir_all(ddl.join("table/app")).unwrap();
    std::fs::create_dir_all(ddl.join("view/app")).unwrap();
    std::fs::write(
        dir.join("design.yaml"),
        "project:\n  name: upgrade_fixture\nsource:\n  dialect: postgresql\nschemas:\n  - app\n",
    )
    .unwrap();
    let (columns, reads) = if added {
        (
            ",\n  key text generated always as (payload->>'key') stored\
             ,\n  ms bigint generated always as (\
                case when jsonb_typeof(payload->'ms') = 'number' \
                then (payload->>'ms')::numeric::bigint end) stored",
            "key, ms",
        )
    } else {
        ("", "payload->>'key' as key")
    };
    std::fs::write(
        ddl.join("table/app/events.ddl"),
        format!(
            "create table if not exists app.events (\n  id int primary key,\n  payload jsonb not null{columns}\n);\n"
        ),
    )
    .unwrap();
    std::fs::write(
        ddl.join("view/app/event_keys.ddl"),
        format!("create or replace view app.event_keys as select id, {reads} from app.events;\n"),
    )
    .unwrap();
}

/// A generated column a release adds arrives GENERATED through the daemon's own
/// deploy, computed for the rows already there. dbd-core before 0.27.6 added a
/// new generated column as a plain one — NULL in every row, so #298's view
/// would have paired nothing — and reported a generated CASE as drifted forever
/// (sensei-hq/dbd#104).
#[tokio::test]
#[ignore = "needs Postgres — rust.yml runs these with --ignored"]
async fn a_generated_column_this_release_adds_is_generated_for_the_rows_already_there() {
    let db = Scratch::new("generated");
    let backups = tempfile::tempdir().unwrap();
    let (v1, v2) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    generated_design(v1.path(), false);
    generated_design(v2.path(), true);
    sensei_bootstrap::database::deploy_to(&db.url(), v1.path(), backups.path())
        .await
        .expect("v1 deploys");
    psql(
        &db.0,
        r#"insert into app.events values (1, '{"key":"k1","ms":12}'), (2, '{"ms":"n/a"}')"#,
    );

    for pass in ["v2 adds the generated columns", "v2 again changes nothing"] {
        sensei_bootstrap::database::deploy_to(&db.url(), v2.path(), backups.path())
            .await
            .expect(pass);
    }

    assert_eq!(
        psql(
            &db.0,
            "select string_agg(attname || '=' || attgenerated::text, ',' order by attname) from pg_attribute \
              where attrelid = 'app.events'::regclass and attname in ('key', 'ms')"
        ),
        "key=s,ms=s",
        "both columns are generated, not plain"
    );
    assert_eq!(
        psql(
            &db.0,
            "select string_agg(concat_ws(':', id, key, ms), ',' order by id) from app.event_keys"
        ),
        "1:k1:12,2",
        "computed for the rows already there; a non-number is NULL"
    );
}
