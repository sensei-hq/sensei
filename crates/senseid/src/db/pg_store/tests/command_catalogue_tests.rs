//! The build/test command catalogue (docs/spec/daemon/tracker-sdlc-metrics.md S3):
//! a datafile row is all it takes to classify a new toolchain, and any change to
//! the catalogue changes its hash, so every session is derived again.
use super::*;
use crate::sdlc::classify::{Category, Exit, RunSignal, classify};

#[tokio::test]
async fn command_catalogue_import() {
    let s = PgStore::test_store().await;
    let before = s.command_catalogue().await.expect("the seeded catalogue reads");
    assert_eq!(
        classify(
            "cargo test 2>&1 | tail -1",
            "test result: FAILED. 1 failed",
            Exit::Ok,
            &before,
            &[]
        ),
        [RunSignal::Fail(Category::Test)],
        "the template carries the seed"
    );
    assert!(classify("zig build", "", Exit::Ok, &before, &[]).is_empty());

    for sql in [
        "INSERT INTO staging.command_patterns (toolchain, pattern, category, wrapper, compiles, priority)
         VALUES ('zig', '^zig\\s+build\\b', 'build', false, false, 0)",
        "INSERT INTO staging.command_markers (toolchain, category, kind, marker)
         VALUES ('zig', 'build', 'fail', 'error: the following build command failed')",
        "CALL staging.import_command_patterns()",
        "CALL staging.import_command_markers()",
    ] {
        sqlx_core::query::query(sql).execute(s.pool()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
    }

    let after = s.command_catalogue().await.expect("the catalogue reads after an import");
    assert_ne!(after.hash, before.hash, "a new row changes the hash");
    assert_eq!(
        classify(
            "zig build 2>&1 | tail -1",
            "error: the following build command failed with exit code 1",
            Exit::Ok,
            &after,
            &[]
        ),
        [RunSignal::Fail(Category::Build)]
    );

    let again = s.command_catalogue().await.expect("reads again");
    assert_eq!(again.hash, after.hash, "the hash is stable for the same rows");
}

async fn exec(s: &PgStore, sql: &str) {
    sqlx_core::query::query(sql).execute(s.pool()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

#[tokio::test]
async fn the_hash_covers_markers_and_flags_but_not_timestamps() {
    let s = PgStore::test_store().await;
    let base = s.command_catalogue().await.expect("reads").hash;

    exec(&s, "UPDATE sensei.command_markers SET modified_at = modified_at + interval '1 day'")
        .await;
    exec(&s, "UPDATE sensei.command_patterns SET modified_at = modified_at + interval '1 day'")
        .await;
    assert_eq!(
        s.command_catalogue().await.unwrap().hash,
        base,
        "a timestamp alone re-derives nothing"
    );

    exec(&s, "INSERT INTO sensei.command_markers (toolchain, category, kind, marker) VALUES ('cargo', 'test', 'fail', 'zzz')").await;
    let with_marker = s.command_catalogue().await.unwrap().hash;
    assert_ne!(with_marker, base, "a marker-only change changes the hash");

    exec(&s, "UPDATE sensei.command_patterns SET compiles = NOT compiles WHERE toolchain = 'go'")
        .await;
    let flipped = s.command_catalogue().await.unwrap().hash;
    assert_ne!(flipped, with_marker, "a flag-only change changes the hash");

    let mut last = flipped;
    for (what, sql) in [
        (
            "owns",
            "UPDATE sensei.command_patterns SET owns = array['vitest'] WHERE toolchain = 'npm'",
        ),
        (
            "priority",
            "UPDATE sensei.command_patterns SET priority = priority + 1 WHERE toolchain = 'go'",
        ),
        (
            "wrapper",
            "UPDATE sensei.command_patterns SET wrapper = NOT wrapper WHERE toolchain = 'go'",
        ),
        (
            "category",
            "UPDATE sensei.command_patterns SET category = 'none' WHERE toolchain = 'swift'",
        ),
        ("a deleted row", "DELETE FROM sensei.command_markers WHERE toolchain = 'mix'"),
    ] {
        exec(&s, sql).await;
        let now = s.command_catalogue().await.unwrap().hash;
        assert_ne!(now, last, "{what} changes the hash");
        last = now;
    }
}

#[tokio::test]
async fn reimporting_the_same_seed_keeps_the_hash() {
    let s = PgStore::test_store().await;
    let before = s.command_catalogue().await.expect("reads").hash;
    // The template keeps the datafile in staging, as a deploy leaves it.
    let (staged,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM staging.command_patterns")
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(staged > 0, "staging holds the seed, so the re-import is not vacuous");
    exec(&s, "CALL staging.import_command_patterns()").await;
    exec(&s, "CALL staging.import_command_markers()").await;
    assert_eq!(s.command_catalogue().await.unwrap().hash, before);
}

#[tokio::test]
async fn the_seed_in_the_template_is_the_datafile() {
    let s = PgStore::test_store().await;
    let (patterns, markers): (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM sensei.command_patterns), (SELECT count(*) FROM sensei.command_markers)",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let lines = |f: &str| f.lines().filter(|l| !l.trim().is_empty()).count() as i64;
    assert_eq!(
        patterns,
        lines(include_str!("../../../../../../database/import/staging/command_patterns.jsonl"))
    );
    assert_eq!(
        markers,
        lines(include_str!("../../../../../../database/import/staging/command_markers.jsonl"))
    );
}

#[tokio::test]
async fn a_catalogue_row_that_does_not_compile_is_an_error() {
    let s = PgStore::test_store().await;
    sqlx_core::query::query(
        "INSERT INTO sensei.command_patterns (toolchain, pattern, category) VALUES ('broken', '(', 'build')",
    )
    .execute(s.pool())
    .await
    .expect("insert");
    let err = s.command_catalogue().await.expect_err("an uncompilable pattern is not skipped");
    assert!(err.contains("broken"), "{err}");
}
