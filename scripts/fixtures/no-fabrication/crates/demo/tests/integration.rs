// Under `tests/`: test code by directory.
async fn it_works(pg: &Pg) {
    let rows = pg.list_rows().await.unwrap_or_default();
}
