//! Front-door intake: axes -> playbook recommendation (§8).

use axum::{extract::State, http::StatusCode, Json};

use crate::api::state::AppState;
use crate::playbook::{recommend, Axes, Intent, Lifecycle, Risk};

/// GET /api/playbook/guide -> { frame, axes: [{axis, prompt, help}], playbooks: [...] }
///
/// Serves the grounding frame + per-axis elicitation prompts + the playbook catalog
/// that `/sensei:intake` uses to run the guided questionnaire. Fetched (not
/// hardcoded) so the guide stays org-tunable at runtime.
pub(crate) async fn get_intake_guide(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Fail closed: a DB error must not read as an empty guide/catalog — that
    // would present the intake questionnaire as if no playbooks were configured.
    let guide = state.pg.list_intake_guide().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let playbooks = state.pg.list_playbooks().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let frame = guide
        .iter()
        .find(|g| g["kind"] == "frame")
        .and_then(|g| g["prompt"].as_str())
        .unwrap_or("")
        .to_string();
    let axes: Vec<_> = guide.into_iter().filter(|g| g["kind"] == "axis").collect();
    Ok(Json(serde_json::json!({ "frame": frame, "axes": axes, "playbooks": playbooks })))
}

/// POST /api/playbook/recommend
/// `{ lifecycle, intent, risk, session_id?, feature?, confirm? }` — pre-classified axes, OR
/// `{ chunk, has_existing_code?, blast?, session_id?, feature?, confirm? }` — free text, classified via
/// `classify_chunk` (gateway, heuristic fallback) before recommending.
///
/// The direct-axes path takes no LLM, so every run it persists is recorded
/// `classified_by = "manual"`, `model_fallback = false`. The `chunk` (text) path
/// records the real `classified_by`/`model_fallback` from `classify_chunk` — this is
/// how §9 measures whether the local model actually pulls its weight vs. the heuristic.
pub(crate) async fn recommend_playbook(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let direct_axes = (
        body["lifecycle"].as_str().and_then(Lifecycle::parse),
        body["intent"].as_str().and_then(Intent::parse),
        body["risk"].as_str().and_then(Risk::parse),
    );

    let (axes, classified_by, model_fallback) = match direct_axes {
        (Some(lf), Some(it), Some(rk)) => (Axes { lifecycle: lf, intent: it, risk: rk }, "manual".to_string(), false),
        _ => {
            let Some(chunk) = body["chunk"].as_str().filter(|s| !s.is_empty()) else {
                return Json(serde_json::json!({
                    "error": "either lifecycle/intent/risk or a non-empty chunk (text) is required"
                }));
            };
            // Bounded inputs to the heuristic fallback — see `heuristic_axes` doc comment
            // for why `has_existing_code`/`blast` stay simple params here rather than a
            // full code-graph-derived blast-radius (deferred; docs/plan/2026-07-19-frontdoor-intake-design.md §"risk").
            let has_existing_code = body["has_existing_code"].as_bool().unwrap_or(true);
            let blast = body["blast"].as_i64().unwrap_or(0);
            let cls = classify_chunk(&state, chunk, has_existing_code, blast).await;
            (cls.axes, cls.classified_by, cls.model_fallback)
        }
    };

    let rules = match state.pg.list_playbook_rules().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("recommend_playbook: list_playbook_rules failed: {e}");
            return Json(serde_json::json!({ "error": e }));
        }
    };
    let rec = recommend(&axes, &rules);

    // A playbook run always happens IN a project — there is no global run (no cwd,
    // no repo, no code graph, no rules scope to compute trust against). Require a
    // resolvable project and NEVER fabricate one on a miss (no-fabrication rule);
    // both persist and the auto-select trust query scope to it.
    let project_ident = body["project"].as_str().or_else(|| body["project_id"].as_str())
        .map(str::trim).filter(|s| !s.is_empty());
    let Some(ident) = project_ident else {
        return Json(serde_json::json!({
            "error": "project (id or name) is required — a playbook run always happens in a project"
        }));
    };
    let project_id = match crate::api::util::resolve_project_uuid(&state, ident).await {
        Ok(Some(pid)) => pid,
        Ok(None) => return Json(serde_json::json!({ "error": format!("unknown project: {ident}") })),
        Err(_) => return Json(serde_json::json!({ "error": "failed to resolve project" })),
    };

    // Persist the run unless this is a preview call. Recommend-and-confirm
    // defaults confirmed=false until the caller confirms; the app intake form's
    // recommend leg passes preview=true (classify + recommend, no row written).
    let confirmed = parse_bool_flag(&body["confirm"]);
    let session_id = body["session_id"].as_str().and_then(|s| s.parse().ok());
    if should_persist(&body)
        && let Err(e) = state
            .pg
            .insert_playbook_run(
                session_id,
                body["feature"].as_str(),
                axes.lifecycle.as_str(),
                axes.intent.as_str(),
                axes.risk.as_str(),
                rec.rule_id,
                &rec.playbook,
                &rec.rationale,
                confirmed,
                Some(classified_by.as_str()),
                model_fallback,
                project_id,
            )
            .await
    {
        tracing::error!("recommend_playbook: insert_playbook_run failed: {e}");
    }

    // Enrich the response with the chosen playbook's tone so the caller (the
    // /sensei:intake procedure) can adopt it as posture without a second round-trip.
    let pb = state.pg.get_playbook(&rec.playbook).await.ok().flatten();
    let opening_tone = pb.as_ref().and_then(|p| p["opening_tone"].as_str()).unwrap_or("").to_string();
    let when_to_use = pb.as_ref().and_then(|p| p["when_to_use"].as_str()).unwrap_or("").to_string();

    // Auto-select-on-trust: only low-risk chunks, only with proven FTR history.
    let (auto_select, trust_n, trust_ftr) = if matches!(axes.risk, crate::playbook::Risk::Low) {
        match state.pg.playbook_combo_trust(
            axes.lifecycle.as_str(), axes.intent.as_str(), axes.risk.as_str(), &rec.playbook, &project_id).await {
            Ok((n, ftr)) => (crate::playbook::is_trusted(axes.risk, n, ftr), n, ftr),
            Err(e) => { tracing::warn!(error=%e, "recommend_playbook: trust query failed — no auto-select"); (false, 0, 0.0) }
        }
    } else { (false, 0, 0.0) };

    // Workstream D: suggest the skills/agents provided by the libraries this project
    // uses ("this repo uses rokkit → load its styling skill / config reviewer"). FAIL
    // CLOSED — on an unresolved project or a query error, OMIT the keys entirely; an
    // empty `[]` (present keys) means "resolved, genuinely none", never a masked
    // failure. (Deliberately NOT the `.ok().flatten()` masking used for the tone read.)
    let mut resp = serde_json::json!({
        "playbook": rec.playbook,
        "rationale": rec.rationale,
        "lifecycle": axes.lifecycle.as_str(),
        "intent": axes.intent.as_str(),
        "risk": axes.risk.as_str(),
        "rule": rec.rule_name,
        "defaulted": rec.defaulted,
        "opening_tone": opening_tone,
        "when_to_use": when_to_use,
        "auto_select": auto_select,
        "trust": { "n": trust_n, "ftr": trust_ftr },
    });
    // Suggest the skills/agents provided by the libraries this project depends on
    // ("this repo uses rokkit → load its styling skill"). Reuses the project_id
    // resolved above. FAIL CLOSED — on a query error, OMIT the keys entirely; an
    // empty `[]` (present keys) means "resolved, genuinely none", never a masked failure.
    match state.pg.list_project_library_capabilities(&project_id).await {
        Ok(caps) => {
            if let Some(obj) = resp.as_object_mut() {
                obj.insert("suggested_skills".into(), caps["suggested_skills"].clone());
                obj.insert("suggested_agents".into(), caps["suggested_agents"].clone());
            }
        }
        Err(e) => tracing::warn!(error = %e, "recommend_playbook: library-capability suggestion failed — omitting"),
    }
    Json(resp)
}

/// Deterministic fallback — also the classifier when the gateway is unavailable.
///
/// `has_existing_code` and `blast` are deliberately simple bounded inputs (not a
/// full code-graph-derived blast-radius — see docs/plan/2026-07-19-frontdoor-intake-design.md
/// §"risk" for the deferred fuller derivation via callers/community reach).
pub(crate) fn heuristic_axes(text: &str, has_existing_code: bool, blast: i64) -> Axes {
    let t = text.to_lowercase();
    let lifecycle = if has_existing_code { Lifecycle::Stable } else { Lifecycle::Greenfield };
    let intent = if t.contains("bug") || t.contains("fix") || t.contains("crash") || t.contains("regression") {
        Intent::Bug
    } else if t.contains("ui") || t.contains("design") || t.contains("mockup") || t.contains("screen") {
        Intent::Ux
    } else if t.contains("improve") || t.contains("enhance") || t.contains("tweak") {
        Intent::Enhancement
    } else if !has_existing_code && (t.contains("explore") || t.contains("spike") || t.contains("try")) {
        Intent::Explore
    } else {
        Intent::Feature
    };
    let risk = if blast >= 10 { Risk::High } else { Risk::Low };
    Axes { lifecycle, intent, risk }
}

/// Result of classifying a chunk into axes: the axes themselves, plus attribution
/// for §9's measurement of whether the local gateway model is actually useful here.
pub(crate) struct Classification {
    pub axes: Axes,
    pub classified_by: String,
    pub model_fallback: bool,
}

/// Classify a text chunk into `{lifecycle, intent, risk}` via the embedded gateway
/// (reasoning chain), falling back to `heuristic_axes` on ANY error, timeout, or
/// unparseable/invalid response. Fail-open — never blocks the caller, never silent
/// (every fallback is logged via `tracing::warn!`).
pub(crate) async fn classify_chunk(
    state: &AppState,
    text: &str,
    has_existing_code: bool,
    blast: i64,
) -> Classification {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};

    let system = "You classify a chunk of software work into three axes for a playbook \
recommender. Respond with ONLY compact JSON and no prose: \
{\"lifecycle\":\"greenfield|stable\",\"intent\":\"explore|ux|feature|enhancement|bug\",\"risk\":\"low|high\"}. \
`lifecycle`: greenfield = new/empty project, stable = existing product. \
`intent`: explore = fuzzy/spike, ux = UX-heavy surface, feature = new known feature, \
enhancement = improving something existing, bug = fixing a defect. \
`risk`: high = touches many callers/a wide blast-radius, low = contained.";
    let user = format!("Chunk to classify:\n<<<CHUNK\n{text}\nCHUNK>>>");

    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        chain: Some("reasoning".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(MessageRole::User, user)],
            system: Some(system.to_string()),
            max_tokens: Some(200),
            temperature: None,
            tools: Vec::new(),
        },
        budget: None,
        auth: None,
        panel: None,
        consensus: None,
        allow_fallback: true,
        credentials: std::collections::HashMap::new(),
    };

    let fut = state.gateway.execute(&request);
    let fallback = || Classification {
        axes: heuristic_axes(text, has_existing_code, blast),
        classified_by: "heuristic".to_string(),
        model_fallback: true,
    };

    match tokio::time::timeout(std::time::Duration::from_secs(8), fut).await {
        Ok(Ok(resp)) if resp.success => match resp.content.as_deref().and_then(parse_axes_response) {
            Some(axes) => Classification {
                axes,
                classified_by: resp.model.unwrap_or_else(|| "gateway".to_string()),
                model_fallback: false,
            },
            None => {
                tracing::warn!(
                    "classify_chunk: gateway response unparseable/invalid — falling back to heuristic"
                );
                fallback()
            }
        },
        Ok(Ok(_)) => {
            tracing::warn!("classify_chunk: gateway response not successful — falling back to heuristic");
            fallback()
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "classify_chunk: gateway error — falling back to heuristic");
            fallback()
        }
        Err(_) => {
            tracing::warn!("classify_chunk: gateway call timed out — falling back to heuristic");
            fallback()
        }
    }
}

/// Extract and validate `{lifecycle,intent,risk}` from a (possibly fenced/prose-wrapped)
/// gateway response. Any missing/invalid axis value is a parse-miss (`None`) — the
/// caller fails open to the heuristic rather than persist a partially-guessed axis set.
fn parse_axes_response(content: &str) -> Option<Axes> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&content[start..=end]).ok()?;
    let lifecycle = v.get("lifecycle").and_then(|s| s.as_str()).and_then(Lifecycle::parse)?;
    let intent = v.get("intent").and_then(|s| s.as_str()).and_then(Intent::parse)?;
    let risk = v.get("risk").and_then(|s| s.as_str()).and_then(Risk::parse)?;
    Some(Axes { lifecycle, intent, risk })
}

/// Parse a truthy flag: a real JSON bool, or the string `"true"` (case-insensitive).
/// The MCP tool layer forwards flags as strings; direct HTTP callers send bools.
/// Shared by the `confirm` and `preview` flags on `recommend_playbook`.
fn parse_bool_flag(v: &serde_json::Value) -> bool {
    v.as_bool()
        .or_else(|| v.as_str().map(|s| s.eq_ignore_ascii_case("true")))
        .unwrap_or(false)
}

/// Whether a `recommend_playbook` call should record a `playbook_run`. Preview
/// calls (the app intake form's recommend leg) classify + recommend without
/// writing a row; the confirm leg persists exactly one.
fn should_persist(body: &serde_json::Value) -> bool {
    !parse_bool_flag(&body["preview"])
}

/// GET /api/playbook/rule-proposals -> { proposals: [...] }
///
/// Pending §9 learned-rule proposals (`source='learned' AND NOT enabled`) — the
/// accept-path list an operator reviews before promoting one into the resolver.
pub(crate) async fn list_rule_proposals(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proposals = state.pg.list_playbook_rule_proposals().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "proposals": proposals })))
}

/// POST /api/playbook/rule/{id}/accept -> { accepted: id } | { error: ... }
///
/// Flips a §9 learned-rule proposal `enabled=true`, making it visible to the
/// resolver's `list_playbook_rules`.
pub(crate) async fn accept_rule(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let uid = match id.parse() {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid id"}))),
    };
    match state.pg.accept_playbook_rule(&uid).await {
        Ok(true) => (StatusCode::OK, Json(serde_json::json!({"accepted": id}))),
        // No matching learned proposal — 404, never a fabricated {accepted}.
        Ok(false) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("no learned playbook proposal with id {id}")}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))),
    }
}

/// GET /api/playbook/model-stats -> { stats: [...] }
///
/// FTR by `classified_by` (+ `model_fallback`) — measures whether the local
/// gateway model's chunk classification is actually useful vs. the heuristic
/// fallback. Dashboard/inspection read; no MCP tool.
pub(crate) async fn model_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let stats = state.pg.playbook_model_stats().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "stats": stats })))
}

#[cfg(test)]
mod classify_tests {
    use super::*;

    #[test]
    fn heuristic_bug_on_stable() {
        let a = heuristic_axes("fix the crash when the token refreshes", /*has_existing_code=*/true, /*blast=*/2);
        assert_eq!(a.intent.as_str(), "bug");
        assert_eq!(a.lifecycle.as_str(), "stable");
    }

    #[test]
    fn heuristic_high_blast() {
        let a = heuristic_axes("rename the session store used everywhere", true, 40);
        assert_eq!(a.risk.as_str(), "high");
    }

    #[test]
    fn bool_flag_accepts_bool_and_string() {
        use serde_json::json;
        // MCP tool sends the string form; direct callers may send a real bool.
        assert!(parse_bool_flag(&json!(true)));
        assert!(parse_bool_flag(&json!("true")));
        assert!(parse_bool_flag(&json!("TRUE")));
        assert!(!parse_bool_flag(&json!("false")));
        assert!(!parse_bool_flag(&json!(false)));
        assert!(!parse_bool_flag(&serde_json::Value::Null));
    }

    #[test]
    fn preview_flag_skips_persist() {
        use serde_json::json;
        assert!(should_persist(&json!({})));                   // no preview → persist
        assert!(should_persist(&json!({ "preview": false }))); // explicit false → persist
        assert!(!should_persist(&json!({ "preview": true })));  // preview → skip
        assert!(!should_persist(&json!({ "preview": "true" }))); // string form → skip
    }
}
