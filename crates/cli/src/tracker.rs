//! `sensei tracker …` — connect a workspace to its issue tracker (spec R10).
//! The HTTP calls are thin; the decisions (which workspace, what body, where the
//! token comes from, what to print) are pure functions with tests.

use clap::Subcommand;
use serde_json::Value;

/// Env var the token may come from (visible to same-user processes, so stdin is
/// offered too). The token is never an argv flag.
pub const TOKEN_ENV: &str = "SENSEI_JIRA_TOKEN";

#[derive(Subcommand, Debug, PartialEq)]
pub enum TrackerCmd {
    /// Connect a workspace to a tracker board (token from SENSEI_JIRA_TOKEN, else stdin)
    Connect {
        #[command(subcommand)]
        kind: ConnectKind,
    },
    /// Sync the workspace's trackers now (one board with --board)
    Sync {
        #[arg(long)]
        workspace: String,
        /// Which of the workspace's boards: its board number or tracker id (needed when it has several)
        #[arg(long)]
        board: Option<String>,
    },
    /// Disconnect one of the workspace's boards (Jira itself is not changed)
    Disconnect {
        #[arg(long)]
        workspace: String,
        /// Which of the workspace's boards: its board number or tracker id (needed when it has several)
        #[arg(long)]
        board: Option<String>,
    },
    /// A board's sprints, with ticket counts and sessions in each window
    Sprints {
        #[arg(long)]
        workspace: String,
        /// Which of the workspace's boards: its board number or tracker id (needed when it has several)
        #[arg(long)]
        board: Option<String>,
    },
    /// Active AI time and tokens per ticket: a board's tickets, a sprint's, or one
    Usage {
        #[arg(long)]
        workspace: String,
        /// Which of the workspace's boards: its board number or tracker id (needed when it has several)
        #[arg(long)]
        board: Option<String>,
        /// A sprint id or exact name
        #[arg(long, conflicts_with = "key")]
        sprint: Option<String>,
        /// One ticket key (current or former), e.g. AUTH-420
        key: Option<String>,
        /// One row per repository (with a ticket key)
        #[arg(long, requires = "key")]
        by_repo: bool,
    },
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum ConnectKind {
    /// Jira Cloud
    Jira {
        #[arg(long)]
        workspace: String,
        /// The board's address, e.g. https://acme.atlassian.net/jira/software/projects/AUTH/boards/42
        #[arg(long, conflicts_with_all = ["url", "board"])]
        board_url: Option<String>,
        /// Site URL (with --board), instead of --board-url
        #[arg(long, requires = "board")]
        url: Option<String>,
        /// Board id (with --url)
        #[arg(long, requires = "url")]
        board: Option<String>,
        /// The Atlassian account email the token belongs to
        #[arg(long)]
        email: String,
        /// Optional: the board's Jira project key, checked against the board
        #[arg(long)]
        project_key: Option<String>,
        /// What the board serves in the workspace: epic, sprint or task
        #[arg(long)]
        level: String,
    },
}

/// The workspace id for `input` (a uuid or an exact workspace name) from the
/// `GET /api/workspaces` list. Zero or several matches is an error naming them.
pub fn resolve_workspace(workspaces: &Value, input: &str) -> Result<String, String> {
    resolve_named(workspaces, input, "workspace")
}

/// The id of the one `noun` in `list` whose id or exact name is `input`.
fn resolve_named(list: &Value, input: &str, noun: &str) -> Result<String, String> {
    let list = list.as_array().map(Vec::as_slice).unwrap_or_default();
    let id_of = |p: &Value| p.get("id").and_then(Value::as_str).map(String::from);
    let name_of = |p: &Value| p.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    if uuid_like(input) {
        return list
            .iter()
            .filter_map(id_of)
            .find(|id| id.eq_ignore_ascii_case(input))
            .ok_or_else(|| format!("no {noun} has id {input}"));
    }
    let matches: Vec<String> =
        list.iter().filter(|p| name_of(p) == input).filter_map(id_of).collect();
    match matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => {
            let names: Vec<String> = list.iter().map(name_of).collect();
            Err(format!("no {noun} named {input:?}; {noun}s: {}", names.join(", ")))
        }
        many => Err(format!(
            "{} {noun}s are named {input:?} ({}); pass one id with --{noun}",
            many.len(),
            many.join(", ")
        )),
    }
}

/// The tracker id for `--board` (a board number or a tracker id) from the
/// `GET …/trackers` list; without `--board`, the workspace's only tracker.
pub fn resolve_tracker(trackers: &Value, board: Option<&str>) -> Result<String, String> {
    let list = trackers.as_array().map(Vec::as_slice).unwrap_or_default();
    let id_of = |t: &Value| t["id"].as_str().unwrap_or_default().to_string();
    let label = |t: &Value| {
        format!(
            "{} #{} ({})",
            t["level"].as_str().unwrap_or("?"),
            t["board_ref"].as_str().unwrap_or("?"),
            t["board_name"].as_str().unwrap_or("?")
        )
    };
    let boards = || list.iter().map(label).collect::<Vec<_>>().join(", ");
    match (board, list) {
        (_, []) => Err("This workspace has no tracker connected.".into()),
        (None, [one]) => Ok(id_of(one)),
        (None, _) => {
            Err(format!("This workspace has several boards; pass one with --board: {}", boards()))
        }
        (Some(b), _) => {
            let matches: Vec<&Value> = list
                .iter()
                .filter(|t| t["board_ref"].as_str() == Some(b) || id_of(t).eq_ignore_ascii_case(b))
                .collect();
            match matches.as_slice() {
                [one] => Ok(id_of(one)),
                [] => Err(format!("No board {b} in this workspace; boards: {}", boards())),
                // Board numbers are per site: two sites can both have a #42.
                many => Err(format!(
                    "{} boards are numbered {b}; pass the tracker id with --board: {}",
                    many.len(),
                    many.iter()
                        .map(|t| format!(
                            "{} on {} ({})",
                            label(t),
                            t["base_url"].as_str().unwrap_or("?"),
                            id_of(t)
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
    }
}

/// The sprint id for `input` (a uuid or an exact sprint name) from the
/// `GET …/sprints` list, by the same rule as workspaces.
pub fn resolve_sprint(sprints: &Value, input: &str) -> Result<String, String> {
    resolve_named(sprints, input, "sprint")
}

/// Active AI time as `h:mm`; blank when no session had a duration.
pub fn active_time(secs: Option<f64>) -> String {
    secs.map(|s| {
        let minutes = (s.max(0.0) / 60.0).floor() as u64;
        format!("{}:{:02}", minutes / 60, minutes % 60)
    })
    .unwrap_or_default()
}

/// The usage table's header.
pub const USAGE_HEADER: [&str; 8] = [
    "key",
    "summary",
    "sessions (branch/prompt)",
    "active AI time",
    "fresh tokens",
    "cache-read",
    "out tokens",
    "cost",
];

/// One usage row's cells, in [`USAGE_HEADER`] order. Unknown values are blank, never `0`.
pub fn usage_cells(label: &str, summary: &str, usage: &Value) -> Vec<String> {
    let count = |v: &Value| v.as_i64().map(|n| n.to_string()).unwrap_or_default();
    vec![
        label.to_string(),
        summary.to_string(),
        format!(
            "{} ({}/{})",
            usage["sessions"].as_i64().unwrap_or(0),
            usage["by_source"]["branch"].as_i64().unwrap_or(0),
            usage["by_source"]["prompt"].as_i64().unwrap_or(0)
        ),
        active_time(usage["active_secs"].as_f64()),
        count(&usage["tokens_fresh"]),
        count(&usage["cache_read"]),
        count(&usage["tokens_out"]),
        usage["metered_cost"].as_f64().map(|c| format!("{c:.2}")).unwrap_or_default(),
    ]
}

/// Left-aligned columns, two spaces apart.
fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let width = |i: usize| {
        rows.iter()
            .map(|r| r.get(i).map_or(0, |c| c.chars().count()))
            .max()
            .unwrap_or(0)
            .max(header[i].chars().count())
    };
    let widths: Vec<usize> = (0..header.len()).map(width).collect();
    let line = |cells: Vec<&str>| {
        cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!("{c:<w$}"))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    let mut out = vec![line(header.to_vec())];
    out.extend(rows.iter().map(|r| line(r.iter().map(String::as_str).collect())));
    out.join("\n")
}

/// A ticket's summary, shortened for a table cell.
fn short(v: &Value) -> String {
    let s = v.as_str().unwrap_or_default();
    if s.chars().count() > 40 {
        format!("{}…", s.chars().take(39).collect::<String>())
    } else {
        s.into()
    }
}

fn uuid_like(s: &str) -> bool {
    s.len() == 36
        && s.char_indices().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) { c == '-' } else { c.is_ascii_hexdigit() }
        })
}

/// The connect request body for the Jira flags and the token.
pub fn jira_connect_body(
    board_url: Option<&str>,
    url: Option<&str>,
    board: Option<&str>,
    email: &str,
    project_key: Option<&str>,
    level: &str,
    token: &str,
) -> Value {
    let mut body = serde_json::Map::new();
    body.insert("kind".into(), "jira".into());
    body.insert("level".into(), level.into());
    match board_url {
        Some(u) => {
            body.insert("board_url".into(), u.into());
        }
        None => {
            if let Some(u) = url {
                body.insert("base_url".into(), u.into());
            }
            if let Some(b) = board {
                body.insert("board_id".into(), b.into());
            }
        }
    }
    body.insert("email".into(), email.into());
    body.insert("api_token".into(), token.into());
    if let Some(k) = project_key {
        body.insert("project_key".into(), k.into());
    }
    Value::Object(body)
}

/// The token: `env` if set and non-empty, else the first line of `stdin`.
pub fn read_token(env: Option<String>, stdin: &mut dyn std::io::BufRead) -> Result<String, String> {
    if let Some(t) = env.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        return Ok(t.to_string());
    }
    let mut line = String::new();
    stdin.read_line(&mut line).map_err(|e| format!("could not read the token from stdin: {e}"))?;
    let t = line.trim();
    if t.is_empty() {
        return Err(format!("no API token: set {TOKEN_ENV} or pipe the token on stdin"));
    }
    Ok(t.to_string())
}

/// One line for a daemon error body `{"error": code, ...}`.
pub fn describe_error(status: u16, body: &Value) -> String {
    let code = body.get("error").and_then(Value::as_str).unwrap_or("unknown");
    let field = |k: &str| body.get(k).and_then(Value::as_str).unwrap_or("?").to_string();
    match code {
        "unauthorized" => "Jira rejected the email and token. Check both, or create a new token at \
             https://id.atlassian.com/manage-profile/security/api-tokens"
            .into(),
        "forbidden" => "This account can't read that board. Ask the board admin for access, or use another account.".into(),
        "board_not_found" => format!("No board #{} on {}.", field("board_ref"), field("base_url")),
        "task_not_on_board" => format!(
            "{} isn't on board #{}: it may be in another Jira project, or outside the board's filter.",
            field("key"),
            field("board_ref")
        ),
        "task_not_found" => "No ticket with that key in sensei.".into(),
        "board_already_connected" => {
            let p = body.get("workspace");
            let name = p.and_then(|p| p.get("name")).and_then(Value::as_str).unwrap_or("?");
            let id = p.and_then(|p| p.get("id")).and_then(Value::as_str).unwrap_or("?");
            format!("This board is already connected to workspace {name} ({id}).")
        }
        "board_project_mismatch" => "That board belongs to a different Jira project than --project-key.".into(),
        "invalid_base_url" => {
            "That isn't a Jira Cloud board address. It should look like \
             https://<your-site>.atlassian.net/jira/software/projects/<KEY>/boards/<number> \
             (older sites: https://<your-site>.jira.com/…)."
                .into()
        }
        "email_required" => "Pass --email: the Atlassian account the token belongs to.".into(),
        "token_required" => format!("No API token: set {TOKEN_ENV} or pipe it on stdin."),
        "unsupported_tracker" => "This tracker type isn't available in this version of sensei.".into(),
        "not_connected" => "This workspace has no tracker connected.".into(),
        "tracker_not_found" => "No such board in this workspace.".into(),
        "level_required" => "Pass --level: what the board serves (epic, sprint or task).".into(),
        "invalid_level" => "--level must be epic, sprint or task.".into(),
        "workspace_not_found" => "No such workspace.".into(),
        "busy" => "A sync is already running for this board.".into(),
        "superseded" => "The board's token changed while it synced. Sync again.".into(),
        "rate_limited" => "Jira is rate-limiting requests. Try again in a minute.".into(),
        "token_missing" | "credential_store" => {
            "The saved token can't be read from the system keychain. Connect again to replace it.".into()
        }
        "transport" | "http_error" | "redirect" | "decode" => {
            format!("Couldn't reach Jira ({code}). Check the site address and your connection.")
        }
        "forbidden_caller" => "The daemon refused this caller.".into(),
        other => format!("Daemon error {status}: {other}"),
    }
}

/// Run a tracker command against the local daemon; returns the exit code.
pub fn run(cmd: TrackerCmd) -> i32 {
    match execute(cmd) {
        Ok(()) => 0,
        Err(msg) => {
            eprintln!("{msg}");
            1
        }
    }
}

fn execute(cmd: TrackerCmd) -> Result<(), String> {
    let base = crate::daemon_url();
    let client = crate::client();
    let workspace_id = |input: &str| -> Result<String, String> {
        let workspaces = get_json(&client, &format!("{base}/api/workspaces"))?;
        resolve_workspace(&workspaces, input)
    };
    // `…/api/workspaces/{ws}/trackers/{tracker}` for `--board`.
    let tracker_uri = |ws: &str, board: Option<&str>| -> Result<String, String> {
        let list = format!("{base}/api/workspaces/{ws}/trackers");
        let tracker = resolve_tracker(&get_json(&client, &list)?, board)?;
        Ok(format!("{list}/{tracker}"))
    };
    match cmd {
        TrackerCmd::Connect {
            kind: ConnectKind::Jira { workspace, board_url, url, board, email, project_key, level },
        } => {
            let id = workspace_id(&workspace)?;
            let env = std::env::var(TOKEN_ENV).ok();
            if env.as_deref().is_none_or(|t| t.trim().is_empty()) {
                eprintln!("Paste the Jira API token, then press Enter:");
            }
            let token = read_token(env, &mut std::io::stdin().lock())?;
            let body = jira_connect_body(
                board_url.as_deref(),
                url.as_deref(),
                board.as_deref(),
                &email,
                project_key.as_deref(),
                &level,
                &token,
            );
            // Two validation requests at up to 30 s each, plus the keyring.
            let resp = crate::client_with_timeout(90)
                .post(format!("{base}/api/workspaces/{id}/trackers"))
                .json(&body)
                .send()
                .map_err(|e| format!("could not reach the sensei daemon: {e}"))?;
            let status = resp.status().as_u16();
            let v: Value = resp.json().unwrap_or(Value::Null);
            if status != 201 {
                return Err(describe_error(status, &v));
            }
            let t = &v["tracker"];
            println!(
                "Connected {} board \"{}\" (#{}) on {}.",
                t["external_scope"].as_str().unwrap_or("?"),
                t["board_name"].as_str().unwrap_or("?"),
                t["board_ref"].as_str().unwrap_or("?"),
                t["base_url"].as_str().unwrap_or("?"),
            );
            println!("First sync running…");
            wait_for_first_sync(
                &client,
                &format!("{base}/api/workspaces/{id}/trackers"),
                t["id"].as_str().unwrap_or_default(),
            )
        }
        TrackerCmd::Sync { workspace, board } => {
            let id = workspace_id(&workspace)?;
            let list = format!("{base}/api/workspaces/{id}/trackers");
            let trackers = get_json(&client, &list)?;
            let ids: Vec<String> = match board.as_deref() {
                Some(b) => vec![resolve_tracker(&trackers, Some(b))?],
                None => trackers
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|t| t["id"].as_str().map(String::from))
                    .collect(),
            };
            if ids.is_empty() {
                return Err(describe_error(404, &serde_json::json!({"error": "not_connected"})));
            }
            for tracker in ids {
                let resp = crate::client_with_timeout(660)
                    .post(format!("{list}/{tracker}/sync"))
                    .send()
                    .map_err(|e| format!("could not reach the sensei daemon: {e}"))?;
                let status = resp.status().as_u16();
                let v: Value = resp.json().unwrap_or(Value::Null);
                if status != 200 {
                    return Err(describe_error(status, &v));
                }
                let c = &v["counts"];
                println!(
                    "Synced {tracker}: {} sprints, {} tickets, {} session links ({} removed, {} sprint members skipped).",
                    c["sprints"], c["tasks"], c["links"], c["removed"], c["skipped_members"]
                );
            }
            Ok(())
        }
        TrackerCmd::Disconnect { workspace, board } => {
            let id = workspace_id(&workspace)?;
            let resp = client
                .delete(tracker_uri(&id, board.as_deref())?)
                .send()
                .map_err(|e| format!("could not reach the sensei daemon: {e}"))?;
            let status = resp.status().as_u16();
            if status == 204 {
                println!("Disconnected. Jira itself is not changed.");
                return Ok(());
            }
            Err(describe_error(status, &resp.json().unwrap_or(Value::Null)))
        }
        TrackerCmd::Sprints { workspace, board } => {
            let id = workspace_id(&workspace)?;
            let sprints =
                get_json(&client, &format!("{}/sprints", tracker_uri(&id, board.as_deref())?))?;
            let rows: Vec<Vec<String>> = sprints
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|s| {
                    let day = |v: &Value| {
                        v.as_str().map(|d| d.chars().take(10).collect()).unwrap_or_default()
                    };
                    let (window, linked, unlinked) = match s["reason"].as_str() {
                        Some(reason) => (reason.replace('_', " "), String::new(), String::new()),
                        None => (
                            s["candidates_in_window"].to_string(),
                            s["linked_sprint"]["sessions"].to_string(),
                            s["unlinked"]["sessions"].to_string(),
                        ),
                    };
                    vec![
                        s["name"].as_str().unwrap_or_default().to_string(),
                        s["state"].as_str().unwrap_or_default().to_string(),
                        day(&s["start_at"]),
                        day(&s["end_at"]),
                        s["tasks"].to_string(),
                        window,
                        linked,
                        unlinked,
                    ]
                })
                .collect();
            println!(
                "{}",
                table(
                    &[
                        "sprint",
                        "state",
                        "start",
                        "end",
                        "tickets",
                        "sessions in window",
                        "linked",
                        "unlinked"
                    ],
                    &rows
                )
            );
            Ok(())
        }
        TrackerCmd::Usage { workspace, board, sprint, key, by_repo } => {
            let id = workspace_id(&workspace)?;
            let mut rows = Vec::new();
            let mut footer = Vec::new();
            if let Some(key) = key {
                let t =
                    get_json(&client, &format!("{base}/api/workspaces/{id}/tracker-tasks/{key}"))?;
                rows.push(usage_cells(
                    t["task"]["key"].as_str().unwrap_or(&key),
                    &short(&t["task"]["summary"]),
                    &t["usage"],
                ));
                if by_repo {
                    for r in t["by_repo"].as_array().map(Vec::as_slice).unwrap_or_default() {
                        rows.push(usage_cells(
                            &format!("  {}", r["name"].as_str().unwrap_or_default()),
                            "",
                            &r["usage"],
                        ));
                    }
                }
            } else {
                let tracker = tracker_uri(&id, board.as_deref())?;
                let mut uri = format!("{tracker}/tasks");
                let mut sprint_row = None;
                if let Some(sprint) = sprint {
                    let sprints = get_json(&client, &format!("{tracker}/sprints"))?;
                    let sid = resolve_sprint(&sprints, &sprint)?;
                    uri = format!("{uri}?sprint={sid}");
                    sprint_row = sprints
                        .as_array()
                        .and_then(|l| l.iter().find(|s| s["id"].as_str() == Some(sid.as_str())))
                        .cloned();
                }
                let tasks = get_json(&client, &uri)?;
                let list = tasks.as_array().map(Vec::as_slice).unwrap_or_default();
                let worked: Vec<&Value> = list
                    .iter()
                    .filter(|t| t["usage"]["sessions"].as_i64().unwrap_or(0) > 0)
                    .collect();
                rows.extend(worked.iter().map(|t| {
                    usage_cells(
                        t["key"].as_str().unwrap_or_default(),
                        &short(&t["summary"]),
                        &t["usage"],
                    )
                }));
                if list.len() > worked.len() {
                    footer.push(format!(
                        "{} tickets without AI sessions not shown.",
                        list.len() - worked.len()
                    ));
                }
                if let Some(s) = sprint_row {
                    match s["reason"].as_str() {
                        Some(reason) => footer
                            .push(format!("Sprint usage: none ({}).", reason.replace('_', " "))),
                        None => {
                            // The sprint's own total: each session once, never a sum of tickets.
                            rows.push(usage_cells("sprint total", "", &s["linked_sprint"]));
                            footer.push(format!(
                                "Unlinked sessions in the sprint window: {}.",
                                s["unlinked"]["sessions"]
                            ));
                        }
                    }
                }
            }
            println!("{}", table(&USAGE_HEADER, &rows));
            for line in footer {
                println!("{line}");
            }
            Ok(())
        }
    }
}

fn get_json(client: &reqwest::blocking::Client, url: &str) -> Result<Value, String> {
    let resp =
        client.get(url).send().map_err(|e| format!("could not reach the sensei daemon: {e}"))?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().unwrap_or(Value::Null);
    if status != 200 {
        return Err(describe_error(status, &v));
    }
    Ok(v)
}

/// Poll the new connection `id` until its status leaves `pending` (2 s steps, ≤ 120 s).
fn wait_for_first_sync(
    client: &reqwest::blocking::Client,
    url: &str,
    id: &str,
) -> Result<(), String> {
    for _ in 0..60 {
        let list = get_json(client, url)?;
        let t = list
            .as_array()
            .and_then(|l| l.iter().find(|t| t["id"].as_str() == Some(id)))
            .ok_or_else(|| {
                describe_error(404, &serde_json::json!({"error": "tracker_not_found"}))
            })?;
        match t["status"].as_str() {
            Some("pending") => std::thread::sleep(std::time::Duration::from_secs(2)),
            Some("error") => {
                let code = t["last_error"].as_str().unwrap_or("unknown");
                return Err(format!(
                    "Connected, but the first sync failed: {}",
                    describe_error(0, &serde_json::json!({ "error": code }))
                ));
            }
            _ => {
                println!(
                    "First sync done: {} sprints, {} tickets.",
                    t["counts"]["sprints"], t["counts"]["tasks"]
                );
                return Ok(());
            }
        }
    }
    println!("The first sync is still running in the daemon; check with `sensei tracker sync`.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(clap::Parser)]
    struct Probe {
        #[command(subcommand)]
        cmd: TrackerCmd,
    }
    fn parse(args: &[&str]) -> Result<TrackerCmd, clap::Error> {
        use clap::Parser;
        let mut full = vec!["tracker"];
        full.extend_from_slice(args);
        Probe::try_parse_from(full).map(|p| p.cmd)
    }

    #[test]
    fn connect_parses_a_board_url_or_url_plus_board() {
        let c = parse(&[
            "connect",
            "jira",
            "--workspace",
            "p",
            "--board-url",
            "https://a.atlassian.net/x/boards/4",
            "--email",
            "dev@example.com",
            "--level",
            "sprint",
        ])
        .unwrap();
        assert!(
            matches!(c, TrackerCmd::Connect { kind: ConnectKind::Jira { ref board_url, .. } } if board_url.is_some())
        );
        assert!(
            parse(&[
                "connect",
                "jira",
                "--workspace",
                "p",
                "--url",
                "https://a.atlassian.net",
                "--board",
                "4",
                "--email",
                "dev@example.com",
                "--level",
                "epic"
            ])
            .is_ok()
        );
        assert!(
            parse(&[
                "connect",
                "jira",
                "--workspace",
                "p",
                "--board-url",
                "https://a.atlassian.net/x/boards/4",
                "--email",
                "dev@example.com"
            ])
            .is_err(),
            "the level is the user's choice, never defaulted"
        );
    }

    #[test]
    fn connect_refuses_mixed_or_half_board_forms_and_has_no_token_flag() {
        assert!(
            parse(&[
                "connect",
                "jira",
                "--workspace",
                "p",
                "--board-url",
                "u",
                "--url",
                "s",
                "--board",
                "4",
                "--email",
                "e"
            ])
            .is_err()
        );
        assert!(
            parse(&["connect", "jira", "--workspace", "p", "--url", "s", "--email", "e"]).is_err(),
            "--url needs --board"
        );
        assert!(
            parse(&[
                "connect",
                "jira",
                "--workspace",
                "p",
                "--board-url",
                "u",
                "--email",
                "e",
                "--token",
                "t"
            ])
            .is_err(),
            "never argv"
        );
        assert!(
            parse(&["connect", "jira", "--workspace", "p", "--board-url", "u"]).is_err(),
            "email is required"
        );
    }

    #[test]
    fn sync_and_disconnect_take_a_workspace_and_optionally_a_board() {
        assert_eq!(
            parse(&["sync", "--workspace", "p"]).unwrap(),
            TrackerCmd::Sync { workspace: "p".into(), board: None }
        );
        assert_eq!(
            parse(&["disconnect", "--workspace", "p", "--board", "42"]).unwrap(),
            TrackerCmd::Disconnect { workspace: "p".into(), board: Some("42".into()) }
        );
    }

    fn trackers() -> Value {
        json!([
            {"id": "77777777-7777-7777-7777-777777777777", "level": "epic", "board_ref": "43", "board_name": "Portfolio"},
            {"id": "88888888-8888-8888-8888-888888888888", "level": "sprint", "board_ref": "42", "board_name": "AUTH Scrum"}
        ])
    }

    #[test]
    fn a_tracker_resolves_by_board_or_id_or_as_the_only_one() {
        assert_eq!(
            resolve_tracker(&trackers(), Some("42")),
            Ok("88888888-8888-8888-8888-888888888888".into())
        );
        assert_eq!(
            resolve_tracker(&trackers(), Some("77777777-7777-7777-7777-777777777777")),
            Ok("77777777-7777-7777-7777-777777777777".into())
        );
        let several = resolve_tracker(&trackers(), None).unwrap_err();
        assert!(
            several.contains("--board")
                && several.contains("epic #43")
                && several.contains("sprint #42"),
            "{several}"
        );
        let only = json!([trackers()[1].clone()]);
        assert_eq!(resolve_tracker(&only, None), Ok("88888888-8888-8888-8888-888888888888".into()));
        assert!(resolve_tracker(&trackers(), Some("99")).unwrap_err().contains("#42"));
        let two_sites = json!([
            {"id": "77777777-7777-7777-7777-777777777777", "level": "sprint", "board_ref": "42", "board_name": "A", "base_url": "https://a.atlassian.net"},
            {"id": "88888888-8888-8888-8888-888888888888", "level": "task", "board_ref": "42", "board_name": "B", "base_url": "https://b.atlassian.net"}
        ]);
        let ambiguous = resolve_tracker(&two_sites, Some("42")).unwrap_err();
        assert!(
            ambiguous.contains("a.atlassian.net")
                && ambiguous.contains("b.atlassian.net")
                && ambiguous.contains("7777"),
            "two boards numbered 42 are never guessed between: {ambiguous}"
        );
        assert_eq!(
            resolve_tracker(&json!([]), None).unwrap_err(),
            "This workspace has no tracker connected."
        );
    }

    fn workspaces() -> Value {
        json!([
            {"id": "11111111-1111-1111-1111-111111111111", "name": "got_a_guy"},
            {"id": "22222222-2222-2222-2222-222222222222", "name": "twin"},
            {"id": "33333333-3333-3333-3333-333333333333", "name": "twin"}
        ])
    }

    #[test]
    fn workspaces_resolve_by_exact_name_or_uuid() {
        assert_eq!(
            resolve_workspace(&workspaces(), "got_a_guy"),
            Ok("11111111-1111-1111-1111-111111111111".into())
        );
        assert_eq!(
            resolve_workspace(&workspaces(), "22222222-2222-2222-2222-222222222222"),
            Ok("22222222-2222-2222-2222-222222222222".into())
        );
        let none = resolve_workspace(&workspaces(), "got_a").unwrap_err();
        assert!(none.contains("no workspace") && none.contains("got_a_guy"), "{none}");
        let two = resolve_workspace(&workspaces(), "twin").unwrap_err();
        assert!(
            two.contains("2 workspaces") && two.contains("2222") && two.contains("3333"),
            "{two}"
        );
        assert!(
            resolve_workspace(&workspaces(), "99999999-9999-9999-9999-999999999999").is_err(),
            "an unknown uuid"
        );
    }

    #[test]
    fn the_connect_body_carries_the_chosen_board_form() {
        assert_eq!(
            jira_connect_body(
                Some("https://a.atlassian.net/x/boards/4"),
                None,
                None,
                "dev@example.com",
                None,
                "sprint",
                "tok"
            ),
            json!({"kind": "jira", "level": "sprint", "board_url": "https://a.atlassian.net/x/boards/4", "email": "dev@example.com", "api_token": "tok"})
        );
        assert_eq!(
            jira_connect_body(
                None,
                Some("https://a.atlassian.net"),
                Some("4"),
                "dev@example.com",
                Some("AUTH"),
                "epic",
                "tok"
            ),
            json!({"kind": "jira", "level": "epic", "base_url": "https://a.atlassian.net", "board_id": "4", "email": "dev@example.com", "api_token": "tok", "project_key": "AUTH"})
        );
    }

    #[test]
    fn the_token_comes_from_env_then_stdin() {
        let mut empty = std::io::Cursor::new(Vec::new());
        assert_eq!(read_token(Some("from-env".into()), &mut empty), Ok("from-env".into()));
        let mut input = std::io::Cursor::new(b"  from-stdin  \nignored\n".to_vec());
        assert_eq!(
            read_token(Some("".into()), &mut input),
            Ok("from-stdin".into()),
            "an empty env var falls through"
        );
        let mut nothing = std::io::Cursor::new(Vec::new());
        assert!(read_token(None, &mut nothing).is_err());
    }

    #[test]
    fn errors_are_described_in_words() {
        assert_eq!(
            describe_error(
                404,
                &json!({"error": "task_not_on_board", "key": "AUTH-404", "board_ref": "42"})
            ),
            "AUTH-404 isn't on board #42: it may be in another Jira project, or outside the board's filter."
        );
        assert_eq!(
            describe_error(404, &json!({"error": "task_not_found"})),
            "No ticket with that key in sensei."
        );
        assert!(
            describe_error(401, &json!({"error": "unauthorized"}))
                .contains("rejected the email and token")
        );
        let taken = describe_error(
            409,
            &json!({"error": "board_already_connected", "workspace": {"id": "x", "name": "other"}}),
        );
        assert!(taken.contains("other"), "{taken}");
        assert!(describe_error(404, &json!({"error": "board_not_found", "base_url": "https://a.atlassian.net", "board_ref": "4"})).contains("#4"));
        assert!(
            describe_error(422, &json!({"error": "invalid_level"}))
                .contains("epic, sprint or task")
        );
        assert!(describe_error(422, &json!({"error": "level_required"})).contains("--level"));
        assert!(describe_error(409, &json!({"error": "superseded"})).contains("Sync again"));
        assert_eq!(
            describe_error(404, &json!({"error": "tracker_not_found"})),
            "No such board in this workspace."
        );
        assert!(
            describe_error(500, &json!({"error": "mystery"})).contains("mystery"),
            "unknown codes are shown, not hidden"
        );
    }

    #[test]
    fn sprints_and_usage_parse() {
        assert_eq!(
            parse(&["sprints", "--workspace", "p"]).unwrap(),
            TrackerCmd::Sprints { workspace: "p".into(), board: None }
        );
        assert_eq!(
            parse(&["usage", "--workspace", "p", "AUTH-1", "--by-repo"]).unwrap(),
            TrackerCmd::Usage {
                workspace: "p".into(),
                board: None,
                sprint: None,
                key: Some("AUTH-1".into()),
                by_repo: true
            }
        );
        assert_eq!(
            parse(&["usage", "--workspace", "p", "--board", "42", "--sprint", "Sprint 2"]).unwrap(),
            TrackerCmd::Usage {
                workspace: "p".into(),
                board: Some("42".into()),
                sprint: Some("Sprint 2".into()),
                key: None,
                by_repo: false
            }
        );
        assert!(
            parse(&["usage", "--workspace", "p", "--sprint", "s", "AUTH-1"]).is_err(),
            "both is an error"
        );
        assert!(
            parse(&["usage", "--workspace", "p", "--by-repo"]).is_err(),
            "--by-repo needs a key"
        );
    }

    #[test]
    fn sprints_resolve_like_workspaces() {
        let sprints = json!([
            {"id": "44444444-4444-4444-4444-444444444444", "name": "Sprint 2"},
            {"id": "55555555-5555-5555-5555-555555555555", "name": "Sprint 1"},
            {"id": "66666666-6666-6666-6666-666666666666", "name": "Sprint 1"}
        ]);
        assert_eq!(
            resolve_sprint(&sprints, "Sprint 2"),
            Ok("44444444-4444-4444-4444-444444444444".into())
        );
        let none = resolve_sprint(&sprints, "Sprint 9").unwrap_err();
        assert!(none.contains("no sprint") && none.contains("Sprint 2"), "{none}");
        let two = resolve_sprint(&sprints, "Sprint 1").unwrap_err();
        assert!(two.contains("2 sprints") && two.contains("--sprint"), "{two}");
    }

    #[test]
    fn active_time_is_hours_and_minutes_and_blank_when_unknown() {
        assert_eq!(active_time(Some(5400.0)), "1:30");
        assert_eq!(active_time(Some(3599.0)), "0:59");
        assert_eq!(active_time(Some(36_000.0)), "10:00");
        assert_eq!(active_time(None), "");
    }

    #[test]
    fn usage_cells_leave_unknown_values_blank() {
        let usage = json!({
            "sessions": 3, "by_source": {"branch": 3, "prompt": 1},
            "active_secs": 180.0, "active_n": 2,
            "tokens_fresh": 15, "tokens_fresh_n": 2,
            "cache_read": null, "cache_read_n": 0,
            "tokens_out": 40, "tokens_out_n": 1,
            "metered_cost": null, "metered_cost_n": 0
        });
        assert_eq!(
            usage_cells("AUTH-2", "Login", &usage),
            vec!["AUTH-2", "Login", "3 (3/1)", "0:03", "15", "", "40", ""]
        );
        assert!(USAGE_HEADER.contains(&"active AI time"), "never just \"hours\"");
        let priced = json!({"sessions": 1, "by_source": {"branch": 1, "prompt": 0}, "metered_cost": 1.5, "metered_cost_n": 1});
        assert_eq!(usage_cells("K-1", "", &priced)[7], "1.50");
    }
}
