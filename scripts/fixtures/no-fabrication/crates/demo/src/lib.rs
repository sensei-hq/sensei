// Fixture for `check-no-fabrication.py --test`. Never compiled.
// A line carrying `expect: <RULE>` must be counted under that rule; every
// other line must produce no hit at all. `expect: <RULE> allowed` is matched
// by the fixture allowlist and must be reported as allowlisted, not counted.

#[cfg(test)]
mod quality;

pub async fn r1_await(pg: &Pg) -> Vec<Row> {
    let rows = pg.list_rows().await.unwrap_or_default(); // expect: R1
    let n = pg
        .count_rows()
        .await
        .unwrap_or(0); // expect: R1
    let body = std::fs::read_to_string(path).unwrap_or_else(|_| String::new()); // expect: R1
    let resp = client.get(url).send().await.ok(); // expect: R1
    let text = resp.text().await.unwrap_or_default(); // expect: R1 allowed
    rows
}

pub async fn r2_ok_question(pg: &Pg) -> Option<Ws> {
    let id = pg.folder_workspace_id(cwd).await.ok()?; // expect: R2
    let m = pg.membership(id).await.ok()??; // expect: R2
    Some(m)
}

pub async fn r3_err_arms(pg: &Pg) -> Vec<Row> {
    let rows = match pg.list().await {
        Ok(r) => r,
        Err(_) => Vec::new(), // expect: R3
    };
    let n = match pg.count().await {
        Ok(v) => v,
        Err(e) => { // expect: R3
            tracing::warn!(error = %e, "count failed");
            0
        }
    };
    let names = match load() {
        Ok(v) => v,
        Err(_) => return Ok(vec![]), // expect: R3
    };
    for p in paths {
        let body = match std::fs::read_to_string(p) {
            Ok(b) => b,
            Err(_) => continue, // expect: R3
        };
    }
    let trust = match pg.trust().await {
        Ok(t) => t,
        Err(e) => { // expect: R3
            tracing::warn!(error = %e, "trust query failed");
            (false, 0, 0.0)
        }
    };
    let map = match pg.distances().await {
        Ok(m) => m,
        Err(_) => std::collections::HashMap::new(), // expect: R3
    };
    rows
}

pub fn r4_drop_bad_rows(rows: Vec<Result<Row, E>>) -> Vec<Row> {
    let a: Vec<Row> = rows.iter().filter_map(|r| r.ok()).collect(); // expect: R4
    let b: Vec<Row> = rows.into_iter().filter_map(Result::ok).collect(); // expect: R4
    let entries: Vec<_> = std::fs::read_dir(dir)?.flatten().collect(); // expect: R4
    a
}

pub fn r5_parse_default(text: &str) -> Config {
    let cfg: Config = serde_json::from_str(text).unwrap_or_default(); // expect: R5
    let t: Option<Toml> = toml::from_str(text).ok(); // expect: R5
    cfg
}

pub fn r6_let_else(paths: &[PathBuf]) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(&paths[0]) else { return Vec::new(); }; // expect: R6
    let Ok(conn) = rusqlite::Connection::open_with_flags( // expect: R6
        db_path,
        flags,
    ) else {
        return Vec::new();
    };
    let Ok(lock) = content.parse::<toml::Value>() else { // expect: R6
        return Manifest::default();
    };
    let Ok(fqn) = self.declare(scope, name) else { return };
    for p in paths {
        let Ok(body) = std::fs::read_to_string(p) else { // expect: R6
            continue;
        };
    }
    vec![text]
}

// ── must NOT be counted ──────────────────────────────────────────────────────

pub async fn propagates(pg: &Pg) -> Result<Vec<Row>, E> {
    let rows = match pg.list().await {
        Ok(r) => r,
        Err(e) => return Err(e.into()),
    };
    let real = pg.find(id).await?.unwrap_or_default();
    Ok(rows)
}

pub fn optional_inputs(v: &serde_json::Value, params: Params, opt: Option<String>) {
    let k = v.get("k").and_then(|x| x.as_str()).unwrap_or("");
    let limit = params.limit.unwrap_or(50);
    let name = opt.unwrap_or_default();
    let label = "pg.x().await.unwrap_or(0) // a string, not code";
    // let rows = pg.list_rows().await.unwrap_or_default();
    /* let id = pg.folder_workspace_id(cwd).await.ok()?; */
    /// Err(_) => Vec::new(),
    let c = 'x';
    let msg = match r {
        Ok(v) => v,
        Err(e) => format!("failed: {e}"),
    };
}

#[cfg(test)]
pub(crate) fn test_only_helper(pg: &Pg) -> Vec<Row> {
    pg.list_rows().await.unwrap_or_default()
}

pub async fn after_a_test_helper(pg: &Pg) -> Vec<Row> {
    pg.list_rows().await.unwrap_or_default() // expect: R1
}

#[cfg(test)]
mod t {
    #[tokio::test]
    async fn fixture() {
        let rows = pg.list_rows().await.unwrap_or_default();
        let x = match pg.list().await { Ok(r) => r, Err(_) => Vec::new() };
    }
}
