// A `*_tests.rs` sibling: test code by name.
pub async fn seeded(pg: &Pg) {
    let rows = pg.list_rows().await.unwrap_or_default();
}
