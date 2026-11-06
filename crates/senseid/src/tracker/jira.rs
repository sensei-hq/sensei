//! The Jira Cloud adapter. All Jira knowledge — host rule, board-URL shapes,
//! endpoints, auth, ticket-key matching — lives in this module. (Spec R4, R6.)

use crate::tracker::http::{InvalidBaseUrl, normalize_base_url};
use crate::tracker::sprint::SprintState;
use crate::tracker::{
    AccountRef, ApiToken, AuthScheme, BoardScope, ConnectParams, Discovered, FieldCheck,
    FieldValue, GetTransport, KnownTask, Link, LinkSource, OwnerLookup, ParamError, PrevState,
    SessionText, Snapshot, SprintRow, TaskRow, TrackerAdapter, TrackerError, TrackerKind,
    ValidateError, Validated, WriteMethod, WriteTransport,
};

/// Jira Cloud's site domains: `atlassian.net`, and `jira.com` for older sites.
const JIRA_CLOUD_DOMAINS: [&str; 2] = [".atlassian.net", ".jira.com"];

/// Jira Cloud's host rule: `<label>.atlassian.net` or `<label>.jira.com`, at
/// least one label before the domain.
pub fn is_jira_cloud_host(host: &str) -> bool {
    JIRA_CLOUD_DOMAINS.iter().any(|domain| {
        host.strip_suffix(domain)
            .is_some_and(|labels| !labels.is_empty() && labels.split('.').all(|l| !l.is_empty()))
    })
}

/// A board id in canonical decimal form (`"042"` → `"42"`); `None` for zero or
/// anything that is not a `u64`.
pub fn canonical_board_id(raw: &str) -> Option<String> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok().filter(|id| *id > 0).map(|id| id.to_string())
}

/// Split a pasted board address into `(base_url, board_id)`.
///
/// Accepts `…/boards/<id>[/anything]` and `…/RapidBoard.jspa?…rapidView=<id>…`.
/// The board id is read before the query is dropped; the origin goes through the
/// shared [`normalize_base_url`] with the Jira host rule.
pub fn parse_board_url(raw: &str) -> Result<(String, String), InvalidBaseUrl> {
    let url = reqwest::Url::parse(raw.trim()).map_err(|_| InvalidBaseUrl)?;
    let board_id = board_id_in(&url).ok_or(InvalidBaseUrl)?;
    let base_url = normalize_base_url(raw, is_jira_cloud_host)?;
    Ok((base_url, board_id))
}

/// The board id a Jira board address carries: the segment after `/boards/`, or
/// the classic `RapidBoard.jspa?rapidView=<id>` parameter.
fn board_id_in(url: &reqwest::Url) -> Option<String> {
    let segments: Vec<&str> = url.path_segments()?.collect();
    if let Some(i) = segments.iter().position(|s| *s == "boards") {
        return segments.get(i + 1).and_then(|s| canonical_board_id(s));
    }
    if segments.last() == Some(&"RapidBoard.jspa") {
        return url
            .query_pairs()
            .find(|(k, _)| k == "rapidView")
            .and_then(|(_, v)| canonical_board_id(&v));
    }
    None
}

/// The Jira Cloud adapter (Agile REST 1.0 + `/rest/api/3/myself`).
pub struct JiraAdapter;

/// Page size for issue listings.
pub const ISSUE_PAGE: u32 = 100;
/// Page size for sprint listings.
pub const SPRINT_PAGE: u32 = 50;

#[async_trait::async_trait]
impl TrackerAdapter for JiraAdapter {
    fn kind(&self) -> TrackerKind {
        TrackerKind::Jira
    }

    fn capabilities(&self) -> crate::tracker::Capabilities {
        crate::tracker::Capabilities { history: true, writeback: true }
    }

    fn connect_params(&self, body: &serde_json::Value) -> Result<ConnectParams, ParamError> {
        let text =
            |k: &str| body.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|v| !v.is_empty());
        let board_id_field = body.get("board_id").and_then(|v| match v {
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        });
        let separate = text("base_url").is_some() || board_id_field.is_some();
        let (base_url, board_ref) = match (text("board_url"), separate) {
            (Some(url), false) => parse_board_url(url).map_err(|_| ParamError::InvalidBaseUrl)?,
            (None, true) => {
                let base = text("base_url").ok_or(ParamError::InvalidBaseUrl)?;
                let base = normalize_base_url(base, is_jira_cloud_host)
                    .map_err(|_| ParamError::InvalidBaseUrl)?;
                let board = board_id_field
                    .as_deref()
                    .and_then(canonical_board_id)
                    .ok_or(ParamError::InvalidBaseUrl)?;
                (base, board)
            }
            // Both forms, or neither: refuse rather than guess which was meant.
            _ => return Err(ParamError::InvalidBaseUrl),
        };
        let account = text("email").ok_or(ParamError::EmailRequired)?.to_string();
        let token = text("api_token").ok_or(ParamError::TokenRequired)?.to_string();
        Ok(ConnectParams {
            base_url,
            board_ref,
            account,
            token: ApiToken(token),
            project_key: text("project_key").map(str::to_uppercase),
        })
    }

    fn auth(&self, account: &str, token: &ApiToken) -> AuthScheme {
        AuthScheme::Basic { user: account.to_string(), token: token.clone() }
    }

    async fn owner(&self, t: &dyn GetTransport) -> Result<OwnerLookup, TrackerError> {
        t.get_json(MYSELF, &[]).await.map(|v| owner_from_myself(&v))
    }

    async fn find_comment(
        &self,
        t: &dyn GetTransport,
        task_external_id: &str,
        owner: &AccountRef,
    ) -> Result<Option<String>, TrackerError> {
        let path = format!("{}/comment", issue_path(task_external_id)?);
        let query = [("orderBy", "-created".to_string()), ("maxResults", "50".to_string())];
        let page = t.get_json(&path, &query).await?;
        let comments =
            page.get("comments").and_then(|c| c.as_array()).ok_or(TrackerError::Decode)?;
        let is_report = |c: &serde_json::Value| {
            c.pointer("/body/content/0/content/0/text").and_then(|x| x.as_str()).is_some_and(
                |text| text.starts_with(crate::tracker::writeback::REPORT_TITLE_PREFIX),
            )
        };
        Ok(comments
            .iter()
            .find(|c| {
                c.pointer("/author/accountId").and_then(|a| a.as_str()) == Some(owner.as_str())
                    && is_report(c)
            })
            .and_then(|c| c.get("id").and_then(|id| id.as_str()).map(String::from)))
    }

    async fn create_comment(
        &self,
        w: &dyn WriteTransport,
        task_external_id: &str,
        report: &crate::tracker::writeback::Report,
    ) -> Result<String, TrackerError> {
        let path = format!("{}/comment", issue_path(task_external_id)?);
        let created = w
            .send_json(WriteMethod::Post, &path, &[], &serde_json::json!({"body": adf(report)}))
            .await?;
        created.get("id").and_then(|id| id.as_str()).map(String::from).ok_or(TrackerError::Decode)
    }

    async fn update_comment(
        &self,
        w: &dyn WriteTransport,
        task_external_id: &str,
        comment_ref: &str,
        report: &crate::tracker::writeback::Report,
    ) -> Result<(), TrackerError> {
        if !is_jira_id(comment_ref) {
            return Err(TrackerError::BadRequest);
        }
        let path = format!("{}/comment/{comment_ref}", issue_path(task_external_id)?);
        let query = [("notifyUsers", "false".to_string())];
        w.send_json(WriteMethod::Put, &path, &query, &serde_json::json!({"body": adf(report)}))
            .await
            .map(|_| ())
    }

    fn field_ref_shape(&self, field_ref: &str) -> bool {
        field_number(field_ref).is_some()
    }

    async fn check_fields(
        &self,
        t: &dyn GetTransport,
        field_refs: &[String],
    ) -> Result<Vec<FieldCheck>, TrackerError> {
        let all = t.get_json("/rest/api/3/field", &[]).await?;
        let all = all.as_array().ok_or(TrackerError::Decode)?;
        Ok(field_refs
            .iter()
            .map(|r| {
                let found = all.iter().find(|f| f.get("id").and_then(|i| i.as_str()) == Some(r));
                FieldCheck {
                    field_ref: r.clone(),
                    exists: found.is_some(),
                    numeric: found
                        .and_then(|f| f.pointer("/schema/type"))
                        .and_then(|kind| kind.as_str())
                        == Some("number"),
                }
            })
            .collect())
    }

    async fn field_in_use(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        field_ref: &str,
    ) -> Result<bool, TrackerError> {
        // Only a validated field number reaches the JQL string.
        let n = field_number(field_ref).ok_or(TrackerError::BadRequest)?;
        let query = [
            ("jql", format!("cf[{n}] is not EMPTY")),
            ("maxResults", "1".to_string()),
            ("fields", "none".to_string()),
        ];
        let page = t.get_json(&format!("{}/issue", board_path(&board.board_ref)), &query).await?;
        let issues = page.get("issues").and_then(|i| i.as_array()).ok_or(TrackerError::Decode)?;
        Ok(!issues.is_empty())
    }

    async fn write_fields(
        &self,
        w: &dyn WriteTransport,
        task_external_id: &str,
        fields: &[(String, FieldValue)],
    ) -> Result<(), TrackerError> {
        // Refused before any request, as `Decode`: a Jira 400 is the only `field_invalid` (W6).
        if !is_jira_id(task_external_id) || !fields.iter().all(|(r, _)| field_number(r).is_some()) {
            return Err(TrackerError::Decode);
        }
        let body: serde_json::Map<String, serde_json::Value> =
            fields.iter().map(|(r, v)| (r.clone(), v.to_json())).collect();
        w.send_json(
            WriteMethod::Put,
            &format!("/rest/api/3/issue/{task_external_id}"),
            &[],
            &serde_json::json!({"fields": body}),
        )
        .await
        .map(|_| ())
    }

    async fn issue_exists(
        &self,
        t: &dyn GetTransport,
        task_external_id: &str,
    ) -> Result<bool, TrackerError> {
        match t.get_json(&issue_path(task_external_id)?, &[("fields", "none".to_string())]).await {
            Ok(_) => Ok(true),
            Err(TrackerError::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn validate(
        &self,
        t: &dyn GetTransport,
        p: &ConnectParams,
    ) -> Result<Validated, ValidateError> {
        let owner = self.owner(t).await.map_err(ValidateError::Tracker)?;
        let board = match t.get_json(&board_path(&p.board_ref), &[]).await {
            Ok(b) => b,
            Err(TrackerError::NotFound) => return Err(ValidateError::BoardNotFound),
            Err(e) => return Err(ValidateError::Tracker(e)),
        };
        let board_name = board
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(ValidateError::Tracker(TrackerError::Decode))?;
        let board_project = board.pointer("/location/projectKey").and_then(|v| v.as_str());
        let mismatch = p
            .project_key
            .as_deref()
            .is_some_and(|want| board_project.map(str::to_uppercase).as_deref() != Some(want));
        if mismatch {
            return Err(ValidateError::BoardProjectMismatch);
        }
        Ok(Validated {
            // A board spanning several projects has no single key; its name is the scope shown.
            external_scope: board_project.unwrap_or(board_name).to_string(),
            board_name: board_name.to_string(),
            owner,
            // Jira keeps the connect body's email and typed board id.
            account: None,
            board_ref: None,
            params: Default::default(),
        })
    }

    async fn fetch_snapshot(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        prev: &PrevState,
    ) -> Result<Snapshot, TrackerError> {
        let base = board_path(&board.board_ref);
        let board_name =
            t.get_json(&base, &[]).await?.get("name").and_then(|v| v.as_str()).map(String::from);

        let sprints: Vec<SprintRow> =
            paged(t, &format!("{base}/sprint"), "values", SPRINT_PAGE, &[])
                .await?
                .iter()
                .map(sprint_row)
                .collect::<Result<_, _>>()?;

        // S4: created/updated always; the story-point field once the board's is known.
        let estimation = match &prev.estimation {
            crate::tracker::Estimation::Field(f) => Some(f.as_str()),
            _ => None,
        };
        let mut fields = format!("{},created,updated", ISSUE_FIELDS[0].1);
        if let Some(f) = estimation {
            fields.push(',');
            fields.push_str(f);
        }
        let tasks: Vec<TaskRow> =
            paged(t, &format!("{base}/issue"), "issues", ISSUE_PAGE, &[("fields", &fields)])
                .await?
                .iter()
                .map(|v| task_row_with(v, estimation))
                .collect::<Result<_, _>>()?;
        // A board learned to have no estimation field clears every ticket's points.
        let tasks = if prev.estimation == crate::tracker::Estimation::NoField {
            tasks.into_iter().map(|t| TaskRow { points: Some(None), ..t }).collect()
        } else {
            tasks
        };

        let mut members = Vec::with_capacity(sprints.len());
        for sprint in &sprints {
            let fetched_closed = prev.members_fetched_state.get(&sprint.external_id)
                == Some(&Some(SprintState::Closed));
            if sprint.state == SprintState::Closed && fetched_closed {
                members.push((sprint.external_id.clone(), None));
                continue;
            }
            let path = format!("{base}/sprint/{}/issue", sprint.external_id);
            let ids = paged(t, &path, "issues", ISSUE_PAGE, &[("fields", "summary")])
                .await?
                .iter()
                .map(issue_id)
                .collect::<Result<Vec<_>, _>>()?;
            members.push((sprint.external_id.clone(), Some(ids)));
        }
        let points_field = match &prev.estimation {
            crate::tracker::Estimation::NotLearned => None,
            crate::tracker::Estimation::NoField => Some("none".to_string()),
            crate::tracker::Estimation::Field(f) => Some(f.clone()),
        };
        Ok(Snapshot {
            board_name,
            sprints,
            tasks,
            members,
            points_field,
            params_extra: Default::default(),
        })
    }

    async fn board_terms(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
    ) -> Result<crate::tracker::BoardTerms, TrackerError> {
        let config =
            t.get_json(&format!("{}/configuration", board_path(&board.board_ref)), &[]).await?;
        let estimation_field = (config.pointer("/estimation/type").and_then(|x| x.as_str())
            == Some("field"))
        .then(|| config.pointer("/estimation/field/fieldId").and_then(|x| x.as_str()))
        .flatten()
        .filter(|f| field_number(f).is_some())
        .map(String::from);
        let fields = t.get_json("/rest/api/3/field", &[]).await?;
        let sprint_field = fields
            .as_array()
            .ok_or(TrackerError::Decode)?
            .iter()
            .find(|f| {
                f.pointer("/schema/custom").and_then(|x| x.as_str())
                    == Some("com.pyxis.greenhopper.jira:gh-sprint")
            })
            .and_then(|f| f.get("id").and_then(|x| x.as_str()))
            .map(String::from);
        let statuses = t
            .get_json("/rest/api/3/status", &[])
            .await?
            .as_array()
            .ok_or(TrackerError::Decode)?
            .iter()
            .filter_map(|st| {
                let text = |p: &str| st.pointer(p).and_then(|x| x.as_str()).map(String::from);
                Some((text("/id")?, text("/name")?, text("/statusCategory/key")?))
            })
            .collect();
        Ok(crate::tracker::BoardTerms { estimation_field, sprint_field, statuses })
    }

    async fn changelog(
        &self,
        t: &dyn GetTransport,
        issue_external_id: &str,
        sprint_field: Option<&str>,
        estimation_field: Option<&str>,
    ) -> Result<Vec<crate::tracker::ChangeRow>, TrackerError> {
        let path = format!("{}/changelog", issue_path(issue_external_id)?);
        let mut out = Vec::new();
        for history in paged(t, &path, "values", CHANGELOG_PAGE, &[]).await? {
            let change_id =
                history.get("id").and_then(|x| x.as_str()).ok_or(TrackerError::Decode)?.to_string();
            let at = history
                .get("created")
                .and_then(|x| x.as_str())
                .and_then(jira_instant)
                .ok_or(TrackerError::Decode)?;
            for item in history.get("items").and_then(|x| x.as_array()).into_iter().flatten() {
                let id = item.get("fieldId").and_then(|x| x.as_str());
                let text = |k: &str| {
                    item.get(k).and_then(|x| x.as_str()).filter(|v| !v.is_empty()).map(String::from)
                };
                // Sprint and status keep ids; the estimate keeps its points as text (A6).
                let (field, from, to) = if id.is_some() && id == sprint_field {
                    ("sprint", text("from"), text("to"))
                } else if id == Some("status") {
                    ("status", text("from"), text("to"))
                } else if id.is_some() && id == estimation_field {
                    ("estimate", text("fromString"), text("toString"))
                } else {
                    continue;
                };
                out.push(crate::tracker::ChangeRow {
                    change_id: change_id.clone(),
                    field,
                    at,
                    from_value: from,
                    to_value: to,
                });
            }
        }
        Ok(out)
    }

    fn match_links(&self, session: &SessionText, known: &[KnownTask]) -> Vec<Link> {
        match_keys(session, known)
    }

    fn unknown_keys(&self, session: &SessionText, known: &[KnownTask]) -> Vec<String> {
        unknown_keys_in(session, known)
    }

    async fn fetch_tasks(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        keys: &[String],
    ) -> Result<Discovered, TrackerError> {
        fetch_keys(t, board, keys).await
    }
}

/// Jira's `match_links` (spec R6): keys of `known` found in the session's branches
/// and prompts, matched on the original bytes, ASCII case-insensitively.
pub fn match_keys(session: &SessionText, known: &[KnownTask]) -> Vec<Link> {
    use std::collections::HashMap;
    let mut by_key: HashMap<&str, Vec<&KnownTask>> = HashMap::new();
    for k in known {
        by_key.entry(k.key.as_str()).or_default().push(k);
    }
    let prefixes = key_prefixes(known);
    // The (task, alias) pairs a text carries, one per occurrence. Jira keys read the
    // same in a branch and a prompt.
    let tasks_in = |text: &str, _source: LinkSource| -> Vec<(&str, String)> {
        keys_in(text.as_bytes(), &prefixes)
            .into_iter()
            .flat_map(|key| {
                by_key
                    .get(key.as_str())
                    .into_iter()
                    .flatten()
                    .map(move |t| (t.external_id.as_str(), key.clone()))
            })
            .collect()
    };
    crate::tracker::link::tally_links(session, known, tasks_in)
}

/// The project-key prefixes of `known`, longest first (the scan order).
fn key_prefixes(known: &[KnownTask]) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut prefixes: Vec<String> = known
        .iter()
        .filter_map(|k| k.key.rsplit_once('-').map(|(p, _)| p.to_ascii_uppercase()))
        .filter(|p| !p.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    prefixes.sort_by_key(|p| std::cmp::Reverse(p.len()));
    prefixes
}

/// R11: keys in the session's branches and prompts that use a prefix the board
/// already has but match no known key. Sorted, once each, shape-checked.
pub fn unknown_keys_in(session: &SessionText, known: &[KnownTask]) -> Vec<String> {
    use std::collections::BTreeSet;
    let prefixes = key_prefixes(known);
    let have: BTreeSet<&str> = known.iter().map(|k| k.key.as_str()).collect();
    session
        .branches
        .iter()
        .chain(&session.prompts)
        .flat_map(|text| keys_in(text.as_bytes(), &prefixes))
        .filter(|k| !have.contains(k.as_str()) && is_jira_key(k))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// One board-scoped search for these (shape-checked) keys.
async fn search_keys(
    t: &dyn GetTransport,
    board: &BoardScope,
    keys: &[String],
) -> Result<Discovered, TrackerError> {
    let query = [
        ("jql", format!("key in ({})", keys.join(","))),
        ("fields", SEARCH_FIELDS.to_string()),
        ("maxResults", crate::tracker::discover::DISCOVER_MAX_KEYS.to_string()),
    ];
    let page = t.get_json(&format!("{}/issue", board_path(&board.board_ref)), &query).await?;
    let issues = page.get("issues").and_then(|v| v.as_array()).ok_or(TrackerError::Decode)?;
    let mut out = Discovered::default();
    for v in issues {
        let task = task_row_with(v, None)?;
        let sprint_ids = v.pointer("/fields/sprint/id").into_iter().chain(
            v.pointer("/fields/closedSprints")
                .and_then(|c| c.as_array())
                .into_iter()
                .flatten()
                .filter_map(|s| s.get("id")),
        );
        for id in sprint_ids {
            let id = id.as_u64().map(|n| n.to_string()).ok_or(TrackerError::Decode)?;
            out.members.push((id, task.external_id.clone()));
        }
        out.tasks.push(task);
    }
    Ok(out)
}

/// R11 fetch: one search for all keys; when Jira rejects it (a key that exists
/// nowhere fails the whole `key in` query, claim C33), one search per key, a
/// rejected key being one the board does not have. Any other failure fails.
pub async fn fetch_keys(
    t: &dyn GetTransport,
    board: &BoardScope,
    keys: &[String],
) -> Result<Discovered, TrackerError> {
    let mut keys: Vec<String> = keys.iter().filter(|k| is_jira_key(k)).cloned().collect();
    keys.sort();
    keys.dedup();
    if keys.is_empty() {
        return Ok(Discovered::default());
    }
    match search_keys(t, board, &keys).await {
        Err(TrackerError::BadRequest) if keys.len() > 1 => {
            let mut out = Discovered::default();
            for key in &keys {
                match search_keys(t, board, std::slice::from_ref(key)).await {
                    Ok(d) => {
                        out.tasks.extend(d.tasks);
                        out.members.extend(d.members);
                    }
                    Err(TrackerError::BadRequest) => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(out)
        }
        Err(TrackerError::BadRequest) => Ok(Discovered::default()),
        other => other,
    }
}

/// Every key shape `PREFIX-digits` in `text`, scanned left to right with prefixes
/// longest first; a key must not follow an ASCII letter or digit, and a matched
/// span is consumed. Comparison is ASCII-only, on the original bytes.
fn keys_in(text: &[u8], prefixes: &[String]) -> Vec<String> {
    let mut keys = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if i > 0 && text[i - 1].is_ascii_alphanumeric() {
            i += 1;
            continue;
        }
        let hit = prefixes.iter().find_map(|p| {
            let p_end = i + p.len();
            let rest = text.get(i..p_end)?;
            if !rest.eq_ignore_ascii_case(p.as_bytes()) || text.get(p_end) != Some(&b'-') {
                return None;
            }
            let digits_start = p_end + 1;
            let digits = text[digits_start..].iter().take_while(|b| b.is_ascii_digit()).count();
            (digits > 0).then(|| {
                let end = digits_start + digits;
                (format!("{p}-{}", String::from_utf8_lossy(&text[digits_start..end])), end)
            })
        });
        match hit {
            Some((key, end)) => {
                keys.push(key);
                i = end;
            }
            None => i += 1,
        }
    }
    keys
}

const ISSUE_FIELDS: [(&str, &str); 1] = [("fields", "summary,issuetype,status,assignee")];
/// The fields R11's board search asks for.
/// Changelog page size (S4): Jira's maximum for the endpoint.
const CHANGELOG_PAGE: u32 = 100;
const SEARCH_FIELDS: &str =
    "summary,issuetype,status,assignee,sprint,closedSprints,created,updated";
/// Who the token belongs to (W1).
const MYSELF: &str = "/rest/api/3/myself";
/// A listing that never signals its end is a provider fault, not a reason to loop.
const MAX_PAGES: usize = 1000;

/// Jira's key shape: an upper-case project key (letters, digits, `_`, starting with
/// a letter), `-`, then digits. Only keys passing this reach a JQL string (R11).
pub fn is_jira_key(key: &str) -> bool {
    let Some((prefix, number)) = key.split_once('-') else { return false };
    let mut chars = prefix.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
}

fn board_path(board_ref: &str) -> String {
    format!("/rest/agile/1.0/board/{board_ref}")
}

/// Every item of a `startAt`-paged listing. Stops on `isLast`, on
/// `startAt >= total`, or on an empty page.
async fn paged(
    t: &dyn GetTransport,
    path: &str,
    items_key: &str,
    page_size: u32,
    extra: &[(&str, &str)],
) -> Result<Vec<serde_json::Value>, TrackerError> {
    let mut out = Vec::new();
    let mut start: u64 = 0;
    for _ in 0..MAX_PAGES {
        let mut query: Vec<(&str, String)> =
            vec![("startAt", start.to_string()), ("maxResults", page_size.to_string())];
        query.extend(extra.iter().map(|(k, v)| (*k, v.to_string())));
        let page = t.get_json(path, &query).await?;
        let items = page.get(items_key).and_then(|v| v.as_array()).ok_or(TrackerError::Decode)?;
        if items.is_empty() {
            return Ok(out);
        }
        start += items.len() as u64;
        out.extend(items.iter().cloned());
        let last = page.get("isLast").and_then(|v| v.as_bool()) == Some(true);
        let past_total =
            page.get("total").and_then(|v| v.as_u64()).is_some_and(|total| start >= total);
        if last || past_total {
            return Ok(out);
        }
    }
    Err(TrackerError::Decode)
}

/// A Jira instant: `2021-01-17T12:34:00.000+0000` (an offset without a colon, which
/// RFC 3339 parsers reject), or RFC 3339 (S4, A6).
fn jira_instant(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z")
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(s))
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

/// A field's value as text (S4): a number's decimal text, a string as it is.
fn value_text(v: Option<&serde_json::Value>) -> Option<String> {
    match v? {
        serde_json::Value::Number(n) => {
            Some(n.as_f64().map_or_else(|| n.to_string(), |f| format!("{f}")))
        }
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// A ticket row, asking for `estimation_field` when the board's is known (S4).
fn task_row_with(
    v: &serde_json::Value,
    estimation_field: Option<&str>,
) -> Result<TaskRow, TrackerError> {
    const CATEGORIES: [&str; 4] = ["new", "indeterminate", "done", "undefined"];
    let instant = |path: &str| v.pointer(path).and_then(|x| x.as_str()).and_then(jira_instant);
    Ok(TaskRow {
        created_at: instant("/fields/created"),
        updated_at: instant("/fields/updated"),
        status_category: v
            .pointer("/fields/status/statusCategory/key")
            .and_then(|x| x.as_str())
            .filter(|k| CATEGORIES.contains(k))
            .map(String::from),
        points: estimation_field.map(|f| value_text(v.get("fields").and_then(|fs| fs.get(f)))),
        ..task_row(v)?
    })
}

fn timestamp(
    v: &serde_json::Value,
    key: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, TrackerError> {
    match v.get(key).and_then(|x| x.as_str()) {
        None => Ok(None),
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| Some(d.with_timezone(&chrono::Utc)))
            .map_err(|_| TrackerError::Decode),
    }
}

fn sprint_row(v: &serde_json::Value) -> Result<SprintRow, TrackerError> {
    let id = v.get("id").and_then(|x| x.as_i64()).ok_or(TrackerError::Decode)?;
    let state = v
        .get("state")
        .and_then(|x| x.as_str())
        .and_then(SprintState::parse)
        .ok_or(TrackerError::Decode)?;
    Ok(SprintRow {
        external_id: id.to_string(),
        seq: id,
        name: v.get("name").and_then(|x| x.as_str()).ok_or(TrackerError::Decode)?.to_string(),
        state,
        start_at: timestamp(v, "startDate")?,
        end_at: timestamp(v, "endDate")?,
        complete_at: timestamp(v, "completeDate")?,
    })
}

fn issue_id(v: &serde_json::Value) -> Result<String, TrackerError> {
    v.get("id").and_then(|x| x.as_str()).map(String::from).ok_or(TrackerError::Decode)
}

/// Who `/myself` says the token belongs to (spec W1): the rules in order, first
/// match wins. Only `NotAPerson` is final; `Unknown` is asked again.
pub fn owner_from_myself(v: &serde_json::Value) -> OwnerLookup {
    let text = |k: &str| v.get(k).and_then(|x| x.as_str());
    match (text("accountType").map(str::to_lowercase), text("accountId")) {
        (Some(t), _) if t != "atlassian" => OwnerLookup::NotAPerson,
        (Some(_), Some(id)) if !id.is_empty() => OwnerLookup::Person(AccountRef::new(id)),
        _ => OwnerLookup::Unknown,
    }
}

/// The report as an Atlassian Document: the title in bold, then one bullet per
/// line. Every string is a `text` node built by serde, never interpolated (W5).
pub fn adf(report: &crate::tracker::writeback::Report) -> serde_json::Value {
    use serde_json::json;
    let text = |t: &str| json!({"type": "text", "text": t});
    let mut title = text(&report.title);
    title["marks"] = json!([{"type": "strong"}]);
    let items: Vec<_> = report
        .lines
        .iter()
        .map(|l| json!({"type": "listItem", "content": [{"type": "paragraph", "content": [text(l)]}]}))
        .collect();
    json!({"type": "doc", "version": 1, "content": [
        {"type": "paragraph", "content": [title]},
        {"type": "bulletList", "content": items},
    ]})
}

/// Jira's numeric issue and comment ids (W10 path safety): only these reach a path.
pub fn is_jira_id(id: &str) -> bool {
    (1..=18).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit())
}

/// The number of a Jira custom field id `customfield_<n>` (`n` from 1, up to ten
/// digits, no leading zero), or `None` for anything else.
fn field_number(field_ref: &str) -> Option<&str> {
    let n = field_ref.strip_prefix("customfield_")?;
    let ok =
        (1..=10).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()) && !n.starts_with('0');
    ok.then_some(n)
}

/// `/rest/api/3/issue/{id}`, only for an id that passes [`is_jira_id`].
fn issue_path(task_external_id: &str) -> Result<String, TrackerError> {
    if !is_jira_id(task_external_id) {
        return Err(TrackerError::BadRequest);
    }
    Ok(format!("/rest/api/3/issue/{task_external_id}"))
}

fn task_row(v: &serde_json::Value) -> Result<TaskRow, TrackerError> {
    let field = |path: &str| v.pointer(path).and_then(|x| x.as_str()).map(String::from);
    Ok(TaskRow {
        external_id: issue_id(v)?,
        key: v.get("key").and_then(|x| x.as_str()).ok_or(TrackerError::Decode)?.to_uppercase(),
        scope: None,
        summary: field("/fields/summary"),
        issue_type: field("/fields/issuetype/name"),
        status: field("/fields/status/name"),
        assignee: field("/fields/assignee/accountId")
            .filter(|a| !a.is_empty())
            .map(AccountRef::new),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jira_cloud_hosts() {
        assert!(is_jira_cloud_host("acme.atlassian.net"));
        assert!(is_jira_cloud_host("my-team.atlassian.net"));
        assert!(is_jira_cloud_host("senecaglobal.jira.com"), "older Cloud sites live on jira.com");
        for h in [
            "jira.com",
            ".jira.com",
            "acme.jira.com.evil.com",
            "acme.jira.com.",
            "eviljira.com",
            "acme.jira.net",
            "atlassian.net",
            ".atlassian.net",
            "acme.atlassian.net.evil.com",
            "acme.atlassian.net.",
            "evilatlassian.net",
            "acme.atlassian.com",
        ] {
            assert!(!is_jira_cloud_host(h), "{h}");
        }
    }

    #[test]
    fn board_ids_are_canonical_decimal() {
        assert_eq!(canonical_board_id("42"), Some("42".into()));
        assert_eq!(canonical_board_id("042"), Some("42".into()));
        for bad in ["0", "000", "-1", "4 2", "", "x", "18446744073709551616"] {
            assert_eq!(canonical_board_id(bad), None, "{bad:?}");
        }
    }

    const SITE: &str = "https://acme.atlassian.net";

    #[test]
    fn modern_board_urls() {
        for (raw, id) in [
            ("https://acme.atlassian.net/jira/software/projects/AUTH/boards/42", "42"),
            ("https://acme.atlassian.net/jira/software/projects/AUTH/boards/042", "42"),
            (
                "https://acme.atlassian.net/jira/software/projects/AUTH/boards/42/backlog?selectedIssue=AUTH-1",
                "42",
            ),
            ("https://ACME.atlassian.net/jira/software/c/projects/AUTH/boards/7#x", "7"),
        ] {
            assert_eq!(parse_board_url(raw), Ok((SITE.into(), id.into())), "{raw}");
        }
        assert_eq!(
            parse_board_url(
                "https://senecaglobal.jira.com/jira/software/c/projects/P057542601/boards/333"
            ),
            Ok(("https://senecaglobal.jira.com".into(), "333".into()))
        );
    }

    #[test]
    fn classic_rapid_board_urls() {
        for (raw, id) in [
            ("https://acme.atlassian.net/secure/RapidBoard.jspa?rapidView=7", "7"),
            ("https://acme.atlassian.net/secure/RapidBoard.jspa?view=planning&rapidView=7", "7"),
        ] {
            assert_eq!(parse_board_url(raw), Ok((SITE.into(), id.into())), "{raw}");
        }
    }

    #[test]
    fn board_urls_without_a_usable_board_are_rejected() {
        for raw in [
            "https://acme.atlassian.net/jira/software/projects/AUTH",
            "https://acme.atlassian.net/jira/software/projects/AUTH/boards/0",
            "https://acme.atlassian.net/jira/software/projects/AUTH/boards/",
            "https://acme.atlassian.net/secure/RapidBoard.jspa?view=planning",
            "https://evil.com/jira/software/projects/AUTH/boards/42",
            "http://acme.atlassian.net/jira/software/projects/AUTH/boards/42",
        ] {
            assert_eq!(parse_board_url(raw), Err(InvalidBaseUrl), "{raw}");
        }
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use crate::tracker::sprint::SprintState;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    fn params(body: serde_json::Value) -> Result<ConnectParams, ParamError> {
        JiraAdapter.connect_params(&body)
    }

    #[test]
    fn connect_params_from_a_board_url() {
        let p = params(json!({
            "board_url": "https://acme.atlassian.net/jira/software/projects/AUTH/boards/042",
            "email": "dev@example.com", "api_token": "t0k"
        }))
        .unwrap();
        assert_eq!(
            (p.base_url.as_str(), p.board_ref.as_str()),
            ("https://acme.atlassian.net", "42")
        );
        assert_eq!((p.account.as_str(), p.project_key), ("dev@example.com", None));
        assert_eq!(p.token, ApiToken("t0k".into()));
    }

    #[test]
    fn connect_params_from_separate_fields() {
        let p = params(json!({
            "base_url": "https://ACME.atlassian.net/", "board_id": "042",
            "email": "dev@example.com", "api_token": "t0k", "project_key": "auth"
        }))
        .unwrap();
        assert_eq!(
            (p.base_url.as_str(), p.board_ref.as_str()),
            ("https://acme.atlassian.net", "42")
        );
        assert_eq!(p.project_key.as_deref(), Some("AUTH"));
        let numeric = params(json!({
            "base_url": "https://acme.atlassian.net", "board_id": 7, "email": "e@x.io", "api_token": "t"
        }))
        .unwrap();
        assert_eq!(numeric.board_ref, "7");
    }

    #[test]
    fn connect_params_refusals() {
        let url = "https://acme.atlassian.net/jira/software/projects/AUTH/boards/42";
        let both = json!({"board_url": url, "base_url": "https://acme.atlassian.net", "board_id": 42,
                          "email": "e@x.io", "api_token": "t"});
        assert_eq!(params(both), Err(ParamError::InvalidBaseUrl), "both forms present");
        assert_eq!(
            params(json!({"email": "e@x.io", "api_token": "t"})),
            Err(ParamError::InvalidBaseUrl),
            "neither"
        );
        assert_eq!(
            params(
                json!({"base_url": "https://acme.atlassian.net", "board_id": 0, "email": "e@x.io", "api_token": "t"})
            ),
            Err(ParamError::InvalidBaseUrl),
            "board 0"
        );
        assert_eq!(
            params(json!({"board_url": url, "api_token": "t"})),
            Err(ParamError::EmailRequired)
        );
        assert_eq!(
            params(json!({"board_url": url, "email": "  ", "api_token": "t"})),
            Err(ParamError::EmailRequired)
        );
        assert_eq!(
            params(json!({"board_url": url, "email": "e@x.io"})),
            Err(ParamError::TokenRequired)
        );
        assert_eq!(
            params(json!({"board_url": url, "email": "e@x.io", "api_token": ""})),
            Err(ParamError::TokenRequired)
        );
    }

    #[test]
    fn the_token_never_appears_in_debug_output() {
        let p = params(json!({
            "board_url": "https://acme.atlassian.net/jira/software/projects/AUTH/boards/42",
            "email": "dev@example.com", "api_token": "SENTINEL-TOKEN-XYZ"
        }))
        .unwrap();
        let dbg = format!("{p:?} {:?}", JiraAdapter.auth(&p.account, &p.token));
        assert!(!dbg.contains("SENTINEL-TOKEN-XYZ"), "{dbg}");
    }

    fn cp(project_key: Option<&str>) -> ConnectParams {
        ConnectParams {
            base_url: "https://acme.atlassian.net".into(),
            board_ref: "42".into(),
            account: "dev@example.com".into(),
            token: ApiToken("t".into()),
            project_key: project_key.map(String::from),
        }
    }

    #[tokio::test]
    async fn validate_derives_scope_and_name_from_the_board() {
        let board = FakeBoard::standard();
        let v = JiraAdapter.validate(&board, &cp(None)).await.unwrap();
        assert_eq!(
            v,
            Validated {
                external_scope: "AUTH".into(),
                board_name: "AUTH Scrum".into(),
                owner: OwnerLookup::Person(AccountRef::new("A1")),
                account: None,
                board_ref: None,
                params: Default::default(),
            }
        );
        assert_eq!(
            JiraAdapter.validate(&board, &cp(Some("AUTH"))).await.unwrap().external_scope,
            "AUTH"
        );
    }

    #[test]
    fn the_owner_is_read_from_myself_by_the_ordered_rules() {
        let person = |id: &str| OwnerLookup::Person(AccountRef::new(id));
        let cases = [
            (json!({"accountType": "atlassian", "accountId": "A1"}), person("A1")),
            (json!({"accountType": "Atlassian", "accountId": "A1"}), person("A1")),
            (json!({"accountType": "atlassian", "accountId": " A1 "}), person(" A1 ")),
            (json!({"accountType": "app"}), OwnerLookup::NotAPerson),
            (json!({"accountType": "customer", "accountId": "C1"}), OwnerLookup::NotAPerson),
            (json!({}), OwnerLookup::Unknown),
            (json!({"accountId": "A1"}), OwnerLookup::Unknown),
            (json!({"accountType": "atlassian"}), OwnerLookup::Unknown),
            (json!({"accountType": "atlassian", "accountId": ""}), OwnerLookup::Unknown),
            (json!({"accountType": "atlassian", "accountId": 123}), OwnerLookup::Unknown),
            (json!({"accountType": null, "accountId": "A1"}), OwnerLookup::Unknown),
            (json!({"accountType": 7, "accountId": "A1"}), OwnerLookup::Unknown),
            (json!("not an object"), OwnerLookup::Unknown),
        ];
        for (body, want) in cases {
            assert_eq!(owner_from_myself(&body), want, "{body}");
        }
    }

    #[tokio::test]
    async fn owner_asks_myself_once() {
        let board = FakeBoard::standard().with(
            "/rest/api/3/myself",
            0,
            json!({"accountType": "app", "accountId": "x"}),
        );
        assert_eq!(JiraAdapter.owner(&board).await, Ok(OwnerLookup::NotAPerson));
        assert_eq!(board.calls(), vec!["/rest/api/3/myself".to_string()]);
        let denied = FakeBoard::standard().fail("/rest/api/3/myself", TrackerError::Forbidden);
        assert_eq!(JiraAdapter.owner(&denied).await, Err(TrackerError::Forbidden));
    }

    #[test]
    fn a_task_row_carries_its_assignee_when_jira_sent_one() {
        let assigned = json!({"id": "10001", "key": "AUTH-1",
            "fields": {"assignee": {"accountId": "A1", "displayName": "Dev"}}});
        assert_eq!(task_row(&assigned).unwrap().assignee, Some(AccountRef::new("A1")));
        for none in [
            json!({"id": "10002", "key": "AUTH-2", "fields": {"assignee": null}}),
            json!({"id": "10003", "key": "AUTH-3", "fields": {}}),
            json!({"id": "10004", "key": "AUTH-4", "fields": {"assignee": {"accountId": ""}}}),
        ] {
            assert_eq!(task_row(&none).unwrap().assignee, None, "{none}");
        }
    }

    #[test]
    fn both_issue_reads_ask_for_the_assignee() {
        let asks = |fields: &str| fields.split(',').any(|f| f == "assignee");
        assert!(asks(ISSUE_FIELDS[0].1), "board issues: {}", ISSUE_FIELDS[0].1);
        assert!(asks(SEARCH_FIELDS), "board search (R11): {SEARCH_FIELDS}");
    }

    // ── Write-back comments (W5) ───────────────────────────────────────────

    use crate::tracker::test_support::RecordingWrites;
    use crate::tracker::transport::PolicyTransport;
    use crate::tracker::writeback::Report;

    fn report() -> Report {
        Report {
            title: "Sensei · AI usage from the assignee's sessions (updated x)".into(),
            lines: vec!["Sessions: 1".into(), "Cost: metered — · API —".into()],
        }
    }

    #[test]
    fn a_report_is_a_document_of_text_nodes() {
        let r = Report { title: "T \"quoted\" <b>".into(), lines: vec!["a".into(), "b".into()] };
        let item = |t: &str| {
            json!({"type": "listItem", "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": t}]}]})
        };
        assert_eq!(
            adf(&r),
            json!({"type": "doc", "version": 1, "content": [
                {"type": "paragraph", "content": [
                    {"type": "text", "text": "T \"quoted\" <b>", "marks": [{"type": "strong"}]}]},
                {"type": "bulletList", "content": [item("a"), item("b")]}]})
        );
    }

    #[test]
    fn only_custom_field_ids_can_be_mapped() {
        for ok in ["customfield_10050", "customfield_1", "customfield_1234567890"] {
            assert!(JiraAdapter.field_ref_shape(ok), "{ok}");
        }
        for bad in [
            "",
            "customfield_",
            "customfield_0",
            "customfield_012",
            "customfield_12345678901",
            "summary",
            "Customfield_10050",
            "customfield_10050 ",
            "customfield_1/x",
            "cf[10050]",
        ] {
            assert!(!JiraAdapter.field_ref_shape(bad), "{bad:?}");
        }
    }

    #[tokio::test]
    async fn fields_are_checked_against_the_sites_field_list() {
        let board = FakeBoard::standard().with(
            "/rest/api/3/field",
            0,
            json!([
            {"id": "customfield_10050", "name": "AI hours", "schema": {"type": "number"}},
            {"id": "customfield_10051", "name": "Notes", "schema": {"type": "string"}},
            {"id": "summary", "name": "Summary", "schema": {"type": "string"}}]),
        );
        let refs =
            ["customfield_10050", "customfield_10051", "customfield_10099"].map(String::from);
        assert_eq!(
            JiraAdapter.check_fields(&board, &refs).await,
            Ok(vec![
                FieldCheck { field_ref: "customfield_10050".into(), exists: true, numeric: true },
                FieldCheck { field_ref: "customfield_10051".into(), exists: true, numeric: false },
                FieldCheck { field_ref: "customfield_10099".into(), exists: false, numeric: false },
            ])
        );
        assert_eq!(board.calls(), ["/rest/api/3/field"], "one request for every ref");
    }

    #[tokio::test]
    async fn a_field_is_in_use_when_any_ticket_on_the_board_has_a_value() {
        let jql = "cf[10050] is not EMPTY";
        let used = FakeBoard::standard()
            .with_jql(jql, json!([{"id": "10001", "key": "AUTH-1", "fields": {}}]));
        assert_eq!(JiraAdapter.field_in_use(&used, &scope(), "customfield_10050").await, Ok(true));
        assert_eq!(used.jql_calls(), [jql]);
        let q = used.query_of("/rest/agile/1.0/board/42/issue").unwrap();
        assert!(
            q.contains(&("maxResults".into(), "1".into()))
                && q.contains(&("fields".into(), "none".into())),
            "{q:?}"
        );
        let empty = FakeBoard::standard();
        assert_eq!(
            JiraAdapter.field_in_use(&empty, &scope(), "customfield_10050").await,
            Ok(false)
        );
        assert_eq!(
            JiraAdapter.field_in_use(&empty, &scope(), "summary").await,
            Err(TrackerError::BadRequest),
            "only a valid custom field id reaches a JQL string"
        );
    }

    #[tokio::test]
    async fn mapped_fields_are_written_in_one_request_and_null_clears() {
        let w = PolicyTransport::new(RecordingWrites::default(), 0);
        let fields = vec![
            ("customfield_10050".to_string(), FieldValue::Dec("2.05".into())),
            ("customfield_10051".to_string(), FieldValue::Null),
            ("customfield_10052".to_string(), FieldValue::Int(3)),
        ];
        JiraAdapter.write_fields(&w, "10001", &fields).await.unwrap();
        let sent = w.raw().sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            (sent[0].method, sent[0].path.as_str()),
            (WriteMethod::Put, "/rest/api/3/issue/10001")
        );
        assert!(sent[0].query.is_empty(), "no notifyUsers on an issue edit (X3)");
        assert_eq!(
            sent[0].body,
            json!({"fields": {"customfield_10050": 2.05, "customfield_10051": null, "customfield_10052": 3}})
        );
        let bad_ref = vec![("summary".to_string(), FieldValue::Int(1))];
        assert_eq!(
            JiraAdapter.write_fields(&w, "10001", &bad_ref).await,
            Err(TrackerError::Decode)
        );
        assert_eq!(JiraAdapter.write_fields(&w, "1/x", &fields).await, Err(TrackerError::Decode));
        assert_eq!(w.raw().sent().len(), 1, "a refused id or field never reaches a request");
    }

    #[test]
    fn only_numeric_ids_reach_a_path() {
        for ok in ["1", "10001", "123456789012345678"] {
            assert!(is_jira_id(ok), "{ok}");
        }
        for bad in ["", "1234567890123456789", "10001/../x", "1?x=1", "-1", "1 ", "AUTH-1", "１"] {
            assert!(!is_jira_id(bad), "{bad:?}");
        }
    }

    #[tokio::test]
    async fn a_comment_is_created_then_updated_without_notifying_watchers() {
        let w = PolicyTransport::new(RecordingWrites::default(), 0);
        let id = JiraAdapter.create_comment(&w, "10001", &report()).await.unwrap();
        assert_eq!(id, "20000");
        JiraAdapter.update_comment(&w, "10001", &id, &report()).await.unwrap();
        let sent = w.raw().sent();
        assert_eq!(sent.len(), 2);
        assert_eq!(
            (sent[0].method, sent[0].path.as_str()),
            (WriteMethod::Post, "/rest/api/3/issue/10001/comment")
        );
        assert_eq!(sent[0].body, json!({"body": adf(&report())}));
        assert!(sent[0].query.is_empty());
        assert_eq!(
            (sent[1].method, sent[1].path.as_str()),
            (WriteMethod::Put, "/rest/api/3/issue/10001/comment/20000")
        );
        assert_eq!(sent[1].query, [("notifyUsers".to_string(), "false".to_string())]);
        assert_eq!(sent[1].body, json!({"body": adf(&report())}));
    }

    #[tokio::test]
    async fn a_malformed_id_never_reaches_a_request() {
        let w = PolicyTransport::new(RecordingWrites::default(), 0);
        let board = FakeBoard::standard();
        let owner = AccountRef::new("A1");
        assert_eq!(
            JiraAdapter.create_comment(&w, "10001/../x", &report()).await,
            Err(TrackerError::BadRequest)
        );
        assert_eq!(
            JiraAdapter.update_comment(&w, "10001", "1?x=1", &report()).await,
            Err(TrackerError::BadRequest)
        );
        assert_eq!(
            JiraAdapter.find_comment(&board, "AUTH-1", &owner).await,
            Err(TrackerError::BadRequest)
        );
        assert_eq!(JiraAdapter.issue_exists(&board, "1/x").await, Err(TrackerError::BadRequest));
        assert!(w.raw().sent().is_empty() && board.calls().is_empty());
    }

    #[tokio::test]
    async fn the_newest_report_by_the_owner_is_adopted_and_nothing_else() {
        let c = |id: &str, author: &str, first: &str| {
            json!({"id": id, "author": {"accountId": author},
                   "body": {"type": "doc", "version": 1, "content": [
                       {"type": "paragraph", "content": [{"type": "text", "text": first}]}]}})
        };
        let path = "/rest/api/3/issue/10001/comment";
        let board = FakeBoard::standard().with(
            path,
            0,
            json!({"comments": [
                c("9", "B2", "Sensei · AI usage from the assignee's sessions (updated y)"),
                c("8", "A1", "Looks good to me"),
                c("7", "A1", "Sensei · AI usage from the assignee's sessions (updated x)"),
                c("6", "A1", "Sensei · AI usage from the assignee's sessions (updated w)"),
            ]}),
        );
        let a1 = AccountRef::new("A1");
        assert_eq!(JiraAdapter.find_comment(&board, "10001", &a1).await, Ok(Some("7".into())));
        assert_eq!(
            board.query_of(path),
            Some(vec![("orderBy".into(), "-created".into()), ("maxResults".into(), "50".into())])
        );
        assert_eq!(
            JiraAdapter.find_comment(&board, "10001", &AccountRef::new("C3")).await,
            Ok(None)
        );
        let none = FakeBoard::standard().with(path, 0, json!({"comments": []}));
        assert_eq!(JiraAdapter.find_comment(&none, "10001", &a1).await, Ok(None));
        let denied = FakeBoard::standard().fail(path, TrackerError::Forbidden);
        assert_eq!(
            JiraAdapter.find_comment(&denied, "10001", &a1).await,
            Err(TrackerError::Forbidden)
        );
    }

    #[tokio::test]
    async fn an_issue_exists_until_jira_says_it_is_gone() {
        let path = "/rest/api/3/issue/10001";
        let board = FakeBoard::standard().with(path, 0, json!({"id": "10001"}));
        assert_eq!(JiraAdapter.issue_exists(&board, "10001").await, Ok(true));
        assert_eq!(board.query_of(path), Some(vec![("fields".into(), "none".into())]));
        let gone = FakeBoard::standard().fail(path, TrackerError::NotFound);
        assert_eq!(JiraAdapter.issue_exists(&gone, "10001").await, Ok(false));
        let denied = FakeBoard::standard().fail(path, TrackerError::Forbidden);
        assert_eq!(JiraAdapter.issue_exists(&denied, "10001").await, Err(TrackerError::Forbidden));
    }

    #[tokio::test]
    async fn validate_refusals() {
        let board = FakeBoard::standard();
        assert_eq!(
            JiraAdapter.validate(&board, &cp(Some("OTHER"))).await,
            Err(ValidateError::BoardProjectMismatch)
        );
        let missing =
            FakeBoard::standard().fail("/rest/agile/1.0/board/42", TrackerError::NotFound);
        assert_eq!(
            JiraAdapter.validate(&missing, &cp(None)).await,
            Err(ValidateError::BoardNotFound)
        );
        let denied = FakeBoard::standard().fail("/rest/api/3/myself", TrackerError::Unauthorized);
        assert_eq!(
            JiraAdapter.validate(&denied, &cp(None)).await,
            Err(ValidateError::Tracker(TrackerError::Unauthorized))
        );
    }

    #[tokio::test]
    async fn a_board_without_a_single_project_uses_its_name() {
        let board = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42",
            0,
            json!({"id": 42, "name": "Platform"}),
        );
        assert_eq!(
            JiraAdapter.validate(&board, &cp(None)).await.unwrap().external_scope,
            "Platform"
        );
    }

    fn scope() -> BoardScope {
        BoardScope { base_url: "https://acme.atlassian.net".into(), board_ref: "42".into() }
    }

    #[tokio::test]
    async fn fetch_snapshot_pages_through_everything() {
        let board = FakeBoard::standard();
        let s = JiraAdapter.fetch_snapshot(&board, &scope(), &PrevState::default()).await.unwrap();
        let keys: Vec<&str> = s.tasks.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, ["AUTH-1", "AUTH-2", "AUTH-3"], "two issue pages, keys upper-cased");
        assert_eq!(s.tasks[0].issue_type.as_deref(), Some("Story"));
        assert_eq!(s.tasks[0].status.as_deref(), Some("To Do"));
        let sprints: Vec<(&str, SprintState, i64)> =
            s.sprints.iter().map(|x| (x.external_id.as_str(), x.state, x.seq)).collect();
        assert_eq!(sprints, [("100", SprintState::Closed, 100), ("101", SprintState::Active, 101)]);
        assert!(s.sprints[0].complete_at.is_some() && s.sprints[1].start_at.is_some());
        let mut members = s.members.clone();
        members.sort();
        assert_eq!(
            members,
            [
                ("100".into(), Some(vec!["10001".into()])),
                ("101".into(), Some(vec!["10002".into(), "10003".into()]))
            ]
        );
    }

    #[tokio::test]
    async fn a_closed_sprint_fetched_after_closing_is_not_fetched_again() {
        let board = FakeBoard::standard();
        let mut prev = PrevState::default();
        prev.members_fetched_state.insert("100".into(), Some(SprintState::Closed));
        prev.members_fetched_state.insert("101".into(), Some(SprintState::Active));
        let s = JiraAdapter.fetch_snapshot(&board, &scope(), &prev).await.unwrap();
        assert!(s.members.contains(&("100".into(), None)), "closed+fetched-closed sprint skipped");
        assert!(!board.was_called("/rest/agile/1.0/board/42/sprint/100/issue"));
        assert!(
            board.was_called("/rest/agile/1.0/board/42/sprint/101/issue"),
            "open sprints always refetched"
        );
    }

    #[tokio::test]
    async fn any_failed_page_fails_the_whole_snapshot() {
        let board = FakeBoard::standard().fail_page(
            "/rest/agile/1.0/board/42/issue",
            2,
            TrackerError::HttpError,
        );
        let r = JiraAdapter.fetch_snapshot(&board, &scope(), &PrevState::default()).await;
        assert_eq!(r, Err(TrackerError::HttpError));
    }

    #[tokio::test]
    async fn an_unknown_sprint_state_is_a_decode_error() {
        let board = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/sprint",
            0,
            json!({"isLast": true, "values": [{"id": 9, "name": "X", "state": "weird"}]}),
        );
        let r = JiraAdapter.fetch_snapshot(&board, &scope(), &PrevState::default()).await;
        assert_eq!(r, Err(TrackerError::Decode));
    }
}

#[cfg(test)]
mod match_tests {
    use super::*;

    const S: uuid::Uuid = uuid::Uuid::from_u128(7);

    fn known(keys: &[(&str, &str)]) -> Vec<KnownTask> {
        // A task's first listed key is its current one.
        let mut seen = std::collections::HashSet::new();
        keys.iter()
            .map(|(id, key)| KnownTask {
                external_id: id.to_string(),
                key: key.to_string(),
                scope: None,
                current: seen.insert(*id),
            })
            .collect()
    }

    /// AUTH-420 = 1, AUTH-421 = 2, MY_PROJ-12 = 3.
    fn standard() -> Vec<KnownTask> {
        known(&[("1", "AUTH-420"), ("2", "AUTH-421"), ("3", "MY_PROJ-12")])
    }

    fn session(branches: &[&str], prompts: &[&str]) -> SessionText {
        SessionText {
            session_id: S,
            branches: branches.iter().map(|b| b.to_string()).collect(),
            prompts: prompts.iter().map(|p| p.to_string()).collect(),
            repo_key: None,
        }
    }

    /// (task, source, matched_key, mentions), sorted.
    fn found(s: &SessionText, k: &[KnownTask]) -> Vec<(String, LinkSource, String, u32)> {
        let mut v: Vec<_> = match_keys(s, k)
            .into_iter()
            .inspect(|l| assert_eq!(l.session_id, S))
            .map(|l| (l.task_external_id, l.source, l.matched_key, l.mentions))
            .collect();
        v.sort();
        v
    }

    fn prompt_keys(text: &str, k: &[KnownTask]) -> Vec<String> {
        found(&session(&[], &[text]), k).into_iter().map(|(_, _, key, _)| key).collect()
    }

    const NONE: Vec<String> = Vec::new();

    #[test]
    fn only_known_keys_link() {
        let k = standard();
        for text in ["UTF-8", "AES-256", "CVE-2026", "AUTH-999", "auth", ""] {
            assert_eq!(prompt_keys(text, &k), NONE, "{text:?}");
        }
        assert!(found(&session(&["fix/auth-420"], &["AUTH-420"]), &[]).is_empty(), "empty known");
    }

    #[test]
    fn a_branch_links_case_insensitively() {
        assert_eq!(
            found(&session(&["fix/auth-420-store"], &[]), &standard()),
            vec![("1".into(), LinkSource::Branch, "AUTH-420".into(), 1)]
        );
    }

    #[test]
    fn boundaries() {
        let k = standard();
        for text in ["AUTH-420_fix", "feature_AUTH-420", "(AUTH-420)", "AUTH-420.", "x/AUTH-420"] {
            assert_eq!(prompt_keys(text, &k), vec!["AUTH-420".to_string()], "{text:?}");
        }
        for text in ["xAUTH-420", "9AUTH-420", "AUTH-4201", "AUTH-0420", "AUTH-", "AUTH 420"] {
            assert_eq!(prompt_keys(text, &k), NONE, "{text:?}");
        }
    }

    #[test]
    fn underscore_prefixes_and_consumed_spans() {
        let k = standard();
        assert_eq!(prompt_keys("MY_PROJ-12", &k), vec!["MY_PROJ-12".to_string()]);
        let mut with_proj = k.clone();
        with_proj.extend(known(&[("4", "PROJ-12")]));
        assert_eq!(
            prompt_keys("see MY_PROJ-12", &with_proj),
            vec!["MY_PROJ-12".to_string()],
            "the consumed span never also yields PROJ-12"
        );
        assert_eq!(prompt_keys("see PROJ-12", &with_proj), vec!["PROJ-12".to_string()]);
    }

    #[test]
    fn no_unicode_folding_and_no_panics() {
        let k = known(&[("1", "KEY-1"), ("2", "AUTH-420")]);
        assert_eq!(prompt_keys("\u{212A}EY-1", &k), NONE, "the Kelvin sign is not K");
        assert_eq!(prompt_keys("ßİ AUTH-420", &k), vec!["AUTH-420".to_string()]);
        assert_eq!(
            prompt_keys("İAUTH-420ß", &k),
            vec!["AUTH-420".to_string()],
            "non-ASCII separates"
        );
        assert_eq!(prompt_keys("🙂🙂🙂 auth-", &k), NONE);
    }

    #[test]
    fn prompt_mentions_count_occurrences_and_branch_mentions_count_branches() {
        let k = standard();
        let s = session(
            &["fix/AUTH-420", "AUTH-420-retry", "main"],
            &["AUTH-420 then auth-420", "and AUTH-421"],
        );
        assert_eq!(
            found(&s, &k),
            vec![
                ("1".into(), LinkSource::Branch, "AUTH-420".into(), 2),
                ("1".into(), LinkSource::Prompt, "AUTH-420".into(), 2),
                ("2".into(), LinkSource::Prompt, "AUTH-421".into(), 1),
            ]
        );
        let twice_in_one_branch = session(&["AUTH-420-AUTH-420"], &[]);
        assert_eq!(found(&twice_in_one_branch, &k)[0].3, 1, "a branch counts once");
    }

    #[test]
    fn aliases_of_one_task_merge_into_one_link() {
        // Task 1 was AUTH-420 and is now IAM-5 (the current key is listed first).
        let k = known(&[("1", "IAM-5"), ("1", "AUTH-420")]);
        assert_eq!(
            found(&session(&["fix/auth-420"], &[]), &k),
            vec![("1".into(), LinkSource::Branch, "AUTH-420".into(), 1)],
            "the old key still links"
        );
        assert_eq!(
            found(&session(&[], &["AUTH-420 AUTH-420 IAM-5"]), &k),
            vec![("1".into(), LinkSource::Prompt, "AUTH-420".into(), 3)],
            "one link; the most-mentioned alias names it; mentions summed"
        );
        assert_eq!(
            found(&session(&[], &["AUTH-420 IAM-5"]), &k),
            vec![("1".into(), LinkSource::Prompt, "IAM-5".into(), 2)],
            "a tie goes to the current key"
        );
    }
}

#[cfg(test)]
mod discover_tests {
    use super::*;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    fn known(keys: &[&str]) -> Vec<KnownTask> {
        keys.iter()
            .enumerate()
            .map(|(i, k)| KnownTask {
                external_id: i.to_string(),
                key: k.to_string(),
                scope: None,
                current: true,
            })
            .collect()
    }

    fn session(branches: &[&str], prompts: &[&str]) -> SessionText {
        SessionText {
            session_id: uuid::Uuid::nil(),
            branches: branches.iter().map(|b| b.to_string()).collect(),
            prompts: prompts.iter().map(|p| p.to_string()).collect(),
            repo_key: None,
        }
    }

    #[test]
    fn the_jira_key_shape() {
        for ok in ["AUTH-1", "MY_PROJ-12", "P057542601-30", "A-0"] {
            assert!(is_jira_key(ok), "{ok}");
        }
        for bad in [
            "auth-1",
            "AUTH-",
            "-1",
            "1AUTH-1",
            "_A-1",
            "AUTH-1)",
            "AU TH-1",
            "AUTH-1 OR 1=1",
            "AUTH-1x",
            "",
        ] {
            assert!(!is_jira_key(bad), "{bad}");
        }
    }

    #[test]
    fn unknown_keys_are_board_shaped_unsynced_and_safe() {
        let k = known(&["AUTH-1", "AUTH-2"]);
        let s = session(
            &["fix/auth-900-login", "feature/AUTH-77"],
            &["AUTH-900 again, AUTH-1, UTF-8, Q-5 and AUTH-9x;DROP"],
        );
        assert_eq!(
            JiraAdapter.unknown_keys(&s, &k),
            vec!["AUTH-77".to_string(), "AUTH-9".into(), "AUTH-900".into()],
            "sorted, once each, upper-cased; known, other-prefix and non-keys excluded"
        );
        assert!(JiraAdapter.unknown_keys(&s, &[]).is_empty(), "no known prefix, nothing to ask");
        // Defence in depth: a malformed stored key's prefix never yields a key
        // that could reach JQL.
        let odd = known(&["A B-1"]);
        assert!(JiraAdapter.unknown_keys(&session(&[], &["A B-5"]), &odd).is_empty());
        for key in JiraAdapter.unknown_keys(&s, &k) {
            assert!(is_jira_key(&key), "{key} must pass the shape check before any JQL");
        }
    }

    fn scope() -> BoardScope {
        BoardScope { base_url: "https://acme.atlassian.net".into(), board_ref: "42".into() }
    }

    #[tokio::test]
    async fn fetch_tasks_asks_the_board_and_reads_memberships() {
        let board = FakeBoard::empty().with_jql(
            "key in (AUTH-77,AUTH-900)",
            json!([{"id": "10900", "key": "auth-900", "fields": {
                "summary": "New thing", "issuetype": {"name": "Bug"}, "status": {"name": "To Do"},
                "sprint": {"id": 101, "state": "active"},
                "closedSprints": [{"id": 100, "state": "closed"}]}}]),
        );
        let keys = vec!["AUTH-900".to_string(), "AUTH-77".into()];
        let d = JiraAdapter.fetch_tasks(&board, &scope(), &keys).await.unwrap();
        assert_eq!(
            board.jql_calls(),
            vec!["key in (AUTH-77,AUTH-900)"],
            "sorted keys, one request"
        );
        assert_eq!(d.tasks.len(), 1);
        assert_eq!(
            (d.tasks[0].external_id.as_str(), d.tasks[0].key.as_str()),
            ("10900", "AUTH-900")
        );
        assert_eq!(d.tasks[0].summary.as_deref(), Some("New thing"));
        let mut members = d.members.clone();
        members.sort();
        assert_eq!(
            members,
            vec![("100".to_string(), "10900".to_string()), ("101".into(), "10900".into())]
        );
    }

    #[tokio::test]
    async fn fetch_tasks_never_puts_an_unchecked_key_in_jql() {
        let board = FakeBoard::empty();
        let keys = vec!["AUTH-1) OR (project = X".to_string(), "AUTH-2".into()];
        JiraAdapter.fetch_tasks(&board, &scope(), &keys).await.unwrap();
        assert_eq!(board.jql_calls(), vec!["key in (AUTH-2)"]);
        let none = FakeBoard::empty();
        let d = JiraAdapter.fetch_tasks(&none, &scope(), &["bad key".to_string()]).await.unwrap();
        assert_eq!((d, none.jql_calls()), (Discovered::default(), Vec::<String>::new()));
    }

    #[tokio::test]
    async fn a_rejected_batch_falls_back_to_one_key_per_request() {
        let board = FakeBoard::empty()
            .fail_jql("key in (AUTH-404,AUTH-900)", TrackerError::BadRequest)
            .fail_jql("key in (AUTH-404)", TrackerError::BadRequest)
            .with_jql(
                "key in (AUTH-900)",
                json!([{"id": "10900", "key": "AUTH-900", "fields": {}}]),
            );
        let keys = vec!["AUTH-404".to_string(), "AUTH-900".into()];
        let d = JiraAdapter.fetch_tasks(&board, &scope(), &keys).await.unwrap();
        assert_eq!(d.tasks.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(), vec!["AUTH-900"]);

        // Any other failure fails the fetch: "not on the board" is only a 400's answer.
        let down =
            FakeBoard::empty().fail_jql("key in (AUTH-404,AUTH-900)", TrackerError::HttpError);
        assert_eq!(
            JiraAdapter.fetch_tasks(&down, &scope(), &keys).await,
            Err(TrackerError::HttpError)
        );
    }

    // ---- ticket history (spec tracker-sdlc-metrics S4, A6) ----

    #[test]
    fn jira_instants_take_offsets_without_a_colon() {
        let want = chrono::DateTime::parse_from_rfc3339("2021-01-17T12:34:00Z").unwrap();
        assert_eq!(jira_instant("2021-01-17T12:34:00.000+0000"), Some(want.into()));
        assert_eq!(jira_instant("2021-01-17T17:34:00.000+0500"), Some(want.into()));
        assert_eq!(jira_instant("2021-01-17T12:34:00.000Z"), Some(want.into()));
        assert_eq!(jira_instant("yesterday"), None);
    }

    #[test]
    fn task_rows_carry_times_category_and_points() {
        let v = json!({"id": "1", "key": "A-1", "fields": {
            "created": "2021-01-17T12:34:00.000+0000", "updated": "2021-01-18T23:45:00.000+0000",
            "status": {"name": "Done", "statusCategory": {"key": "done"}},
            "customfield_10016": 3.0}});
        let t = task_row_with(&v, Some("customfield_10016")).unwrap();
        assert_eq!(t.created_at, jira_instant("2021-01-17T12:34:00.000+0000"));
        assert_eq!(t.updated_at, jira_instant("2021-01-18T23:45:00.000+0000"));
        assert_eq!(t.status_category.as_deref(), Some("done"));
        assert_eq!(t.points, Some(Some("3".to_string())));
        let text = json!({"id": "1", "key": "A-1", "fields": {"customfield_10016": "5", "status": {"statusCategory": {"key": "odd"}}}});
        let t = task_row_with(&text, Some("customfield_10016")).unwrap();
        assert_eq!((t.points, t.status_category), (Some(Some("5".to_string())), None));
        let junk = json!({"id": "1", "key": "A-1", "fields": {"customfield_10016": {"value": 1}}});
        assert_eq!(task_row_with(&junk, Some("customfield_10016")).unwrap().points, Some(None));
        let bare = json!({"id": "1", "key": "A-1", "fields": {}});
        let t = task_row_with(&bare, None).unwrap();
        assert_eq!(
            (t.points, t.created_at, t.updated_at),
            (None, None, None),
            "not asked, so kept"
        );
    }

    #[tokio::test]
    async fn the_issue_fetch_asks_for_the_estimation_field_once_it_is_known() {
        let board = FakeBoard::standard();
        JiraAdapter.fetch_snapshot(&board, &scope(), &PrevState::default()).await.unwrap();
        let fields = |b: &FakeBoard| {
            b.query_of("/rest/agile/1.0/board/42/issue")
                .unwrap()
                .into_iter()
                .find(|(k, _)| k == "fields")
                .unwrap()
                .1
        };
        let asked = fields(&board);
        assert!(asked.contains("created") && asked.contains("updated"), "{asked}");
        assert!(!asked.contains("customfield"), "{asked}");
        let board = FakeBoard::standard();
        let prev = PrevState {
            estimation: crate::tracker::Estimation::Field("customfield_10016".into()),
            ..Default::default()
        };
        JiraAdapter.fetch_snapshot(&board, &scope(), &prev).await.unwrap();
        assert!(fields(&board).ends_with(",customfield_10016"), "{}", fields(&board));
        let board = FakeBoard::standard();
        let none =
            PrevState { estimation: crate::tracker::Estimation::NoField, ..Default::default() };
        let s = JiraAdapter.fetch_snapshot(&board, &scope(), &none).await.unwrap();
        assert!(
            s.tasks.iter().all(|t| t.points == Some(None)),
            "learned no field: clear the points"
        );
        let s = JiraAdapter
            .fetch_snapshot(&FakeBoard::standard(), &scope(), &PrevState::default())
            .await
            .unwrap();
        assert!(s.tasks.iter().all(|t| t.points.is_none()), "not learned: keep them");
    }

    #[tokio::test]
    async fn board_terms_come_from_configuration_fields_and_statuses() {
        let board = FakeBoard::empty()
            .with("/rest/agile/1.0/board/42/configuration", 0, json!({"estimation": {"type": "field",
                "field": {"fieldId": "customfield_10016", "displayName": "Story point estimate"}}}))
            .with("/rest/api/3/field", 0, json!([
                {"id": "summary", "name": "Summary", "schema": {"type": "string", "system": "summary"}},
                {"id": "customfield_10020", "name": "Sprints", "schema": {"type": "array",
                 "custom": "com.pyxis.greenhopper.jira:gh-sprint"}}]))
            .with("/rest/api/3/status", 0, json!([
                {"id": "1", "name": "To Do", "statusCategory": {"key": "new"}},
                {"id": "10001", "name": "Done", "statusCategory": {"key": "done"}}]));
        let terms = JiraAdapter.board_terms(&board, &scope()).await.unwrap();
        assert_eq!(terms.estimation_field.as_deref(), Some("customfield_10016"));
        assert_eq!(terms.sprint_field.as_deref(), Some("customfield_10020"));
        assert_eq!(
            terms.statuses,
            [
                ("1".to_string(), "To Do".to_string(), "new".to_string()),
                ("10001".to_string(), "Done".to_string(), "done".to_string())
            ]
        );
        let by_count = FakeBoard::empty()
            .with(
                "/rest/agile/1.0/board/42/configuration",
                0,
                json!({"estimation": {"type": "none"}}),
            )
            .with("/rest/api/3/field", 0, json!([]))
            .with("/rest/api/3/status", 0, json!([]));
        let terms = JiraAdapter.board_terms(&by_count, &scope()).await.unwrap();
        assert_eq!((terms.estimation_field, terms.sprint_field), (None, None));
        let bad = FakeBoard::empty()
            .with(
                "/rest/agile/1.0/board/42/configuration",
                0,
                json!({"estimation": {"type": "field", "field": {"fieldId": "customfield_0"}}}),
            )
            .with("/rest/api/3/field", 0, json!([]))
            .with("/rest/api/3/status", 0, json!([]));
        assert_eq!(
            JiraAdapter.board_terms(&bad, &scope()).await.unwrap().estimation_field,
            None,
            "not a field id"
        );
    }

    #[tokio::test]
    async fn the_changelog_keeps_sprint_status_and_estimate_by_field_id() {
        let item = |field: &str, id: &str, from: &str, to: &str, from_s: &str, to_s: &str| json!({"field": field, "fieldId": id, "from": from, "to": to, "fromString": from_s, "toString": to_s});
        let board = FakeBoard::empty()
            .with("/rest/api/3/issue/10001/changelog", 0, json!({"startAt": 0, "maxResults": 1, "isLast": false,
                "values": [{"id": "h1", "created": "2026-09-01T10:00:00.000+0000", "items": [
                    item("Sprints", "customfield_10020", "", "100", "", "Sprint 1"),
                    item("status", "status", "1", "3", "To Do", "In Progress"),
                    item("Story point estimate", "customfield_10016", "", "", "", "3"),
                    item("summary", "summary", "", "", "a", "b")]}]}))
            .with("/rest/api/3/issue/10001/changelog", 1, json!({"startAt": 1, "maxResults": 1, "isLast": true,
                "values": [{"id": "h2", "created": "2026-09-05T10:00:00.000+0000", "items": [
                    item("Sprint", "customfield_99999", "100", "100,101", "Sprint 1", "Sprint 1, Sprint 2")]}]}));
        let rows = JiraAdapter
            .changelog(&board, "10001", Some("customfield_10020"), Some("customfield_10016"))
            .await
            .unwrap();
        let got: Vec<_> = rows
            .iter()
            .map(|r| {
                (r.change_id.as_str(), r.field, r.from_value.as_deref(), r.to_value.as_deref())
            })
            .collect();
        assert_eq!(
            got,
            [
                ("h1", "sprint", None, Some("100")),
                ("h1", "status", Some("1"), Some("3")),
                ("h1", "estimate", None, Some("3")),
            ],
            "the second page's item is another field, named Sprint but not the site's"
        );
        assert_eq!(rows[0].at, jira_instant("2026-09-01T10:00:00.000+0000").unwrap());
        assert!(
            JiraAdapter.changelog(&board, "../x", None, None).await.is_err(),
            "the path is guarded"
        );
        let unnamed = FakeBoard::empty().with("/rest/api/3/issue/10002/changelog", 0, json!({"startAt": 0, "isLast": true,
            "values": [{"id": "h9", "created": "2026-09-01T10:00:00.000+0000", "items": [{"field": "Sprint", "to": "1"}]}]}));
        assert!(
            JiraAdapter.changelog(&unnamed, "10002", None, None).await.unwrap().is_empty(),
            "no Sprint field known: an item without a field id is nobody's sprint"
        );
    }
}
