// Declared `#[cfg(test)] mod quality;` from lib.rs: test code with no marker of its own.
pub async fn seeded(pg: &Pg) {
    let rows = pg.list_rows().await.unwrap_or_default();
}
