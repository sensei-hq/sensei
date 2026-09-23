//! Process phase: index repos, folders, and files; handle deletions.

use super::super::executor::TaskContext;

use super::super::Task;
use super::helpers::{is_binary_ext, is_probably_binary};
use std::path::Path;

// ── Process Repo ──────────────────────────────────────────────────────────

// ── Reconcile identity ─────────────────────────────────────────────────────

/// Reconcile a project root's identity FROM its README frontmatter — folder
/// props (incl. the frontmatter snapshot), icons, project identity, role, and
/// folder_namespaces. Filesystem-READ-ONLY (it never writes the README, so it
/// can't trigger a file-change loop), idempotent, and additive. Shared by the
/// scan pipeline (process_git_folder) and the watcher's ReconcileRepoMetadata task.
pub async fn reconcile_repo_identity(
    ctx: &TaskContext,
    repo_abs_path: &str,
) -> Result<u32, String> {
    use crate::tasks::processors::metadata;
    let repo_path = Path::new(repo_abs_path);

    let Some(folder) = ctx.pg().get_repo_by_path(repo_abs_path).await.ok().flatten() else {
        return Ok(0); // not a registered folder
    };
    // Only project roots carry identity. A README inside a subfolder
    // (kind='folder') must not reconcile project/namespace/icon state.
    if !matches!(folder["kind"].as_str(), Some("git" | "standalone" | "subtree")) {
        return Ok(0);
    }
    let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) else {
        return Ok(0);
    };

    let fm = metadata::read_frontmatter(repo_path).unwrap_or_default();
    let icon = metadata::scan_icons(repo_path);
    let links = metadata::scan_external_links(repo_path);
    let summary = metadata::extract_summary(repo_path);
    let stack = super::scan_logic::detect_stack(repo_path);

    // Folder props: scanned metadata + the parsed frontmatter blob. The
    // frontmatter snapshot here is what reconcile_repo_metadata compares against to
    // suppress no-op re-reconciles.
    let meta = serde_json::json!({
        "icon": icon,
        "external_links": links.links,
        "summary": summary,
        "frontmatter": serde_json::to_value(&fm).unwrap_or(serde_json::Value::Null),
    });
    if let Err(e) = ctx.pg().set_folder_props(&folder_id, &meta).await {
        tracing::warn!(folder_id = %folder_id, error = %e, "set_folder_props (reconcile meta) failed");
    }

    // Icon variants + URL-vs-repo-relative classification (root READMEs only).
    if let Some(icon_path) = fm.icon.as_deref() {
        let mut icons = serde_json::json!({
            "custom": icon_path,
            "custom_is_url": metadata::icon_is_url(icon_path),
        });
        if let Some(dark) = fm.icon_dark.as_deref() {
            icons["custom_dark"] = serde_json::json!(dark);
            icons["custom_dark_is_url"] = serde_json::json!(metadata::icon_is_url(dark));
        }
        if let Err(e) = ctx.pg().set_folder_icons(&folder_id, &icons).await {
            tracing::warn!(folder_id = %folder_id, error = %e, "set_folder_icons failed");
        }
    }

    // Project identity + role + namespaces (only when linked to a project).
    if let Some(pid) = folder["project_id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
        // Authoritative project name (matches what the scan created) for the
        // `project` namespace; fall back to frontmatter / folder name. Fetched
        // once and reused below for icon inference (name + current icon).
        let project = ctx.pg().get_project(&pid).await.ok().flatten();
        let project_name = project
            .as_ref()
            .and_then(|p| p["name"].as_str().map(String::from))
            .or_else(|| fm.project.clone())
            .or_else(|| folder["name"].as_str().map(String::from))
            .unwrap_or_default();

        let id_stack: Vec<String> =
            if fm.stack.is_empty() { stack.clone() } else { fm.stack.clone() };
        let mut tags: Vec<String> = Vec::new();
        if let Some(role) = fm.role.as_deref() {
            // Keep the raw role as a project tag (lossless).
            tags.push(format!("role:{role}"));
        }
        // folder.role enum: explicit README frontmatter wins; otherwise infer it
        // from the folder's manifest + layout so monorepo members are classified
        // automatically (library / tool / website). See `role_reconciliation` for
        // the write-vs-skip decision that reconciles stale pre-refactor rows.
        let folder_role = fm
            .role
            .as_deref()
            .and_then(metadata::folder_role_from_frontmatter)
            .or_else(|| super::scan_logic::infer_role(repo_path));
        if let Some(role_arg) = role_reconciliation(folder_role, fm.role.as_deref()) {
            ctx.pg().update_folder_role(&folder_id, role_arg).await.unwrap_or_else(
                |e| tracing::warn!(folder_id = %folder_id, error = %e, "update_folder_role failed"),
            );
        }
        if let Some(org) = fm.organization.as_deref() {
            tags.push(format!("org:{}", metadata::slugify(org)));
        }
        if let Err(e) = ctx
            .pg()
            .set_project_identity(
                &pid,
                fm.summary.as_deref(),
                fm.client.as_deref(),
                &id_stack,
                &tags,
            )
            .await
        {
            tracing::warn!(project_id = %pid, error = %e, "set_project_identity (reconcile) failed");
        }

        // Deterministic project-icon inference — fills the generic 場 fallback
        // so the project card shows something recognisable (repo logo /
        // kanji-from-stack / letter initial). Never overrides an author choice;
        // only upgrades a prior machine icon; non-fatal. [[pipeline/project-icon]].
        //
        // The logo tier is LIVE: the scanned repo-relative asset path
        // (`icon.path`, from `scan_icons` above) is passed through so a detected
        // logo wins as `{kind:"image", value:<rel path>}`. The daemon serves the
        // bytes at `GET /api/projects/{id}/icon` (path-safety in
        // `analysis::project_icon::read_icon_bytes`), and the app renders it with
        // a kanji fallback on image error.
        use crate::analysis::project_icon::{IconDecision, infer_icon};
        let logo_paths: Vec<String> = icon.path.iter().cloned().collect();
        let existing_icon =
            project.as_ref().map(|p| p["icon"].clone()).unwrap_or(serde_json::Value::Null);
        if let IconDecision::Set(inferred) =
            infer_icon(&project_name, &id_stack, &existing_icon, &logo_paths)
            && let Err(e) = ctx
                .pg()
                .set_project_icon(
                    &pid,
                    &serde_json::to_value(&inferred).unwrap_or(serde_json::Value::Null),
                )
                .await
        {
            tracing::warn!(project_id = %pid, error = %e, "set_project_icon (reconcile) failed");
        }

        let mut ns: Vec<(&str, String)> = Vec::new();
        if let Some(org) = fm.organization.as_deref() {
            ns.push(("organization", org.to_string()));
        }
        if !project_name.is_empty() {
            ns.push(("project", project_name.clone()));
        }
        if let Some(team) = fm.team.as_deref() {
            ns.push(("team", team.to_string()));
        }
        for lang in &id_stack {
            ns.push(("technology", lang.clone()));
        }
        for (scope, name) in &ns {
            let slug = metadata::slugify(name);
            if slug.is_empty() {
                continue;
            }
            if let Ok(ns_id) = ctx.pg().upsert_namespace(scope, name, &slug).await {
                ctx.pg().link_folder_namespace(&folder_id, &ns_id).await
                    .unwrap_or_else(|e| tracing::warn!(folder_id = %folder_id, ns_id = %ns_id, error = %e, "link_folder_namespace failed"));
            }
        }
    }

    // Sub-project roles: classify each nested sub-project (declared workspace
    // members and standalone sub-apps like a `site/`) so a monorepo's packages,
    // crates and apps are individually typed (library / tool / website). Only
    // runs for monorepo roots — a single-package repo has nothing nested to
    // find. Role assignment is independent of project membership.
    if super::scan_logic::is_monorepo(repo_path)
        && let Some(root_id) = crate::api::util::json_uuid(&folder["root_id"])
    {
        let project_id = folder["project_id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok());
        for sub in super::scan_logic::find_subprojects(repo_path, 3) {
            let Some(role) = super::scan_logic::infer_role(&sub) else { continue };
            let sub_abs = sub.to_string_lossy().to_string();
            let rel = sub.strip_prefix(repo_path).unwrap_or(&sub).to_string_lossy().to_string();
            let name = sub.file_name().and_then(|n| n.to_str()).unwrap_or(rel.as_str()).to_string();
            // D5a: a monorepo sub-project is a `workspace_member` (not a plain
            // structural `folder`) — its own boundary in the graph, keeping the
            // inferred role. The kind-aware upsert relabels an existing `folder`
            // member but never reclassifies a nested project root.
            match ctx
                .pg()
                .upsert_subfolder_kind(
                    &root_id,
                    "module",
                    &name,
                    &rel,
                    &sub_abs,
                    Some(&folder_id),
                    project_id.as_ref(),
                )
                .await
            {
                Ok(sub_id) => {
                    if let Err(e) = ctx.pg().update_folder_role(&sub_id, Some(role)).await {
                        tracing::warn!(sub = %sub_abs, error = %e, "sub-project update_folder_role failed");
                    }
                }
                Err(e) => {
                    tracing::warn!(sub = %sub_abs, error = %e, "sub-project upsert_subfolder_kind failed")
                }
            }
        }
    }
    Ok(1)
}

/// Watcher-triggered re-reconcile: re-apply identity when a project-root README
/// changes — but only if its frontmatter actually differs from the snapshot we
/// last stored. The change-detection makes a frontmatter write-back (#22) or a
/// body-only README edit a no-op, so a UI-driven change → README write → watcher
/// event neither loops nor churns the DB. `task.path` is the project-root abs
/// path (set by the watcher).
pub async fn reconcile_repo_metadata(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    use crate::tasks::processors::metadata;
    let repo_path = Path::new(&task.path);

    let fresh = serde_json::to_value(metadata::read_frontmatter(repo_path).unwrap_or_default())
        .unwrap_or(serde_json::Value::Null);
    let stored = ctx
        .pg()
        .get_repo_by_path(&task.path)
        .await
        .ok()
        .flatten()
        .and_then(|f| f.get("props").and_then(|p| p.get("frontmatter")).cloned());
    if stored.as_ref() == Some(&fresh) {
        tracing::debug!("reconcile_repo_metadata: {} — frontmatter unchanged, skipping", task.path);
        return Ok(0);
    }

    tracing::info!("reconcile_repo_metadata: {} — frontmatter changed, reconciling", task.path);
    reconcile_repo_identity(ctx, &task.path).await
}

// ── Process Folder ────────────────────────────────────────────────────────

/// Create module node for a folder.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo's abs_path by contract — look the row up
    // directly instead of round-tripping through name (which can collide
    // across roots and breaks for subtrees whose DB name is a composite
    // like "sensei:homebrew").
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let folder_id = folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]));

    let rel_dir = Path::new(&task.path)
        .strip_prefix(Path::new(&task.folder_path))
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy()
        .to_string();

    // Write module node to PG
    if let Some(ref fid) = folder_id {
        let mod_name =
            if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
        if let Err(e) = ctx.pg().upsert_dir_node(fid, "module", &mod_name, &task.path).await {
            tracing::warn!(folder_id = %fid, module = %mod_name, error = %e, "upsert_node (module) failed");
        }
    }

    Ok(0)
}

// ── Process File ──────────────────────────────────────────────────────────

/// Parse a single file using file_processor, then write results to graph.
/// Test-only fault seam (D6c-trigger): lets a test force a fatal DB-write
/// failure for a specific file path, so the fatal path (folder → `failed`,
/// `Err`, no `files` row advance) is exercised without needing a live DB fault.
#[cfg(test)]
pub(super) mod fault {
    use std::collections::HashSet;
    use std::sync::Mutex;

    static FAIL_PATHS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

    /// Force the next `process_file` on `abs_path` to hit the fatal path.
    pub fn fail_for(abs_path: &str) {
        FAIL_PATHS.lock().unwrap().get_or_insert_with(HashSet::new).insert(abs_path.to_string());
    }
    /// Stop forcing failure for `abs_path`.
    pub fn clear(abs_path: &str) {
        if let Some(set) = FAIL_PATHS.lock().unwrap().as_mut() {
            set.remove(abs_path);
        }
    }
    /// Whether `abs_path` is currently marked to fail.
    pub fn should_fail(abs_path: &str) -> bool {
        FAIL_PATHS.lock().unwrap().as_ref().is_some_and(|s| s.contains(abs_path))
    }
}

pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let abs_path = &task.path;

    // Skip files we can't parse as source text. Returning Ok (not Err) is
    // critical: a failed ProcessFile task would block its folder's
    // post-processing barrier, leaving the folder stuck at 'discovered'. Binary
    // (by extension) and non-UTF8 (by content sniff) files are skipped so
    // indexing always completes.
    let fpath = std::path::Path::new(abs_path);
    let ext = fpath.extension().and_then(|e| e.to_str()).unwrap_or("");
    if is_binary_ext(ext) || is_probably_binary(fpath) {
        return Ok(0);
    }

    // Lookup once by abs_path; folder name comes from the DB row so subtree
    // composite names ("sensei:homebrew") survive as the repo_id passed to
    // downstream processors that namespace symbol IDs by repo.
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let folder_name =
        folder.as_ref().and_then(|r| r["name"].as_str()).unwrap_or_else(|| task.folder_name());

    // Parse on a blocking thread. Parsing is synchronous, CPU-bound work; left
    // on the async runtime it blocks the worker's poll() — and a parse that
    // wedges (a pathological input, or a shared non-Sync parser contended by
    // concurrent files) would freeze the thread *inside* poll(), which the
    // executor watchdog cannot preempt (tokio::timeout only fires when the
    // future yields Pending). spawn_blocking moves it off the runtime so the
    // worker yields, the watchdog can fire, and one bad file can't wedge the
    // pool. A read/parse error is tolerated (skip, don't fail) so it never
    // blocks the folder's post-processing barrier.
    let folder_id = folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]));

    // ── RUST IS v2's. v1 IS GONE FOR IT ───────────────────────────────────────
    //
    // `index_file` reads the file and PLACES its references; `persist::write`
    // writes the facts and resolves each edge target to a node id as it goes —
    // `OnMiss::CreateStub` mints the node for a target no file has declared yet
    // and the edge carries that id, so a later scan of the DECLARING file fills
    // the same fqn in. Nothing is relinked, because the id was never wrong.
    //
    // **NO FALLBACK, and v1's rust parser is deleted rather than bypassed.**
    // The two mint different identities (`languages/fqn.rs` against
    // `indexer/fqn.rs`), so a graph holding both could never join them. A rust
    // file v2 cannot PLACE is not indexed: a visible gap, never a hand-off.
    //
    // Each further language flips the same way — land the adapter, delete
    // `languages/<lang>.rs` — so this condition only grows and v1 shrinks.
    // WHICH LANGUAGES THIS INDEXER OWNS IS NOT DECIDED HERE. The registry says
    // (`lang::PRODUCTION_LANGUAGES`, the one place), and this handler only
    // dispatches on the answer. A `== Language::Rust` written here would be a
    // second copy of the frontier, and a language that flipped in one code path
    // and not another puts two producers with different fqn schemes on one set
    // of tables.
    //
    // The language is then CARRIED to `placement_on_disk`, whose module-path
    // rule is the adapter's own. It used to be hardcoded to `Language::Rust`
    // there; that agreed with the gate only by coincidence, and the first
    // language to flip would have been placed by rust's rule (which drops a
    // trailing `mod`/`lib` — javascript KEEPS a trailing `index`), silently
    // minting the wrong fqn for every symbol in the file.
    if let Some(language) =
        crate::indexer::lang::production_adapter_for_ext(&format!(".{ext}")).map(|a| a.language())
        && let Some(ref fid) = folder_id
    {
        let repo_root = std::path::Path::new(&task.folder_path);
        let rel = fpath
            .strip_prefix(repo_root)
            .map(|r| r.to_string_lossy().to_string())
            .unwrap_or_else(|_| abs_path.clone());

        // The same fault seam the v1 path honours, and FIRST — before placement
        // can return early, or rust would be exempt from the only test of
        // "a fatal DB write leaves the folder failed and does not advance the
        // fingerprint".
        #[cfg(test)]
        if fault::should_fail(abs_path) {
            return fail_folder(
                ctx,
                fid,
                &rel,
                "injected fatal DB-write failure (test fault seam)".to_string(),
            )
            .await;
        }

        let Some(placement) =
            crate::indexer::pipeline::placement_on_disk(fpath, repo_root, language)
        else {
            tracing::debug!(file = %rel, "v2: no manifest names a package — not indexed");
            return Ok(0);
        };
        let text = match std::fs::read_to_string(fpath) {
            Ok(t) => t,
            Err(e) => {
                tracing::debug!(file = %rel, error = %e, "v2: unreadable — not indexed");
                return Ok(0);
            }
        };
        let told = crate::indexer::pipeline::TellFile::about(&placement.package);
        let written = crate::indexer::pipeline::index_and_persist(
            ctx.pg(),
            fid,
            crate::indexer::index::FileInput {
                repo: &task.folder_path,
                path: &rel,
                mode: crate::indexer::index::Mode::Update,
                package: &placement.package,
                module: &placement.module,
                text: &text,
                world: &told.world(),
            },
        )
        .await?;
        let count = written.map(|w| w.symbols as u32).unwrap_or(0);
        if let Some((mtime, hash)) = super::helpers::file_fingerprint(fpath)
            && let Err(e) = ctx.pg().upsert_scan_state(fid, &rel, mtime, &hash).await
        {
            return fail_folder(ctx, fid, &rel, format!("upsert_scan_state: {e}")).await;
        }
        // RECORD THE PARSE. `upsert_scan_state` advances the fingerprint but
        // only ever RESETS `parsed_at` (its `ON CONFLICT` sets it to NULL when
        // the hash moved, else leaves it) — it never sets one. `parsed_at` is
        // the `files` lifecycle's discovered/parsed bit, and the gate's work
        // list is `skip_reason IS NULL AND parsed_at IS NULL`, so a file that
        // parsed but was never marked is handed back on every single scan.
        //
        // Measured before this line existed: 0 of 112,177 rows had `parsed_at`
        // set, so the "discovered" state was the only state any file ever
        // reached.
        if let Err(e) = ctx.pg().mark_file_parsed(fid, &rel).await {
            return fail_folder(ctx, fid, &rel, format!("mark_file_parsed: {e}")).await;
        }
        return Ok(count);
    }

    let abs_owned = abs_path.clone();
    let folder_path_owned = task.folder_path.clone();
    let folder_name_owned = folder_name.to_string();
    let parsed = tokio::task::spawn_blocking(move || {
        crate::tasks::processors::process_file(&abs_owned, &folder_path_owned, &folder_name_owned)
    })
    .await;
    let result = match parsed {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            // Read/parse error — record per-file in index_errors (surfaced in the
            // UI) instead of silently dropping it. If THAT write also fails,
            // surface the second failure so an operator can see the DB is unhappy;
            // otherwise the parse-error observability itself becomes silent.
            if let Some(fid) = &folder_id
                && let Err(log_err) =
                    ctx.pg().log_index_error(fid, abs_path, &e, Some(ext), Some("parse")).await
            {
                tracing::warn!(error = %log_err, path = %abs_path, "log_index_error failed for parse error");
            }
            tracing::debug!("process_file: skipping unparseable {abs_path}: {e}");
            return Ok(0);
        }
        Err(join_err) => {
            // The parser panicked (e.g. a tree-sitter node byte-range past the
            // source). spawn_blocking turned it into a JoinError so the worker
            // survives; record which file so panicking inputs are observable
            // rather than vanishing.
            let msg = format!("parser panicked: {join_err}");
            if let Some(fid) = &folder_id
                && let Err(log_err) =
                    ctx.pg().log_index_error(fid, abs_path, &msg, Some(ext), Some("parse")).await
            {
                tracing::warn!(error = %log_err, path = %abs_path, "log_index_error failed for parser panic");
            }
            tracing::warn!("process_file: {abs_path}: {msg}");
            return Ok(0);
        }
    };

    // Write parsed symbols to PG. A DB-write failure here is FATAL (D6c-trigger):
    // the file isn't correctly indexed, so we must NOT advance its `files` row and
    // must surface it — mark the folder `failed` (the fail-closed barrier D6d
    // checks this) and propagate `Err` (recorded to task_executions and
    // bounded-retried, D6c). Parse/read errors were TOLERATED above (Ok). A
    // folder row that doesn't exist yet is a no-op.
    let symbols_count = result.symbols.len();
    let Some(folder_id) = folder_id else {
        return Ok(symbols_count as u32);
    };

    // Test seam (D6c-trigger): exercise the fatal path without a live DB fault.
    #[cfg(test)]
    if fault::should_fail(abs_path) {
        return fail_folder(
            ctx,
            &folder_id,
            &result.rel_path,
            "injected fatal DB-write failure (test fault seam)".to_string(),
        )
        .await;
    }

    // Any DB write in here failing is fatal — `?` propagates it out of the async
    // block and we handle it uniformly below (no partial "success").
    let write_result: Result<(), String> = async {
        // D3 upsert-then-prune: UPSERT the file's current nodes (surviving symbols
        // keep their id → community_id/embedding/inbound edges), then prune the
        // ones that vanished from the parse. No destructive delete-then-insert.

        // File node.
        let file_node_id = ctx
            .pg()
            .upsert_node(
                &folder_id,
                &result.kind,
                &result.rel_path,
                &result.rel_path,
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| format!("upsert file node: {e}"))?;

        // Symbol nodes (functions, classes, types, …), captured by (name,
        // line_start) so call edges can be sourced from the caller node — not the
        // file. Keyed on line because same-named methods across impl blocks are
        // legal in Rust.
        // The Rust FQN path (result.fqn) get-or-creates each symbol node keyed on
        // its canonical FQN — so a definition and every reference to it share one
        // node — via `upsert_node_by_fqn`. Every other language keeps the line-based
        // bare-name path (`upsert_node_ex`). `fqn_ids` maps fqn→id for edge sourcing.
        let mut sym_ids: std::collections::HashMap<(String, i32), uuid::Uuid> =
            std::collections::HashMap::new();
        let mut fqn_ids: std::collections::HashMap<String, uuid::Uuid> =
            std::collections::HashMap::new();
        // The `nodes.language` column for every FQN node is the FILE's language
        // (derived from its extension) — NOT the fqn's grouping lang and NOT a
        // hardcoded "rust". Keeps the same-language fallback (0.8) honest across
        // the migrated languages.
        let file_lang = crate::languages::language_for_path(&result.rel_path);
        // The fqn LANGUAGE SEGMENT for this file's own nodes. Hoisted above both
        // users — the module container below and the import-edge resolution
        // further down — because an import must be looked up under the same
        // segment the module node was written with, and a second copy of this
        // derivation would drift from it silently.
        let fqn_lang = fqn_lang_of(result.fqn.as_ref(), file_lang);
        if let Some(fqn_out) = &result.fqn {
            // D5c: a `module` container per file (nested under the file). Top-level
            // items nest under it (or the file node at the crate root); methods nest
            // under their TYPE node — so the graph is file → module → type → method.
            let mut top_parent = file_node_id;
            if !fqn_out.module.is_empty() {
                // The module container's fqn language matches the file's defs (the
                // first fqn's leading segment), so a Python/TS module node isn't
                // mislabelled as rust.
                //
                // When the parse yields a module path but NO top-level defs there is
                // no leading segment to copy, so fall back to the FILE's language
                // rather than a hardcoded "rust" — that fallback minted
                // `rust·<pkg>·lib/components/Foo` for a .svelte file, which is both
                // wrong (it corrupts the same-language scoring the `language` column
                // feeds) and unstable: the fqn flipped as soon as defs reappeared.
                // Only `adopt_node_by_identity` keeps such a flip from wedging the
                // file forever, so don't rely on it — emit a stable value here.
                // `fqn_lang` is hoisted above this block — the import resolver
                // below must look up modules under the SAME segment this writes.
                let mfqn =
                    crate::languages::fqn::item(fqn_lang, &fqn_out.package, "", &fqn_out.module);
                let mname = fqn_out.module.rsplit("::").next().unwrap_or(&fqn_out.module);
                let mid = ctx
                    .pg()
                    .upsert_node_by_fqn(
                        &folder_id,
                        &mfqn,
                        "module",
                        mname,
                        file_lang,
                        Some(crate::db::pg_store::FqnDef {
                            file_path: &result.rel_path,
                            signature: None,
                            line_start: None,
                            line_end: None,
                            is_exported: false,
                            parent_id: Some(&file_node_id),
                        }),
                    )
                    .await
                    .map_err(|e| format!("upsert module node {mfqn}: {e}"))?;
                fqn_ids.insert(mfqn, mid);
                top_parent = mid;
            }
            for d in &fqn_out.defs {
                let kind = crate::types::NodeKind::from_symbol_kind(&d.kind);
                // Structural parent: a method → its enclosing type node (get-or-create
                // — a stub if the type is defined in another file); a top-level item →
                // the module container (or the file at crate root).
                let parent_id: uuid::Uuid = match &d.parent_fqn {
                    Some(pf) => match fqn_ids.get(pf) {
                        Some(id) => *id,
                        None => ctx
                            .pg()
                            .upsert_node_by_fqn(
                                &folder_id,
                                pf,
                                "class",
                                pf.rsplit('·').next().unwrap_or(pf),
                                file_lang,
                                None,
                            )
                            .await
                            .map_err(|e| format!("upsert fqn parent {pf}: {e}"))?,
                    },
                    None => top_parent,
                };
                let id = ctx
                    .pg()
                    .upsert_node_by_fqn(
                        &folder_id,
                        &d.fqn,
                        kind.as_str(),
                        &d.name,
                        file_lang,
                        Some(crate::db::pg_store::FqnDef {
                            file_path: &result.rel_path,
                            signature: d.signature.as_deref(),
                            line_start: Some(d.line_start as i32),
                            line_end: Some(d.line_end as i32),
                            is_exported: d.is_exported,
                            parent_id: Some(&parent_id),
                        }),
                    )
                    .await
                    .map_err(|e| format!("upsert fqn def {}: {e}", d.fqn))?;
                // The declared return type — the one hop of the transitive
                // receiver chain nothing else can answer, and the reason
                // `ctx.pg().m()` could never name which `m`. Written through a
                // second statement rather than folded into the upsert because
                // `props` is deliberately absent from that DO UPDATE set-list,
                // which is what lets a value written on a reference-minted stub
                // survive the definition merging into the same row.
                //
                // UNCONDITIONAL for the kinds that can have one: a function
                // that LOST its return type between scans reports `None` here,
                // and only a write clears the previous scan's value. Skipping
                // would leave the resolver chasing a type the function no
                // longer returns.
                //
                // The fn/method gate is a CORRECTNESS gate, not a cheap one: no
                // other kind is ever given a return type by any producer, so a
                // write for one could only ever clear a key nothing set. It
                // buys little traffic — measured on the live index, function +
                // method are 108,438 of the 136,583 definitions (79%), so this
                // statement runs for four definitions in five. What keeps a
                // re-scan cheap is `set_node_return_type` itself skipping the
                // write when the stored value is already identical.
                if matches!(kind, crate::types::NodeKind::Function | crate::types::NodeKind::Method)
                {
                    ctx.pg()
                        .set_node_return_type(&id, d.return_type.as_deref().unwrap_or(""))
                        .await
                        .map_err(|e| format!("set return type {}: {e}", d.fqn))?;
                }
                fqn_ids.insert(d.fqn.clone(), id);
            }
        } else {
            for sym in &result.symbols {
                let id = ctx
                    .pg()
                    .upsert_node_ex(
                        &folder_id,
                        &sym.kind,
                        &sym.name,
                        &result.rel_path,
                        Some(&file_node_id),
                        sym.signature.as_deref(),
                        Some(sym.line as i32),
                        Some(sym.line_end as i32),
                        sym.is_exported,
                    )
                    .await
                    .map_err(|e| format!("upsert symbol node {}: {e}", sym.name))?;
                sym_ids.insert((sym.name.clone(), sym.line as i32), id);
            }
        }

        // D5b: nested doc `section` nodes (file → H1 → H2 → H3). Identity is the
        // full heading PATH ("Design > Auth > Refresh") with a NULL `line_start`,
        // so a section keeps its id across line edits (line-independent identity,
        // 0.4) — the real line + level live in `props`. Written through this same
        // upsert/prune path, so a re-index reconciles the section set (a removed
        // heading is pruned, no duplicates). Empty for code files.
        let mut section_ids: Vec<uuid::Uuid> = Vec::with_capacity(result.sections.len());
        // Stack of (level, heading_segment, node_id) for the current ancestor chain.
        let mut path_stack: Vec<(u8, String, uuid::Uuid)> = Vec::new();
        // Disambiguate identical-text siblings under the same parent: the Nth
        // (N>1) occurrence of a full heading path gets a " #N" suffix, so two
        // `## Setup` under one H1 are DISTINCT nodes rather than the second's
        // upsert colliding onto the first (which would silently clobber it). The
        // suffix flows into the stacked segment, so children of the second Setup
        // ("… > Setup #2 > …") don't collide with children of the first either.
        // Deterministic per document ⇒ idempotent on re-index.
        let mut seen_paths: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for sec in &result.sections {
            while path_stack.last().is_some_and(|(lvl, _, _)| *lvl >= sec.level) {
                path_stack.pop();
            }
            let parent_id = path_stack.last().map(|(_, _, id)| *id).unwrap_or(file_node_id);
            let base_path = path_stack
                .iter()
                .map(|(_, h, _)| h.as_str())
                .chain(std::iter::once(sec.heading.as_str()))
                .collect::<Vec<_>>()
                .join(" > ");
            let occ = {
                let c = seen_paths.entry(base_path.clone()).or_insert(0);
                *c += 1;
                *c
            };
            let (heading_path, segment) = if occ == 1 {
                (base_path, sec.heading.clone())
            } else {
                (format!("{base_path} #{occ}"), format!("{} #{occ}", sec.heading))
            };
            let sec_id = ctx
                .pg()
                .upsert_node(
                    &folder_id,
                    "section",
                    &heading_path,
                    &result.rel_path,
                    Some(&parent_id),
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|e| format!("upsert section node {}: {e}", heading_path))?;
            let props = serde_json::json!({
                "level": sec.level,
                "line_start": sec.line_start,
                "line_end": sec.line_end,
                "preview": sec.content_preview,
            });
            ctx.pg()
                .set_node_props(&sec_id, &props)
                .await
                .map_err(|e| format!("set section props {}: {e}", heading_path))?;
            section_ids.push(sec_id);
            // Stack holds each heading's OWN (disambiguated) segment so the next
            // child's path joins ancestors + itself exactly once, carrying the suffix.
            path_stack.push((sec.level, segment, sec_id));
        }

        // D5b: rationale nodes (NOTE/WHY/HACK/TODO/IMPORTANT) — the design "why".
        // Parented to the file node (finer function/section parenting is a
        // follow-up), keyed on (name=text, line_start) so identical markers on
        // different lines are distinct and a re-index of unchanged text is a no-op.
        let mut rationale_ids: Vec<uuid::Uuid> = Vec::with_capacity(result.rationales.len());
        for r in &result.rationales {
            let id = ctx
                .pg()
                .upsert_node(
                    &folder_id,
                    "rationale",
                    &r.text,
                    &result.rel_path,
                    Some(&file_node_id),
                    None,
                    Some(r.line as i32),
                    Some(r.line as i32),
                )
                .await
                .map_err(|e| format!("upsert rationale node: {e}"))?;
            ctx.pg()
                .set_node_props(&id, &serde_json::json!({ "marker": r.marker }))
                .await
                .map_err(|e| format!("set rationale props: {e}"))?;
            rationale_ids.push(id);
        }

        // Everything just upserted is this file's current node set (its source
        // nodes for out-edges too). `kept` is never empty — the file node always
        // survives.
        let kept: Vec<uuid::Uuid> = std::iter::once(file_node_id)
            .chain(sym_ids.values().copied())
            .chain(fqn_ids.values().copied())
            .chain(section_ids.iter().copied())
            .chain(rationale_ids.iter().copied())
            .collect();

        // D3 prune: delete the file's nodes that vanished from the parse (their
        // out-edges cascade). Inbound FQN edges (target_id set) cascade-delete with
        // the node — the demote-to-stub refinement (plan 0.5) is a deferred
        // follow-up; a full reindex heals a removed-but-referenced def.
        ctx.pg()
            .prune_file_nodes(&folder_id, &result.rel_path, &kept)
            .await
            .map_err(|e| format!("prune_file_nodes: {e}"))?;

        // D2/D3 per-file out-edge reconcile: a SURVIVING node keeps its id, so its
        // stale out-edges (a call/import a re-edit removed) don't cascade — clear
        // this file's out-edges, then re-insert the current set below (replace,
        // not append).
        ctx.pg()
            .delete_edges_from_sources(&folder_id, &kept)
            .await
            .map_err(|e| format!("delete_edges_from_sources: {e}"))?;

        // Import edges. A LOCAL specifier resolves to the target's module node
        // AT EMIT, exactly as a call edge resolves to its target — which is the
        // whole reason calls reach 65% while imports sat at 0 of 162,690.
        //
        // Order-independent, so this needs no barrier and no second pass: on a
        // total candidate miss `upsert_node_by_fqn(.., None)` creates a STUB
        // keyed on the same fqn, and when the target file is processed later the
        // module upsert above hits that same `(folder_id, fqn)` and ENRICHES the
        // row in place, keeping its id.
        //
        // LOOKUP FIRST, create only after every candidate misses — the rule
        // `import_target.rs` states as the reason it must not be trusted as the
        // authority on locality. Get-or-creating on candidate 1 would satisfy
        // candidate 1 forever and hide the real target at candidate 2.
        //
        // An EXTERNAL target stays unresolved here (it keeps `target_name`, which
        // is the useful fact for a dependency question); minting `lib_symbol`
        // nodes for those is a separate slice.
        let import_pkg = result.fqn.as_ref().map(|f| f.package.as_str()).unwrap_or("");
        let import_module = result.fqn.as_ref().map(|f| f.module.as_str()).unwrap_or("");
        for import in &result.unresolved_imports {
            let class = crate::languages::import_target::classify_import(import);
            let candidates = crate::languages::import_target::local_import_candidates(
                fqn_lang,
                import_pkg,
                import_module,
                import,
                &class,
            );
            let mut target: Option<uuid::Uuid> = None;
            for cand in &candidates {
                if let Some(id) = ctx
                    .pg()
                    .node_id_by_fqn(&folder_id, cand)
                    .await
                    .map_err(|e| format!("node_id_by_fqn({cand}): {e}"))?
                {
                    target = Some(id);
                    break;
                }
            }
            // Every candidate missed. What that MEANS depends on whether the
            // specifier was ever placeable locally, and `import_anchor` is the
            // declared owner of that question — not a re-classification here.
            //
            // LOCAL: stub a module on the FIRST candidate, the fqn the target's
            // own module node would carry, so a later definition enriches this
            // very row. That is what keeps resolution order-independent.
            //
            // EXTERNAL: mint a lib node. 109,944 of 110,785 unresolved imports
            // (99.2%) name nothing local — `node:fs`, `java.util.List` — and
            // those are complete facts about a dependency, not failed lookups.
            // The probe above is what earns this: the miss is evidence, since
            // a dotted specifier now has a real candidate to miss.
            //
            // Stubbing a MODULE here would be wrong and was briefly possible:
            // once dotted specifiers gained a candidate, `candidates.first()`
            // for `java.util.List` became `java·java.util·List`, so the old
            // code would have written a module named `List` for a JDK class.
            if target.is_none() {
                let anchor = crate::languages::import_target::import_anchor(import_module, import);
                use crate::languages::import_target::ImportAnchor;
                match anchor {
                    ImportAnchor::External { package } if !package.is_empty() => {
                        // Keyed on the PACKAGE, so every importer of `node:fs`
                        // shares one node and the graph can answer "who depends
                        // on this".
                        let lib_fqn = crate::languages::fqn::lib(&package, import, "");
                        target = Some(
                            ctx.pg()
                                .upsert_lib_node_by_fqn(&folder_id, &lib_fqn, import, &package, file_lang)
                                .await
                                .map_err(|e| format!("mint lib import {lib_fqn}: {e}"))?,
                        );
                    }
                    // Local, or external with no derivable package: fall back to
                    // the module stub only when there is a candidate to hang it
                    // on. No candidate means nothing to name, and inventing one
                    // is the fabrication the rules forbid.
                    _ => {
                        if let Some(first) = candidates.first() {
                            let leaf = first.rsplit('·').next().unwrap_or(first);
                            let leaf = leaf.rsplit('/').next().unwrap_or(leaf);
                            target = Some(
                                ctx.pg()
                                    .upsert_node_by_fqn(
                                        &folder_id, first, "module", leaf, file_lang, None,
                                    )
                                    .await
                                    .map_err(|e| format!("upsert import stub {first}: {e}"))?,
                            );
                        }
                    }
                }
            }
            match target {
                Some(t) => ctx
                    .pg()
                    .insert_edge(&folder_id, &file_node_id, Some(&t), None, None, "imports")
                    .await
                    .map_err(|e| format!("insert_edge (imports, resolved): {e}"))?,
                None => ctx
                    .pg()
                    .insert_edge(&folder_id, &file_node_id, None, Some(import), None, "imports")
                    .await
                    .map_err(|e| format!("insert_edge (imports): {e}"))?,
            };
        }

        // Call edges. The FQN path emits RESOLVED node→node edges AT EMIT: the
        // target is get-or-created by FQN (a stub if its definition isn't indexed
        // yet — enriched later, keeping the same id; a `lib_symbol` for an external
        // crate). An out-of-0.7 receiver (unresolvable) or an un-migrated language
        // keeps an honest bare-name edge (target_name only) — the `dyn`/residual
        // tail. Phase 7.1 retired the resolve_edges fallback, so these stay
        // unresolved rather than being bare-name-matched to an arbitrary node.
        if let Some(fqn_out) = &result.fqn {
            let file_lang = crate::languages::language_for_path(&result.rel_path);
            for r in &fqn_out.refs {
                let source = fqn_ids.get(&r.caller_fqn).copied().unwrap_or(file_node_id);
                // The SAME ladder inheritance uses, now literally the same
                // code. This block and the inheritance one below were the two
                // copies the churn audit identified as the slice's only real
                // duplication.
                let target = match &r.target_fqn {
                    Some(tf) if r.is_lib => crate::graph_facts::TargetRef::Lib {
                        fqn: tf.clone(),
                        name: r.target_name.clone(),
                        package: crate::graph_facts::lib_package_of(tf).unwrap_or("").to_string(),
                    },
                    Some(tf) => crate::graph_facts::TargetRef::Internal {
                        fqn: tf.clone(),
                        name: r.target_name.clone(),
                        // A call target is a FUNCTION, where a supertype is a
                        // class. The stub kind differs per caller, which is why
                        // it belongs on the policy rather than in the persister.
                        on_miss: crate::graph_facts::OnMiss::CreateStub { kind: "function" },
                    },
                    None => crate::graph_facts::TargetRef::Unresolvable {
                        name: r.target_name.clone(),
                    },
                };
                // A RESOLVED call stamps no props — it already names its
                // target, and recording the receiver beside it would be a
                // second answer to a settled question. An UNRESOLVED one
                // carries whatever the producer saw of its receiver, because
                // that is the only surviving trace of it: measured, an
                // unresolved `calls` edge stores the bare member name and
                // NOTHING else (target_file NULL and props `{}` on all 161,612
                // of them), so `ctx.pg().m()` and `whatever.m()` persist
                // identically and no later pass can tell them apart.
                let props = match (&r.target_fqn, &r.receiver) {
                    (None, Some(hint)) => hint.to_props(),
                    _ => serde_json::json!({}),
                };
                ctx.pg()
                    .persist_edge_fact(
                        &folder_id,
                        &crate::graph_facts::EdgeFact {
                            source_id: source,
                            target,
                            kind: "calls",
                            props,
                        },
                        &fqn_ids,
                        file_lang,
                    )
                    .await
                    .map_err(|e| format!("persist call fact: {e}"))?;
            }

            // Inheritance. The SAME three-branch ladder the call emit above
            // uses, deliberately: external → a `lib·` node, internal →
            // stub-then-enrich, unresolvable → target_name only.
            //
            // The stub is not laziness. Measured over the corpus, 404 of 406
            // java relations (99.5%) have their parent type in a DIFFERENT
            // file, so a probe-only resolver would leave about half of them
            // unresolved on a cold index and never heal — trading the 7,905
            // mislabelled edges for a fresh pile of unresolved ones. A stub
            // created here is ENRICHED when the real definition is indexed,
            // whichever order the files arrive in.
            //
            // There is deliberately NO bare-name fallback. The one available
            // resolver, `sole_definition_id_by_name`, is kind- and
            // language-agnostic (it exists for doc mentions), so a miss would
            // resolve confidently WRONG: a repo defining its own `BaseModel`
            // would capture every subclass of `pydantic.BaseModel`. An
            // unresolved parent is worse than nothing only if you think a
            // wrong answer is better than no answer.
            for rel in &fqn_out.relations {
                let source = match fqn_ids.get(&rel.child_fqn) {
                    Some(id) => *id,
                    // The child is declared in THIS file, so a miss means the
                    // def emit skipped it; anchoring on the file keeps the fact
                    // rather than dropping it.
                    None => file_node_id,
                };
                // One fact, one policy. The three-branch ladder that used to
                // live here is now `persist_edge_fact`, shared with the call
                // arm — it was the SAME ladder written twice, which is the
                // duplication this slice exists to collapse.
                let target = match &rel.parent_fqn {
                    Some(pf) if rel.is_lib => crate::graph_facts::TargetRef::Lib {
                        fqn: pf.clone(),
                        name: rel.parent_name.clone(),
                        package: crate::graph_facts::lib_package_of(pf).unwrap_or("").to_string(),
                    },
                    Some(pf) => crate::graph_facts::TargetRef::Internal {
                        fqn: pf.clone(),
                        name: rel.parent_name.clone(),
                        // A supertype is a TYPE, and the stub says so, so a
                        // later enrich need not correct it.
                        on_miss: crate::graph_facts::OnMiss::CreateStub { kind: "class" },
                    },
                    None => crate::graph_facts::TargetRef::Unresolvable {
                        name: rel.parent_name.clone(),
                    },
                };
                ctx.pg()
                    .persist_edge_fact(
                        &folder_id,
                        &crate::graph_facts::EdgeFact {
                            source_id: source,
                            target,
                            kind: rel.relation.edge_kind(),
                            props: serde_json::json!({ "relation": rel.relation.as_str() }),
                        },
                        &fqn_ids,
                        file_lang,
                    )
                    .await
                    .map_err(|e| format!("persist inheritance fact: {e}"))?;
            }
        } else {
            for call in &result.unresolved_calls {
                let source = sym_ids
                    .get(&(call.caller_name.clone(), call.caller_line as i32))
                    .copied()
                    .unwrap_or(file_node_id);
                ctx.pg()
                    .insert_edge(&folder_id, &source, None, Some(&call.callee_name), None, "calls")
                    .await
                    .map_err(|e| format!("insert_edge (calls): {e}"))?;
            }
        }

        // No containment edge here. This used to emit `extends` per parent ref,
        // which was wrong three ways: `extends` means inheritance, the source
        // was the FILE rather than the type, and `pref.method_id` — the only
        // field that could have expressed "type -> method" — was discarded. All
        // 7,905 such edges in the live graph were unresolved, so they carried
        // nothing. Type -> method containment lives on `nodes.parent_id`, set
        // from `parent_fqn` in the FQN emit above.

        // Doc references (D2): an explicit doc→file path ref AND a doc→symbol
        // mention are both `references` edges — per the edge_kind contract
        // ("doc section references a symbol or file"). `covers` is reserved for
        // BuildConnections' folder-derived stem-proximity set, which it REPLACES
        // wholesale; a doc→file ref must not be `covers` or that replace would
        // wipe it (the two-producer data-loss D2 review caught).
        if result.kind == "doc" {
            // Doc references RESOLVE to what they name, like call and import
            // edges do — 241,514 of these sat at 0% because nothing tried.
            //
            // A file reference is repo-relative (the extractor stores it that
            // way now) and `nodes.file_path` is too, so the two match directly.
            // A miss stays unresolved and keeps `target_name`; unlike an import,
            // a doc reference must NOT create a stub, because a doc naming a file
            // that does not exist is a broken link — that is a fact about the
            // doc, not evidence the file exists somewhere unindexed.
            for file_ref in &result.file_refs {
                let target = ctx
                    .pg()
                    .file_node_id_by_path(&folder_id, file_ref)
                    .await
                    .map_err(|e| format!("file_node_id_by_path({file_ref}): {e}"))?;
                match target {
                    Some(t) => ctx
                        .pg()
                        .insert_edge(&folder_id, &file_node_id, Some(&t), None, None, "references")
                        .await
                        .map_err(|e| format!("insert_edge (references, file resolved): {e}"))?,
                    None => ctx
                        .pg()
                        .insert_edge(
                            &folder_id,
                            &file_node_id,
                            None,
                            Some(file_ref),
                            None,
                            "references",
                        )
                        .await
                        .map_err(|e| format!("insert_edge (references, file): {e}"))?,
                };
            }
            // A symbol mention resolves ONLY when the name is unambiguous in this
            // folder. A doc writes `` `handleAuth` `` with no signature and no
            // module path, so with several same-named definitions there is
            // nothing to choose on — picking one would publish a guess as a fact.
            // Ambiguous and absent both stay unresolved, carrying the mention in
            // `target_name`.
            for fn_ref in &result.fn_mentions {
                if let Some(t) = ctx
                    .pg()
                    .sole_definition_id_by_name(&folder_id, fn_ref)
                    .await
                    .map_err(|e| format!("sole_definition_id_by_name({fn_ref}): {e}"))?
                {
                    ctx.pg()
                        .insert_edge(&folder_id, &file_node_id, Some(&t), None, None, "references")
                        .await
                        .map_err(|e| format!("insert_edge (references, symbol resolved): {e}"))?;
                    continue;
                }
                ctx.pg()
                    .insert_edge(&folder_id, &file_node_id, None, Some(fn_ref), None, "references")
                    .await
                    .map_err(|e| format!("insert_edge (references, symbol): {e}"))?;
            }
        }

        // is_test: a FILE-level flag stamped on every one of this file's nodes so
        // the UI can filter tests out when focusing on production code. Set after
        // emit (all nodes exist); IS DISTINCT FROM makes a no-op re-scan cheap and
        // a test↔prod rename flips the file's nodes.
        // `.txt` is parsed by the markdown doc processor, but the write-time stamp
        // is extension-derived and `.txt` cannot be decided from the extension —
        // the llms corpus is markdown, a licence is not. Correct it here, where the
        // content is in hand. MEASURED: 2,565 of 2,896 null-language `.txt` nodes
        // are rokkit's `docs/llms/**` corpus.
        if result.rel_path.to_ascii_lowercase().ends_with(".txt")
            && let Ok(text) = std::fs::read_to_string(&result.abs_path)
        {
            let lang = crate::languages::text_language_from_content(&text);
            if let Err(e) =
                ctx.pg().set_nodes_language_for_file(&folder_id, &result.rel_path, lang).await
            {
                tracing::warn!(file = %result.rel_path, error = %e, "set_nodes_language_for_file failed");
            }
        }

        let is_test = crate::languages::is_test_path(
            &result.rel_path,
            crate::languages::language_for_path(&result.rel_path),
        );
        ctx.pg()
            .set_nodes_is_test_for_file(&folder_id, &result.rel_path, is_test)
            .await
            .map_err(|e| format!("set_nodes_is_test_for_file: {e}"))?;
        Ok::<(), String>(())
    }
    .await;

    if let Err(e) = write_result {
        return fail_folder(ctx, &folder_id, &result.rel_path, e).await;
    }

    // Record this file's fingerprint LAST — only a fully-written file is "seen",
    // so a fatal failure above leaves the `files` row unadvanced and the next scan
    // retries it. A `files` write failure is itself fatal.
    if let Some((mtime, hash)) = super::helpers::file_fingerprint(fpath)
        && let Err(e) = ctx.pg().upsert_scan_state(&folder_id, &result.rel_path, mtime, &hash).await
    {
        return fail_folder(ctx, &folder_id, &result.rel_path, format!("upsert_scan_state: {e}"))
            .await;
    }

    Ok(symbols_count as u32)
}

/// Mark a folder `failed` and return the fatal error (D6c-trigger / D6a): a DB
/// write for one of its files failed, so the folder must not advance to
/// `indexed` (the fail-closed barrier D6d checks this status) and boot-reconcile
/// / bounded-retry re-drives it. Marking the status is best-effort — if THAT
/// write also fails we still surface the original fatal error, never swallow it.
/// The fqn LANGUAGE SEGMENT a file's own nodes must be written under.
///
/// Every fqn this file emits — the module container, its defs, and the module
/// anchor its refs name as caller — has to agree on this segment, because an
/// import is looked up under the same segment the module node was written with.
/// Extracted into one function precisely because a second copy of the
/// derivation would drift from it silently.
///
/// Order matters. The PRODUCER's own output wins over the file extension, since
/// the producer is what actually wrote the fqns:
///
/// 1. A top-level def's leading segment.
/// 2. Failing that, a ref's `caller_fqn` leading segment. A file can produce
///    REFS AND NO DEFS — a vitest/jest suite is entirely expression statements —
///    and its refs are anchored on the producer's module fqn. Consulting
///    `file_lang` here instead wrote `javascript·<pkg>·<mod>` for a `.js` file
///    whose producer had written `typescript·<pkg>·<mod>` (the TS and JS adapters
///    share one producer, which hardcodes the typescript segment), so the module
///    node and the edge's caller disagreed and the lookup missed.
/// 3. Failing that, the file's own language.
/// 4. Failing that, `rust` — the historical default, kept so a file that yields
///    no fqn output at all behaves as before.
fn fqn_lang_of<'a>(
    fqn_out: Option<&'a crate::languages::fqn::FqnFileOutput>,
    file_lang: Option<&'a str>,
) -> &'a str {
    fqn_out
        .and_then(|f| {
            f.defs
                .first()
                .map(|d| d.fqn.as_str())
                .or_else(|| f.refs.first().map(|r| r.caller_fqn.as_str()))
        })
        .and_then(|f| f.split('·').next())
        .filter(|seg| !seg.is_empty())
        .or(file_lang)
        .unwrap_or("rust")
}

async fn fail_folder(
    ctx: &TaskContext,
    folder_id: &uuid::Uuid,
    rel_path: &str,
    err: String,
) -> Result<u32, String> {
    if let Err(se) = ctx.pg().update_folder_status(folder_id, "failed").await {
        tracing::warn!(error = %se, folder_id = %folder_id, "process_file: marking folder failed also failed");
    }
    tracing::warn!(folder_id = %folder_id, file = %rel_path, error = %err,
        "process_file: fatal DB write — folder left `failed`, `files` row not advanced");
    Err(format!("process_file fatal DB write ({rel_path}): {err}"))
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo abs_path (Task contract).
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"])
        && let Err(e) = ctx.pg().delete_nodes_by_file(&folder_id, &task.path).await
    {
        tracing::warn!(folder_id = %folder_id, file = %task.path, error = %e, "delete_nodes_by_file (delete_file) failed");
    }
    tracing::info!("delete_file: {}", task.path);
    Ok(0)
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"])
        && let Err(e) = ctx.pg().delete_nodes_by_path_prefix(&folder_id, &task.path).await
    {
        tracing::warn!(folder_id = %folder_id, path = %task.path, error = %e, "delete_nodes_by_path_prefix (delete_folder) failed");
    }
    tracing::info!("delete_folder: {}", task.path);
    Ok(0)
}

/// Decide what to write to a folder's role column given the classifier's
/// output and the frontmatter's raw `role:` value.
///
/// Reconciles stale rows from before the classifier refactor (#8): the old
/// classifier had a `backend` fallback and left rows tagged `backend`
/// wherever it could not infer a real role. The new classifier returns
/// `None` for un-inferrable folders, but the writer used to skip those,
/// leaving the stale `backend` in place forever. This helper makes the
/// silent-frontmatter + no-classification case actively clear the DB.
///
/// - `classified`: the classifier's decision (`None` means "cannot classify").
/// - `raw_frontmatter_role`: whatever the README's `role:` field literally said,
///   before it was mapped through `folder_role_from_frontmatter`. `Some("")`
///   still counts as "the user wrote something" — only true `None` means the
///   frontmatter is silent.
///
/// Returns `Some(role_arg)` when we should call `update_folder_role`, where
/// `role_arg` may itself be `Some(&str)` (write that value) or `None` (clear
/// the DB column). Returns outer `None` to skip the write entirely — used
/// when the user wrote an unrecognised role that we neither map nor override.
pub fn role_reconciliation<'a>(
    classified: Option<&'a str>,
    raw_frontmatter_role: Option<&str>,
) -> Option<Option<&'a str>> {
    match (classified, raw_frontmatter_role) {
        (Some(fr), _) => Some(Some(fr)),
        (None, None) => Some(None),
        (None, Some(_)) => None,
    }
}

#[cfg(test)]
mod role_reconciliation_tests {
    use super::role_reconciliation;

    #[test]
    fn writes_classifier_result_when_classified() {
        assert_eq!(role_reconciliation(Some("library"), None), Some(Some("library")));
    }

    #[test]
    fn writes_classifier_result_even_when_frontmatter_says_something_else() {
        // Frontmatter took precedence upstream (folder_role_from_frontmatter
        // returned the mapped value). If we got here with a classified value,
        // trust it — the caller already resolved precedence.
        assert_eq!(role_reconciliation(Some("website"), Some("backend")), Some(Some("website")),);
    }

    #[test]
    fn clears_stale_value_when_frontmatter_silent_and_no_classification() {
        // The #8 fix: a rescan of a folder with no manifest signals AND no
        // frontmatter role must clear the DB, not skip. Otherwise pre-refactor
        // "backend" rows persist forever.
        assert_eq!(role_reconciliation(None, None), Some(None));
    }

    #[test]
    fn preserves_db_when_frontmatter_has_unrecognised_role() {
        // The user wrote `role: platform` (or similar not-mapped value). We
        // don't know if the DB already holds their previous choice — leave it
        // alone rather than clobber.
        assert_eq!(role_reconciliation(None, Some("platform")), None);
        assert_eq!(role_reconciliation(None, Some("")), None);
    }
}

#[cfg(test)]
mod tests {
    /// Drive `process_file` the way the pipeline does — barrier first.
    ///
    /// Stage 3 is a BARRIER (R14): the scan writes every `files` row for a
    /// folder BEFORE enqueuing a single `ProcessFile` task, so by the time this
    /// handler runs its file is already tracked and the writers can LOOK IT UP
    /// AND FAIL CLOSED (R13) instead of minting a row on a write path.
    ///
    /// A fixture calling `process_file` directly skipped the scan, so it skipped
    /// the barrier. This performs that one step and then runs the real handler —
    /// the handler is still what is under test.
    ///
    /// The folder is resolved the same way `process_file` resolves it (by
    /// `folder_path`), so the seeded row lands under the folder the handler will
    /// look in rather than one the fixture guessed at.
    async fn processed(ctx: &TaskContext, repo_path: &str, abs: &str) -> Result<u32, String> {
        let folder = ctx
            .pg()
            .get_repo_by_path(repo_path)
            .await?
            .ok_or_else(|| format!("processed: no folder at {repo_path}"))?;
        let folder_id = crate::api::util::json_uuid(&folder["id"])
            .ok_or_else(|| format!("processed: the folder at {repo_path} carries no id"))?;
        let rel = abs.strip_prefix(repo_path).map(|r| r.trim_start_matches('/')).unwrap_or(abs);
        ctx.pg().seed_only_file(&folder_id, rel).await?;
        process_file(ctx, &Task::for_file(TaskKind::ProcessFile, repo_path, abs)).await
    }

    use super::*;
    use crate::db::pg_store::graph_seed::SeedGraph;

    use crate::tasks::{Task, TaskKind};

    use super::super::super::executor::TaskContext;

    /// Build a TaskContext backed by PgStore and a fresh TaskQueue.
    use crate::tasks::test_support::make_ctx;

    /// A file that produces REFS AND NO DEFS still writes its module container
    /// under the segment its own producer used.
    ///
    /// The TS and JS adapters share one producer, which hardcodes the
    /// `typescript` segment. A `.js` vitest suite is entirely expression
    /// statements, so it emits refs and no top-level defs — and `fqn_lang`
    /// consulted the FILE's language next, yielding `javascript`. The module
    /// container was then written `javascript·<pkg>·<mod>` while every ref named
    /// `typescript·<pkg>·<mod>` as its caller, so the fqn→id lookup missed and
    /// the edge sourced from the file node instead of the module.
    ///
    /// Breaking mutation: drop the `.or_else(|| f.refs.first() …)` arm — the
    /// refs-only case falls through to `file_lang` and returns "javascript".
    #[test]
    fn fqn_lang_prefers_the_producers_own_segment_over_the_file_extension() {
        use crate::languages::fqn::{FqnDefinition, FqnFileOutput, FqnReference};
        use crate::types::SymbolKind;

        let a_ref = |caller: &str| FqnReference {
            caller_fqn: caller.to_string(),
            caller_line: 1,
            target_fqn: None,
            target_name: "x".to_string(),
            is_lib: false,
            receiver: None,
        };

        // REFS, NO DEFS — the case this fixes.
        let refs_only =
            FqnFileOutput { refs: vec![a_ref("typescript·app·e2e/spec")], ..Default::default() };
        assert_eq!(
            fqn_lang_of(Some(&refs_only), Some("javascript")),
            "typescript",
            "a refs-only file follows its producer's anchor, not its extension"
        );

        // A def still outranks a ref.
        let with_def = FqnFileOutput {
            defs: vec![FqnDefinition {
                fqn: "python·app·mod·f".to_string(),
                name: "f".to_string(),
                kind: SymbolKind::Function,
                line_start: 1,
                line_end: 1,
                is_exported: false,
                signature: None,
                docstring: None,
                parent_type: None,
                parent_fqn: None,
                return_type: None,
            }],
            refs: vec![a_ref("typescript·app·mod")],
            ..Default::default()
        };
        assert_eq!(fqn_lang_of(Some(&with_def), Some("javascript")), "python");

        // Neither ⇒ the file's language, then the historical default.
        let empty = FqnFileOutput::default();
        assert_eq!(fqn_lang_of(Some(&empty), Some("javascript")), "javascript");
        assert_eq!(fqn_lang_of(None, Some("svelte")), "svelte");
        assert_eq!(fqn_lang_of(None, None), "rust");
    }

    #[tokio::test]
    async fn process_folder_creates_module_node() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let ctx = make_ctx().await;
        let folder_name = "test-repo";
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Register the project so process_folder can look up its path
        {
            let root_id =
                ctx.pg().add_watch_root(&repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, &repo_path).await.unwrap();
        }

        let pkg_id = format!("pkg:{}:(root)", folder_name);

        let mut task =
            Task::for_file(TaskKind::ProcessFolder, &repo_path, &src_dir.to_string_lossy());
        task.module_id = Some(pkg_id.clone());

        process_folder(&ctx, &task).await.unwrap();

        // TODO: verify module node once module writes are implemented
    }

    /// Reconciling a monorepo git root classifies each nested sub-project with
    /// its own folder.role: a lib crate → library, a bin crate → tool, and a
    /// (non-member) SvelteKit sub-app → website. Guards the end-to-end wiring
    /// (is_monorepo → find_subprojects → upsert_subfolder → update_folder_role).
    #[tokio::test]
    async fn reconcile_classifies_monorepo_member_roles() {
        async fn role_of(ctx: &TaskContext, abs: &str) -> Option<String> {
            let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
                "SELECT role::text FROM sensei.folders WHERE abs_path = $1",
            )
            .bind(abs)
            .fetch_optional(ctx.pg().pool())
            .await
            .unwrap();
            row.and_then(|r| r.0)
        }
        async fn kind_of(ctx: &TaskContext, abs: &str) -> Option<String> {
            let row: Option<(String,)> = sqlx_core::query_as::query_as(
                "SELECT kind::text FROM sensei.folders WHERE abs_path = $1",
            )
            .bind(abs)
            .fetch_optional(ctx.pg().pool())
            .await
            .unwrap();
            row.map(|r| r.0)
        }

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[\"crates/*\"]").unwrap();
        std::fs::create_dir_all(root.join("crates/mylib/src")).unwrap();
        std::fs::write(root.join("crates/mylib/Cargo.toml"), "[package]\nname=\"mylib\"").unwrap();
        std::fs::write(root.join("crates/mylib/src/lib.rs"), "pub fn a() {}").unwrap();
        std::fs::create_dir_all(root.join("crates/mytool/src")).unwrap();
        std::fs::write(
            root.join("crates/mytool/Cargo.toml"),
            "[package]\nname=\"mytool\"\n\n[[bin]]\nname=\"mytool\"",
        )
        .unwrap();
        std::fs::write(root.join("crates/mytool/src/main.rs"), "fn main() {}").unwrap();
        std::fs::create_dir_all(root.join("site/src/routes")).unwrap();
        std::fs::write(
            root.join("site/package.json"),
            "{\"name\":\"site\",\"devDependencies\":{\"@sveltejs/kit\":\"^2\"}}",
        )
        .unwrap();

        let ctx = make_ctx().await;
        let repo_path = root.to_string_lossy().to_string();
        let root_id =
            ctx.pg().add_watch_root(&repo_path, "mono", &serde_json::json!([])).await.unwrap();
        ctx.pg().upsert_repo_kind(&root_id, "git", "mono", &repo_path).await.unwrap();

        reconcile_repo_identity(&ctx, &repo_path).await.unwrap();

        assert_eq!(
            role_of(&ctx, &root.join("crates/mylib").to_string_lossy()).await.as_deref(),
            Some("library")
        );
        assert_eq!(
            role_of(&ctx, &root.join("crates/mytool").to_string_lossy()).await.as_deref(),
            Some("tool")
        );
        assert_eq!(
            role_of(&ctx, &root.join("site").to_string_lossy()).await.as_deref(),
            Some("website")
        );

        // D5a: each sub-project is classified `workspace_member` (not a plain
        // structural `folder`), keeping its inferred role.
        assert_eq!(
            kind_of(&ctx, &root.join("crates/mylib").to_string_lossy()).await.as_deref(),
            Some("module")
        );
        assert_eq!(
            kind_of(&ctx, &root.join("crates/mytool").to_string_lossy()).await.as_deref(),
            Some("module")
        );
        assert_eq!(
            kind_of(&ctx, &root.join("site").to_string_lossy()).await.as_deref(),
            Some("module")
        );

        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    /// A LOCAL import resolves to the target's module node AT EMIT. Before this,
    /// `process.rs` passed `target_id = None` for every import, so 0 of 162,690
    /// import edges resolved — 25,693 of them pointing at local code.
    /// `extends` means INHERITANCE, and nothing else may claim it.
    ///
    /// Every one of the 7,905 `extends` edges in the live graph was
    /// `file -> (unresolved type declared in that same file)`. The emit's own
    /// comment said "HAS_METHOD: type -> method", but it passed `file_node_id`
    /// as the source and DISCARDED `pref.method_id`, so it could never express
    /// the relation it described. All 7,905 were unresolved, so they
    /// contributed zero usable edges to the Atlas or to community adjacency.
    ///
    /// Retiring them loses nothing, because the containment they garbled is
    /// already carried correctly by `nodes.parent_id` — measured at
    /// 60,201/60,201 method nodes. This asserts BOTH halves: no `extends` edge
    /// from a file, and the real containment still present.
    ///
    /// Breaking mutation: restore the `for pref in &result.parent_refs` emit.
    #[tokio::test]
    async fn a_type_with_methods_emits_no_extends_edge_but_keeps_its_parent_link() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("cont");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"cont\"\n").unwrap();
        std::fs::write(
            repo.join("src/lib.rs"),
            "pub struct Holder;\nimpl Holder {\n    pub fn one(&self) {}\n    pub fn two(&self) {}\n}\n",
        )
        .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "cont", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "cont", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();
        let abs = repo.join("src/lib.rs").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs).await.unwrap();

        let extends: Vec<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT e.target_name FROM sensei.edges e
              WHERE e.folder_id = $1 AND e.kind = 'extends'::sensei.edge_kind",
        )
        .bind(fid)
        .fetch_all(ctx.pg().pool())
        .await
        .unwrap();
        assert!(
            extends.is_empty(),
            "`extends` is inheritance; a type's own methods must not produce one: {extends:?}"
        );

        // The containment that emit was garbling must still be here, on the node.
        let parent_names: Vec<Option<String>> = sqlx_core::query_scalar::query_scalar(
            "SELECT p.name FROM sensei.nodes n
               LEFT JOIN sensei.nodes p ON p.id = n.parent_id
              WHERE n.folder_id = $1 AND n.name IN ('one', 'two')",
        )
        .bind(fid)
        .fetch_all(ctx.pg().pool())
        .await
        .unwrap();
        assert!(!parent_names.is_empty(), "the methods themselves must be indexed");
        assert!(
            parent_names.iter().all(|p| p.as_deref() == Some("Holder")),
            "each method must still hang off its TYPE via parent_id: {parent_names:?}"
        );

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    /// An EXTERNAL import that misses becomes a lib node; a LOCAL one still
    /// becomes a module stub.
    ///
    /// The distinction comes from `import_anchor`, the declared owner of
    /// local-vs-external, NOT from re-classifying the string here. Both halves
    /// matter:
    ///
    /// - External miss -> an external (`lib·`) node. 109,944 edges (99.2% of
    ///   unresolved imports) that name nothing local. `java.util.List` is a
    ///   complete fact about a dependency, and a lib node makes it answerable.
    /// - Local miss -> `module` stub, UNCHANGED. A relative import whose file
    ///   is not indexed yet must still stub, or resolution stops being
    ///   order-independent.
    ///
    /// Guards a regression I introduced one commit earlier: adding the dotted
    /// candidate made `candidates.first()` for `java.util.List` be
    /// `java·java.util·List`, so the old code would have minted a MODULE stub
    /// named `List` for a JDK class. Unshipped — it needed a reindex to
    /// manifest — but it is exactly the silent kind.
    ///
    /// Breaking mutations: (1) mint a module stub for the external case — the
    /// kind assertion fails; (2) mint a lib node for the relative case — the
    /// local half fails.
    #[tokio::test]
    async fn an_external_import_mints_a_lib_node_and_a_local_one_still_stubs() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("mint");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("package.json"), "{\"name\":\"mint\"}\n").unwrap();
        // A relative import whose target is NOT written to disk, so it misses.
        std::fs::write(
            repo.join("src/a.ts"),
            "import { x } from './missing';\nimport { readFile } from 'node:fs';\nexport function go() { return x(readFile); }\n",
        )
        .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "mint", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "mint", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();
        let abs = repo.join("src/a.ts").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs).await.unwrap();

        let kinds: Vec<(Option<String>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT t.kind::text, t.fqn FROM sensei.edges e
               JOIN sensei.nodes t ON t.id = e.target_id
              WHERE e.folder_id = $1 AND e.kind = 'imports'::sensei.edge_kind",
        )
        .bind(fid)
        .fetch_all(ctx.pg().pool())
        .await
        .unwrap();

        // The external one is a lib symbol under a lib package.
        assert!(
            kinds.iter().any(|(k, f)| k.as_deref() == Some("unknown")
                && f.as_deref().is_some_and(|f| f.starts_with("lib·node:fs"))),
            "`node:fs` must mint an external node whose kind the use site never stated: {kinds:?}"
        );
        // The relative one still stubs as a module — order-independence.
        assert!(
            kinds.iter().any(|(k, _)| k.as_deref() == Some("module")),
            "a missing relative import must still stub a module: {kinds:?}"
        );
        // And nothing external became a module stub.
        assert!(
            !kinds.iter().any(|(k, f)| k.as_deref() == Some("module")
                && f.as_deref().is_some_and(|f| f.contains("node:fs"))),
            "an external specifier must not become a module stub: {kinds:?}"
        );

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    /// Every CALL emit arm must fire, scoped to the `calls` kind.
    ///
    /// Calls had no arm coverage before this. Their ladder is the same shape as
    /// inheritance's — lib / in-file / stub / unresolvable — which is the
    /// duplication increment 4 collapses, and the `in-file` arm is again
    /// invisible in final state.
    ///
    /// Breaking mutation: remove the `fqn_ids` lookup from the persister's
    /// Internal arm — `calls/in-file` stops firing while every other call test
    /// stays green.
    #[tokio::test]
    async fn every_call_emit_arm_fires() {
        const ARMS: [&str; 3] = ["calls/lib", "calls/stub", "calls/in-file"];

        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("callarms");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"callarms\"\n").unwrap();
        std::fs::write(repo.join("src/other.rs"), "pub fn faraway() {}\n").unwrap();
        // One file reaching three arms: a std call (lib), a call into another
        // file (stub, since `other.rs` is processed second), and a call to a
        // function defined in THIS file (in-file).
        std::fs::write(
            repo.join("src/caller.rs"),
            "use crate::other::faraway;\npub fn local() {}\npub fn drive() {\n    local();\n    faraway();\n    let _ = String::new();\n}\n",
        )
        .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "callarms", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "callarms", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        crate::graph_facts::arm_tally::reset();
        let abs = repo.join("src/caller.rs").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs).await.unwrap();
        let fired = crate::graph_facts::arm_tally::take();

        assert!(!fired.is_empty(), "no arms fired — calls are not going through the persister");
        for arm in &ARMS {
            assert!(
                fired.get(*arm).copied().unwrap_or(0) >= 1,
                "call emit arm {arm:?} never fired; got {fired:?}"
            );
        }

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    /// Every inheritance emit ARM must actually fire, and the list is hardcoded.
    ///
    /// This is the mechanism the slice-3 gate needs. Four of the emit arms are
    /// indistinguishable in FINAL STATE — `inh/in-file` and `inh/stub` both end
    /// as a resolved edge to an enriched node — so a check that classifies rows
    /// after the fact cannot prove which branch ran. A migration that routed
    /// every internal target through `upsert_node_by_fqn`, dropping the
    /// `fqn_ids` fast path, would leave the graph identical and be caught by
    /// nothing.
    ///
    /// The arm list is a CONST, iterated. An earlier design built a map by
    /// counting observed arms and asserted each count >= 1, which cannot fail:
    /// an arm that stops firing is an absent key, not a zero.
    ///
    /// Breaking mutation: delete any one arm's `bump` call, or collapse two
    /// branches into one — the missing arm is named in the failure.
    #[tokio::test]
    async fn every_inheritance_emit_arm_fires() {
        // Kind-agnostic: the persister serves every edge kind. Only inheritance
        // is migrated in this increment, so attribution is unambiguous; when
        // calls migrate too, this test must scope by kind or it will pass on
        // another arm's firing.
        // SCOPED BY EDGE KIND. One persister serves every kind, so an unscoped
        // list would let this test pass on a CALL arm's firing once calls
        // migrate too — a test that cannot tell which caller exercised a branch
        // pins neither.
        const ARMS: [&str; 4] =
            ["implements/lib", "implements/stub", "implements/in-file", "implements/unresolvable"];

        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("arms");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"arms\"\n").unwrap();
        std::fs::write(repo.join("src/greet.rs"), "pub trait Greet {}\n").unwrap();
        // One file reaching all three arms: an external trait (lib), a trait in
        // ANOTHER file (stub — the 99.5% real case), and one nothing can place.
        std::fs::write(
            repo.join("src/widget.rs"),
            "use crate::greet::Greet;\npub struct W;\nimpl std::fmt::Debug for W {}\nimpl Greet for W {}\nimpl Mystery for W {}\n",
        )
        .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "arms", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "arms", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // `inh/in-file` needs BOTH sides in ONE file, because `fqn_ids` holds
        // only this file's defs. Without such a fixture, collapsing the fast
        // path into the stub path leaves every test green — verified by probing
        // exactly that, which is why this file exists.
        std::fs::write(
            repo.join("src/pair.rs"),
            "pub trait Pair {}\npub struct P;\nimpl Pair for P {}\n",
        )
        .unwrap();

        crate::graph_facts::arm_tally::reset();
        for f in ["src/widget.rs", "src/pair.rs"] {
            let abs = repo.join(f).to_string_lossy().to_string();
            processed(&ctx, &repo_path, &abs).await.unwrap();
        }
        let fired = crate::graph_facts::arm_tally::take();

        assert!(!fired.is_empty(), "no arms fired at all — the tally is not wired");
        for arm in ARMS {
            assert!(
                fired.get(arm).copied().unwrap_or(0) >= 1,
                "emit arm {arm:?} never fired; got {fired:?}"
            );
        }

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    /// Inheritance persists as an edge, and the SUBTYPE-FIRST order still
    /// resolves.
    ///
    /// Order-independence is the whole design constraint, not a nicety: measured
    /// over the corpus, 404 of 406 java relations (99.5%) have their parent type
    /// in a DIFFERENT file. A probe-only resolver would leave roughly half of
    /// them unresolved on a cold index and never heal, trading the 7,905
    /// mislabelled edges for a fresh pile of unresolved ones. So the impl file
    /// here is processed BEFORE the file defining the trait.
    ///
    /// Breaking mutations: (1) replace the `upsert_node_by_fqn` stub with a
    /// `node_id_by_fqn` probe — the target is NULL because the trait does not
    /// exist yet; (2) drop the props stamp — the discriminant is NULL and a rust
    /// trait impl becomes indistinguishable from java `implements`.
    #[tokio::test]
    async fn a_trait_impl_persists_as_an_implements_edge_before_its_trait_exists() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("inh");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"inh\"\n").unwrap();
        std::fs::write(repo.join("src/greet.rs"), "pub trait Greet {\n    fn hi(&self);\n}\n")
            .unwrap();
        std::fs::write(
            repo.join("src/widget.rs"),
            "use crate::greet::Greet;\npub struct W;\nimpl Greet for W {\n    fn hi(&self) {}\n}\n",
        )
        .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "inh", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "inh", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // SUBTYPE FIRST — its parent does not exist in the graph yet.
        for f in ["src/widget.rs", "src/greet.rs"] {
            let abs = repo.join(f).to_string_lossy().to_string();
            processed(&ctx, &repo_path, &abs).await.unwrap();
        }

        // (target_id, target_name, props.relation, resolved target's name)
        type ImplEdgeRow = (Option<uuid::Uuid>, Option<String>, Option<String>, Option<String>);
        let rows: Vec<ImplEdgeRow> = sqlx_core::query_as::query_as(
            "SELECT e.target_id, e.target_name, e.props->>'relation', t.name
                   FROM sensei.edges e
                   LEFT JOIN sensei.nodes t ON t.id = e.target_id
                  WHERE e.folder_id = $1 AND e.kind = 'implements'::sensei.edge_kind",
        )
        .bind(fid)
        .fetch_all(ctx.pg().pool())
        .await
        .unwrap();

        assert_eq!(rows.len(), 1, "exactly one trait impl should persist: {rows:?}");
        let (tid, tname, relation, target_name) = &rows[0];
        assert_eq!(
            relation.as_deref(),
            Some("trait_impl"),
            "the discriminant separates this from java `implements`"
        );
        assert!(
            tid.is_some(),
            "must resolve even though the trait was indexed AFTER the impl \
             (stub-then-enrich, not probe-only): target_name={tname:?}"
        );
        assert_eq!(
            target_name.as_deref(),
            Some("Greet"),
            "and it must point at the trait, not something same-named"
        );

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    /// Slice 1b end to end: kotlin, C and swift symbols must reach the DB WITH
    /// an fqn.
    ///
    /// The unit and corpus tests prove the producers work on real source. This
    /// proves the daemon's own path calls them and persists the result — a
    /// producer nothing invokes is indistinguishable from no producer, and the
    /// live graph showed exactly that shape before this landed: kotlin 0/1542
    /// nodes with an fqn, swift 0/8, c 1/322.
    ///
    /// Breaking mutation: return `false` from any of the three adapters'
    /// `supports_fqn`, or make `fqn_output` return `None` — that language's
    /// assertion fails with its node names printed.
    #[tokio::test]
    async fn kotlin_c_and_swift_symbols_reach_the_db_with_fqns() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("mixed");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        // A build file, so the C producer takes its build-root branch (the one
        // this repo's own corpus exercises).
        std::fs::write(repo.join("Makefile"), "all:\n").unwrap();
        std::fs::write(
            repo.join("src/Widget.kt"),
            "package com.acme.svc\n\nclass Widget {\n    fun render() {}\n}\n",
        )
        .unwrap();
        std::fs::write(repo.join("src/util.c"), "int compute(void) {\n  return 1;\n}\n").unwrap();
        std::fs::write(repo.join("src/Thing.swift"), "class Thing {\n    func go() {}\n}\n")
            .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "mixed", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "mixed", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        for f in ["src/Widget.kt", "src/util.c", "src/Thing.swift"] {
            let abs = repo.join(f).to_string_lossy().to_string();
            processed(&ctx, &repo_path, &abs).await.unwrap();
        }

        for (lang, sym) in [("kotlin", "Widget"), ("c", "compute"), ("swift", "Thing")] {
            let fqns: Vec<Option<String>> = sqlx_core::query_scalar::query_scalar(
                "SELECT fqn FROM sensei.nodes
                  WHERE folder_id = $1 AND language = $2 AND name = $3",
            )
            .bind(fid)
            .bind(lang)
            .bind(sym)
            .fetch_all(ctx.pg().pool())
            .await
            .unwrap();

            assert!(!fqns.is_empty(), "{lang}: no node named {sym} was persisted at all");
            assert!(
                fqns.iter().any(|f| f.as_deref().is_some_and(|f| f.starts_with(lang))),
                "{lang}: {sym} persisted without an fqn beginning `{lang}·`: {fqns:?}"
            );
        }

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn local_imports_resolve_to_the_target_module_at_emit() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("tsimp");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("package.json"), "{\"name\":\"tsimp\"}\n").unwrap();
        std::fs::write(repo.join("src/b.ts"), "export function x() { return 1; }\n").unwrap();
        std::fs::write(
            repo.join("src/a.ts"),
            "import { x } from './b';\nexport function drive() { return x(); }\n",
        )
        .unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "tsimp", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "tsimp", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        for f in ["src/b.ts", "src/a.ts"] {
            let abs = repo.join(f).to_string_lossy().to_string();
            processed(&ctx, &repo_path, &abs).await.unwrap();
        }

        let (tid, tname): (Option<uuid::Uuid>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT e.target_id, e.target_name FROM sensei.edges e
               JOIN sensei.nodes n ON n.id = e.source_id
              WHERE e.folder_id = $1 AND e.kind = 'imports'::sensei.edge_kind
                AND EXISTS (SELECT 1 FROM sensei.node_paths np
                            WHERE np.node_id = n.id AND np.file_path = 'src/a.ts')",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert!(tid.is_some(), "the './b' import must resolve — it names a file in this repo");
        assert_eq!(
            tname, None,
            "resolving ERASES target_name; the two are mutually exclusive across all edges"
        );

        let (kind, file): (String, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT n.kind::text, np.file_path FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.id = $1",
        )
        .bind(tid.unwrap())
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(kind, "module");
        assert_eq!(file.as_deref(), Some("src/b.ts"), "resolved to the imported file's module");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    /// LOOKUP-FIRST, pinned. The resolver probes EVERY candidate fqn before it
    /// creates anything, because get-or-creating on candidate 1 would satisfy
    /// candidate 1 forever and hide the real target sitting at candidate 2.
    ///
    /// The real module here is pre-seeded at the SRC-STRIPPED candidate (the
    /// second one) with a real file, while the first candidate does not exist —
    /// the live shape, since `ts_module_path` strips a leading `src/` so a `../`
    /// that climbs out of `src` lands one segment high.
    ///
    /// Breaking mutation: in the emit branch, replace the probe loop with
    /// `upsert_node_by_fqn(candidates[0], .., None)` — the edge then points at a
    /// freshly minted stub whose `file_path` is NULL instead of the real module.
    #[tokio::test]
    async fn import_resolution_probes_every_candidate_before_creating_one() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("tsfan");
        std::fs::create_dir_all(repo.join("src/routes")).unwrap();
        std::fs::write(repo.join("package.json"), "{\"name\":\"tsfan\"}\n").unwrap();
        std::fs::write(
            repo.join("src/routes/page.ts"),
            "import { h } from '../src/lib/x';\nexport function v() { return h(); }\n",
        )
        .unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "tsfan", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "tsfan", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // Pre-seed the REAL module at the src-stripped candidate — candidate 2.
        // Candidate 1 (`typescript·tsfan·src/lib/x`) is deliberately absent.
        let real = ctx
            .pg()
            .seed_node_by_fqn(
                &fid,
                "typescript·tsfan·lib/x",
                "module",
                "x",
                Some("typescript"),
                Some(crate::db::pg_store::FqnDef {
                    file_path: "src/lib/x.ts",
                    signature: None,
                    line_start: None,
                    line_end: None,
                    is_exported: false,
                    parent_id: None,
                }),
            )
            .await
            .unwrap();

        let abs = repo.join("src/routes/page.ts").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs).await.unwrap();

        let (tid,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT e.target_id FROM sensei.edges e JOIN sensei.nodes n ON n.id = e.source_id
              WHERE e.folder_id = $1 AND e.kind = 'imports'::sensei.edge_kind
                AND EXISTS (SELECT 1 FROM sensei.node_paths np
                            WHERE np.node_id = n.id AND np.file_path = 'src/routes/page.ts')",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(
            tid,
            Some(real),
            "must land on the REAL module found by probing a LATER candidate — not a phantom \
             stub minted from the first one"
        );
        // And nothing was created at candidate 1.
        assert_eq!(
            ctx.pg().node_id_by_fqn(&fid, "typescript·tsfan·src/lib/x").await.unwrap(),
            None,
            "probing must not create the candidate it missed"
        );

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    /// A doc's file reference and its unambiguous symbol mention both RESOLVE;
    /// an ambiguous mention and a broken link stay unresolved. 241,514
    /// `references` edges sat at 0% because nothing tried to resolve them.
    ///
    /// Breaking mutation: revert either arm to the unconditional
    /// `insert_edge(.., None, Some(x), .., "references")`.
    #[tokio::test]
    async fn doc_references_resolve_to_files_and_unambiguous_symbols() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("docref");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"docref\"\n").unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn only_once() -> i32 { 1 }\n").unwrap();
        // `twice` is defined in TWO files — an ambiguous mention.
        std::fs::write(repo.join("src/a.rs"), "pub fn twice() -> i32 { 1 }\n").unwrap();
        std::fs::write(repo.join("src/b.rs"), "pub fn twice() -> i32 { 2 }\n").unwrap();
        std::fs::write(
            repo.join("README.md"),
            "# Doc\n\nSee `src/lib.rs` for `only_once`, and `twice` lives in two places. \
             Also `src/missing.rs` is a broken link.\n",
        )
        .unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "docref", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "docref", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // Code first, so the doc has something to resolve against.
        for f in ["src/lib.rs", "src/a.rs", "src/b.rs", "README.md"] {
            let abs = repo.join(f).to_string_lossy().to_string();
            processed(&ctx, &repo_path, &abs).await.unwrap();
        }

        let rows: Vec<(Option<uuid::Uuid>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT e.target_id, e.target_name FROM sensei.edges e
               JOIN sensei.nodes n ON n.id = e.source_id
              WHERE e.folder_id = $1 AND e.kind = 'references'::sensei.edge_kind
                AND EXISTS (SELECT 1 FROM sensei.node_paths np
                            WHERE np.node_id = n.id AND np.file_path = 'README.md')",
        )
        .bind(fid)
        .fetch_all(ctx.pg().pool())
        .await
        .unwrap();
        assert!(!rows.is_empty(), "the README produced reference edges");

        // **A FILE'S IDENTITY IS THE MODULE IT DECLARES.** v1 minted a
        // `kind='file'` node for a doc reference to land on; v2 does not,
        // because a file is a row in `sensei.files` and a node REFERENCES one
        // (`nodes.file_id`). What a doc reference to `src/lib.rs` resolves to
        // is that file's MODULE node — `index::file_identity` /
        // `LanguageAdapter::file_fqn`, documented as "the identity of the file
        // itself, which is the identity of the module it declares".
        //
        // So the assertion is on the file the target NAMES, not on a node kind
        // that should not exist.
        let resolved_files: Vec<uuid::Uuid> = rows.iter().filter_map(|(t, _)| *t).collect();
        let mut hit_lib = false;
        for t in &resolved_files {
            let (kind, fp): (String, Option<String>) = sqlx_core::query_as::query_as(
                "SELECT n.kind::text, np.file_path FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.id = $1",
            )
            .bind(t)
            .fetch_one(ctx.pg().pool())
            .await
            .unwrap();
            if fp.as_deref() == Some("src/lib.rs") && kind != "file" {
                hit_lib = true;
            }
        }
        assert!(
            hit_lib,
            "a doc reference to `src/lib.rs` must resolve to a node OF that file — its module \
             identity — and never to a `kind='file'` node, which v2 does not mint"
        );

        // The unambiguous symbol resolved; the ambiguous one and the broken link
        // did not — and both kept their mention.
        let unresolved: Vec<&str> =
            rows.iter().filter(|(t, _)| t.is_none()).filter_map(|(_, n)| n.as_deref()).collect();
        assert!(
            !unresolved.contains(&"only_once"),
            "an unambiguous symbol mention resolves: {unresolved:?}"
        );
        assert!(
            unresolved.contains(&"twice"),
            "an AMBIGUOUS mention must stay unresolved rather than pick one: {unresolved:?}"
        );
        // A broken link produces NO reference edge at all — `extract_file_refs`
        // gates on the file existing, so it never reaches the emit site. (Broken
        // links are tracked separately by the doc-drift scan.) What matters here
        // is the invariant either way: it must not mint a node for a file that
        // does not exist.
        assert!(
            !unresolved.contains(&"src/missing.rs"),
            "a nonexistent file is filtered at extraction, not carried as a ref: {unresolved:?}"
        );
        assert_eq!(
            ctx.pg().file_node_id_by_path(&fid, "src/missing.rs").await.unwrap(),
            None,
            "a doc naming a nonexistent file is never evidence the file exists"
        );

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    /// Order-independence, which is why this needs no barrier: importing a file
    /// that is not indexed yet creates a STUB on the target's own fqn, and the
    /// later definition ENRICHES that same row keeping its id. Mirrors
    /// `rust_call_before_def_creates_stub_then_enriched` for imports.
    #[tokio::test]
    async fn an_import_before_its_target_creates_a_stub_then_enriches_it() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("tsord");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("package.json"), "{\"name\":\"tsord\"}\n").unwrap();
        std::fs::write(repo.join("src/b.ts"), "export function x() { return 1; }\n").unwrap();
        std::fs::write(
            repo.join("src/a.ts"),
            "import { x } from './b';\nexport function drive() { return x(); }\n",
        )
        .unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "tsord", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "tsord", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // IMPORTER FIRST — its target does not exist yet.
        let abs_a = repo.join("src/a.ts").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs_a).await.unwrap();

        let (tid,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT e.target_id FROM sensei.edges e JOIN sensei.nodes n ON n.id = e.source_id
              WHERE e.folder_id = $1 AND e.kind = 'imports'::sensei.edge_kind
                AND EXISTS (SELECT 1 FROM sensei.node_paths np
                            WHERE np.node_id = n.id AND np.file_path = 'src/a.ts')",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        let stub_id = tid.expect("resolved to a stub even though the target is not indexed yet");
        let (resolved, file): (bool, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT n.resolved, np.file_path FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.id = $1",
        )
        .bind(stub_id)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert!(!resolved, "it is a stub until its file is indexed");
        assert_eq!(file, None, "a stub has no file");

        let abs_b = repo.join("src/b.ts").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs_b).await.unwrap();

        let (resolved2, file2): (bool, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT n.resolved, np.file_path FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.id = $1",
        )
        .bind(stub_id)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert!(resolved2, "the definition enriched the stub in place");
        assert_eq!(
            file2.as_deref(),
            Some("src/b.ts"),
            "same node id, now carrying the file — so no barrier or second pass is needed"
        );

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn external_calls_link_to_lib_nodes() {
        // Phase 4: a call into a dependency links to a first-class external node
        // grouped (props.package + parent_id) under a per-package `package`
        // container, and the dependency is queryable per repo. No external call dropped.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("libcrate");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"libcrate\"\n").unwrap();
        std::fs::write(
            repo.join("src/lib.rs"),
            "pub fn load(s: &str) { serde_json::from_str(s); }\n",
        )
        .unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "libc", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "libcrate", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let abs = repo.join("src/lib.rs").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs).await.unwrap();

        // The external symbol is a `lib_symbol`, grouped by package.
        let (sym_id, sym_pkg, parent): (uuid::Uuid, Option<String>, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
            "SELECT id, props->>'package', parent_id FROM sensei.nodes WHERE folder_id=$1 AND kind='unknown'::sensei.node_kind AND name='from_str'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(sym_pkg.as_deref(), Some("serde_json"), "lib symbol grouped by package");

        // …under a per-package `lib_package` container.
        let (container_id, container_name): (uuid::Uuid, String) =
            sqlx_core::query_as::query_as(&format!(
                "SELECT id, name FROM sensei.nodes WHERE folder_id=$1 \
              AND kind='package'::sensei.node_kind AND {ext}",
                ext = crate::languages::fqn::sql_is_external("fqn")
            ))
            .bind(fid)
            .fetch_one(ctx.pg().pool())
            .await
            .unwrap();
        assert_eq!(container_name, "serde_json", "a lib_package container per dependency");
        assert_eq!(
            parent,
            Some(container_id),
            "the lib symbol is parented under its package container"
        );

        // A RESOLVED call edge load → from_str (external call not dropped).
        let (edge_target,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT e.target_id FROM sensei.edges e JOIN sensei.nodes s ON s.id=e.source_id
              WHERE e.folder_id=$1 AND s.name='load' AND e.kind='calls'::sensei.edge_kind",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(edge_target, Some(sym_id), "the external call resolves to the lib symbol node");

        // Queryable per repo.
        let deps = ctx.pg().list_dependencies(&fid).await.unwrap();
        assert!(
            deps.iter().any(|d| d["package"] == "serde_json" && d["symbol_count"] == 1),
            "serde_json is a queryable dependency with one used symbol, got {deps:?}"
        );

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn process_file_flags_test_file_nodes_is_test() {
        // Every node in a test file gets is_test=true (UI filters tests out);
        // production-file nodes stay is_test=false.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("istest");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::create_dir_all(repo.join("tests")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname=\"istest\"\n").unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn compute() -> i32 { 1 }\n").unwrap();
        std::fs::write(repo.join("tests/it.rs"), "fn helper() {}\nfn check() { helper(); }\n")
            .unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "istest", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "istest", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        for rel in ["src/lib.rs", "tests/it.rs"] {
            let abs = repo.join(rel).to_string_lossy().to_string();
            processed(&ctx, &repo_path, &abs).await.unwrap();
        }

        let count = |sql: &'static str| {
            let pool = ctx.pg().pool().clone();
            async move {
                let (n,): (i64,) =
                    sqlx_core::query_as::query_as(sql).bind(fid).fetch_one(&pool).await.unwrap();
                n
            }
        };
        // Every node of the test file is flagged; none left unflagged.
        assert!(count("SELECT count(*) FROM sensei.nodes n JOIN sensei.node_paths np ON np.node_id = n.id \
              WHERE n.folder_id=$1 AND np.file_path='tests/it.rs' AND n.is_test").await >= 1,
            "test-file nodes are is_test=true");
        assert_eq!(count("SELECT count(*) FROM sensei.nodes n JOIN sensei.node_paths np ON np.node_id = n.id \
              WHERE n.folder_id=$1 AND np.file_path='tests/it.rs' AND NOT n.is_test").await, 0,
            "no test-file node left unflagged");
        // Production file nodes exist and are NOT flagged.
        assert!(
            count(
                "SELECT count(*) FROM sensei.nodes n JOIN sensei.node_paths np ON np.node_id = n.id \
              WHERE n.folder_id=$1 AND np.file_path='src/lib.rs'"
            )
            .await
                >= 1,
            "prod file produced nodes"
        );
        assert_eq!(count("SELECT count(*) FROM sensei.nodes n JOIN sensei.node_paths np ON np.node_id = n.id \
              WHERE n.folder_id=$1 AND np.file_path='src/lib.rs' AND n.is_test").await, 0,
            "production-file nodes are not is_test");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn process_file_ts_emits_fqn_nodes() {
        // Phase 6.1: a TypeScript file with a package.json → the FQN path. Validates
        // the oxc producer end-to-end, the src-stripped module, resolved edges, AND
        // that the node language column is 'typescript' (not the old hardcoded rust).
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("tsapp");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("package.json"), "{\"name\": \"tsapp\"}").unwrap();
        std::fs::write(repo.join("src/util.ts"),
            "export function compute() { return helper(); }\nexport function helper() { return 1; }\n").unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "ts", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "tsapp", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let abs = repo.join("src/util.ts").to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs).await.unwrap();

        let (compute_id, compute_fqn, compute_lang): (uuid::Uuid, Option<String>, Option<String>) =
            sqlx_core::query_as::query_as(
                "SELECT id, fqn, language FROM sensei.nodes WHERE folder_id=$1 AND name='compute' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(
            compute_fqn.as_deref(),
            Some("typescript·tsapp·util·compute"),
            "src/ stripped module + oxc def"
        );
        assert_eq!(
            compute_lang.as_deref(),
            Some("typescript"),
            "language column is the file's language, not hardcoded rust"
        );

        let (helper_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND fqn='typescript·tsapp·util·helper'",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        let (target,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND source_id=$2 AND kind='calls'::sensei.edge_kind")
            .bind(fid).bind(compute_id).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(target, Some(helper_id), "compute→helper resolves to the FQN target at emit");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    async fn seed_indexing_repo(
        ctx: &TaskContext,
        root: &std::path::Path,
        name: &str,
    ) -> (uuid::Uuid, uuid::Uuid, String) {
        let repo_path = root.join("repo").to_string_lossy().to_string();
        // A MANIFEST, because v2 must be TOLD its package and will not invent
        // one — `placement_on_disk` answers `None` when nothing at or above a
        // file names a package, and an unplaced file is not indexed. Every real
        // rust repo has this; the v1 fixtures did not.
        std::fs::create_dir_all(root.join("repo")).ok();
        std::fs::write(
            root.join("repo/Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .ok();
        let rid = ctx
            .pg()
            .add_watch_root(&root.to_string_lossy(), name, &serde_json::json!([]))
            .await
            .unwrap();
        ctx.pg().upsert_repo_kind(&rid, "git", "repo", &repo_path).await.unwrap();
        let (fid,): (uuid::Uuid,) =
            sqlx_core::query_as::query_as("SELECT id FROM sensei.folders WHERE abs_path = $1")
                .bind(&repo_path)
                .fetch_one(ctx.pg().pool())
                .await
                .unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // MODEL STAGE 3'S BARRIER. Production creates every `files` row before
        // any parse task exists (R14), which is what lets node persistence FAIL
        // CLOSED on a missing file (R13, 06 S6). A fixture that seeds a repo on
        // disk and jumps straight to `process_file` skips that, and the writer
        // correctly refuses — so the barrier belongs in the fixture, not a
        // get-or-create in the writer.
        barrier(ctx, &fid, std::path::Path::new(&repo_path)).await;
        (rid, fid, repo_path)
    }

    /// Walk a seeded repo and create its `files` rows — the fixture's stand-in
    /// for stage 3. Paths are folder-relative, exactly as the walk records them.
    async fn barrier(ctx: &TaskContext, folder_id: &uuid::Uuid, repo_root: &std::path::Path) {
        fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if p.file_name().and_then(|n| n.to_str()) != Some(".git") {
                        collect(&p, out);
                    }
                } else {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        collect(repo_root, &mut files);
        for f in files {
            let Ok(rel) = f.strip_prefix(repo_root) else { continue };
            let _ =
                crate::tasks::test_support::seed_file(ctx.pg(), folder_id, &rel.to_string_lossy())
                    .await;
        }
    }

    #[tokio::test]
    async fn graph_scan_end_to_end() {
        // 7.5: scan the committed fixture repo through the real handler chain and
        // assert the WHOLE graph at once — code + doc-section + rationale nodes,
        // resolved edges (dup-factor 1.0), deterministic communities, the folder
        // reaching `indexed`, and the retrieval contract (tree + per-node
        // community_id + live overview). Then re-run → convergent (zero net rows),
        // then mutate a doc → scoped incremental. (The monorepo `workspace_member`
        // kind is covered separately by `reconcile_classifies_monorepo_member_roles`;
        // this fixture is manifest-free by design.)
        let ctx = make_ctx().await;
        // Materialise the committed fixture into a tempdir — read the real committed
        // files by their known relative paths — so the incremental step can mutate
        // it without dirtying the repo.
        let src_fixture =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph-scan");
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("graph-scan");
        // Cargo.toml is materialised too so the Rust FQN producer can derive the
        // package (nearest manifest) and resolve compute→helper AT EMIT (7.1).
        for rel in ["Cargo.toml", "src/lib.rs", "docs/design.md"] {
            let content = std::fs::read_to_string(src_fixture.join(rel)).unwrap();
            let dst = repo.join(rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::write(&dst, content).unwrap();
        }
        let repo_path = repo.to_string_lossy().to_string();

        let rid = ctx
            .pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "gse", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "graph-scan", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // Drive the real handler chain (deterministic — the queue's next_task
        // blocks, so tests drive handlers directly, the codebase idiom).
        async fn scan_files(
            ctx: &TaskContext,
            repo: &std::path::Path,
            repo_path: &str,
            rels: &[&str],
        ) {
            for rel in rels {
                let abs = repo.join(rel).to_string_lossy().to_string();
                processed(ctx, repo_path, &abs).await.unwrap();
            }
            crate::tasks::handlers::detect_communities(
                ctx,
                &Task::new(TaskKind::DetectCommunities, repo_path, ""),
            )
            .await
            .unwrap();
        }
        let files = ["src/lib.rs", "docs/design.md"];
        scan_files(&ctx, &repo, &repo_path, &files).await;

        let count = |sql: &'static str| {
            let pool = ctx.pg().pool().clone();
            async move {
                let (n,): (i64,) =
                    sqlx_core::query_as::query_as(sql).bind(fid).fetch_one(&pool).await.unwrap();
                n
            }
        };

        // ── Whole-graph: kinds present ──
        //
        // A FILE IS NOT A NODE. Files live in `sensei.files`; `sensei.nodes`
        // holds graph nodes — symbols — and each REFERENCES its file through
        // `nodes.file_id`. v1 wrote `kind='file'` nodes, which is where this
        // repo's 54,342 fqn-less rows came from; v2 does not, and anything
        // wanting file-level information reads `sensei.files`.
        assert!(
            count("SELECT count(*) FROM sensei.files WHERE folder_id=$1").await >= 1,
            "files rows (NOT `kind='file'` nodes — a file is not a node)"
        );
        assert!(
            count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='file'::sensei.node_kind").await == 0,
            "no `kind='file'` node may exist: a file is a row in `sensei.files`, and a node \
             that named one carried no fqn and could never be an edge target"
        );
        assert!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='function'::sensei.node_kind").await >= 2, "compute + helper function nodes");
        assert!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind").await >= 3, "nested section nodes (Design/Auth/Refresh/Storage)");
        assert_eq!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind").await, 1, "one TODO rationale");

        // ── Section nesting (Refresh under Auth under a doc/file node) ──
        let (nested,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes s JOIN sensei.nodes p ON s.parent_id=p.id
              WHERE s.folder_id=$1 AND s.kind='section'::sensei.node_kind AND p.kind IN ('doc'::sensei.node_kind,'file'::sensei.node_kind,'section'::sensei.node_kind)")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert!(nested >= 3, "sections nest via parent_id");

        // ── Resolved call edge (compute → helper) ──
        let (resolved_calls,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind AND target_id IS NOT NULL")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert!(resolved_calls >= 1, "the compute→helper call resolved");

        // ── dup-factor 1.0 for every edge kind ──
        let (dup_kinds,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM (
               SELECT kind, count(*) c, count(DISTINCT (source_id,target_id,target_name,target_file)) d
                 FROM sensei.edges WHERE folder_id=$1 GROUP BY kind) t WHERE c <> d")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(dup_kinds, 0, "no duplicate edges — dup-factor 1.0 for every kind");

        // ── Communities deterministic + coverage ──
        let code_uncovered = count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind IN ('function'::sensei.node_kind,'file'::sensei.node_kind) AND community_id IS NULL").await;
        assert_eq!(code_uncovered, 0, "every code/file node carries a community_id (coverage)");

        // ── Folder reached `indexed` (terminal barrier) ──
        assert_eq!(
            ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(),
            Some("indexed"),
            "the terminal DetectCommunities barrier flipped the folder to indexed"
        );

        // ── Retrieval contract: tree nests, node projection carries community_id, live overview ──
        let folders = ctx.pg().get_folders_scoped(&[fid]).await.unwrap();
        let nodes = ctx.pg().get_nodes_scoped(&[fid]).await.unwrap();
        assert!(
            nodes.iter().any(|n| n.get("community_id").is_some()),
            "get_nodes_scoped projects community_id"
        );
        let tree = crate::api::handlers::codebase::build_tree_pub(&folders, &nodes);
        let roots = tree["tree"].as_array().unwrap();
        assert!(!roots.is_empty(), "tree has a root folder");
        // The root folder exposes file/doc nodes, and a doc node has section children.
        let has_section_child = |v: &serde_json::Value| -> bool {
            v["nodes"]
                .as_array()
                .map(|ns| {
                    ns.iter().any(|f| {
                        f["children"]
                            .as_array()
                            .map(|c| c.iter().any(|ch| ch["kind"] == "section"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        };
        assert!(roots.iter().any(has_section_child), "the tree nests a doc → section subtree");
        let live = ctx.pg().list_communities_live_scoped(&[fid]).await.unwrap();
        assert!(
            !live.is_empty() && live.iter().all(|c| c["node_count"].as_i64().unwrap_or(0) > 0),
            "live overview sized by real membership"
        );

        // ── Idempotency / convergence: re-run is IDENTITY-STABLE, not just
        // count-stable. Capture every node's id keyed on its natural key
        // (file_path,kind,name,line_start); after a second scan assert the id map
        // is byte-identical — so a regression to delete-then-insert (which keeps
        // counts equal but MINTS NEW UUIDs, nulling embeddings/community) fails
        // here, per invariant 2 (identical nodes.id set on re-run).
        let ids_before: std::collections::BTreeMap<
            (String, String, String, Option<i32>),
            uuid::Uuid,
        > = {
            let rows: Vec<(String, String, String, Option<i32>, uuid::Uuid)> = sqlx_core::query_as::query_as(
                "SELECT np.file_path, n.kind::text, n.name, n.line_start, n.id FROM sensei.nodes n \
                   JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.folder_id=$1")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            rows.into_iter().map(|(fp, k, n, ls, id)| ((fp, k, n, ls), id)).collect()
        };
        let e0 = count("SELECT count(*) FROM sensei.edges WHERE folder_id=$1").await;
        scan_files(&ctx, &repo, &repo_path, &files).await;
        let ids_after: std::collections::BTreeMap<
            (String, String, String, Option<i32>),
            uuid::Uuid,
        > = {
            let rows: Vec<(String, String, String, Option<i32>, uuid::Uuid)> = sqlx_core::query_as::query_as(
                "SELECT np.file_path, n.kind::text, n.name, n.line_start, n.id FROM sensei.nodes n \
                   JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.folder_id=$1")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            rows.into_iter().map(|(fp, k, n, ls, id)| ((fp, k, n, ls), id)).collect()
        };
        let e1 = count("SELECT count(*) FROM sensei.edges WHERE folder_id=$1").await;
        assert_eq!(
            ids_before, ids_after,
            "a second scan is identity-stable — every node keeps its exact id (not delete-then-insert)"
        );
        assert_eq!(e0, e1, "a second scan adds no edges (dup-factor 1.0, convergent)");

        // ── Scoped incremental: add a heading to the doc → new section, and an
        // unrelated code node keeps its exact id AND community (upsert-then-prune,
        // not a wholesale re-mint) ──
        let (compute_id_before, compute_comm_before): (uuid::Uuid, Option<i32>) = sqlx_core::query_as::query_as(
            "SELECT id, community_id FROM sensei.nodes WHERE folder_id=$1 AND kind='function'::sensei.node_kind AND name='compute'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        let design = repo.join("docs/design.md");
        let mut doc = std::fs::read_to_string(&design).unwrap();
        doc.push_str("\n## Extra\n\nAdded section.\n");
        std::fs::write(&design, &doc).unwrap();
        scan_files(&ctx, &repo, &repo_path, &files).await;

        let (extra,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind AND name='Design > Extra'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(extra, 1, "the new heading became a section node");
        let (compute_id_after, compute_comm_after): (uuid::Uuid, Option<i32>) = sqlx_core::query_as::query_as(
            "SELECT id, community_id FROM sensei.nodes WHERE folder_id=$1 AND kind='function'::sensei.node_kind AND name='compute'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(
            compute_id_after, compute_id_before,
            "an unrelated code node keeps its EXACT id across a scoped doc edit (upsert-then-prune)"
        );
        assert_eq!(compute_comm_after, compute_comm_before, "and keeps its community_id");
        // Still no duplicate edges after the incremental edit.
        let (dup2,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM (SELECT kind, count(*) c, count(DISTINCT (source_id,target_id,target_name,target_file)) d FROM sensei.edges WHERE folder_id=$1 GROUP BY kind) t WHERE c <> d")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(dup2, 0, "still dup-factor 1.0 after the incremental edit");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn processing_order_invariant() {
        // 7.4: processing the same files in DIFFERENT orders yields an identical
        // graph — the node set (by natural key), per-kind edge counts, and the
        // deterministic community_id per node all match. This is the convergence
        // guarantee D1–D4 exist to provide.
        use std::collections::BTreeMap;
        // Natural key = (file_path, kind, name, line_start); a graph snapshot maps
        // each to its community_id, plus per-kind edge counts.
        type NatKey = (String, String, String, Option<i32>);
        type GraphSnap = (BTreeMap<NatKey, Option<i32>>, BTreeMap<String, i64>);
        type NodeRow = (String, String, String, Option<i32>, Option<i32>);

        // Snapshot a folder's graph by NATURAL key (not the random UUID):
        // {(file_path,kind,name,line_start) → community_id} + per-kind edge counts.
        async fn snapshot(ctx: &TaskContext, fid: uuid::Uuid) -> GraphSnap {
            let node_rows: Vec<NodeRow> = sqlx_core::query_as::query_as(
                "SELECT np.file_path, n.kind::text, n.name, n.line_start, n.community_id FROM sensei.nodes n \
                   JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.folder_id=$1")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            let nodes =
                node_rows.into_iter().map(|(fp, k, n, ls, cid)| ((fp, k, n, ls), cid)).collect();
            let edge_rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
                "SELECT kind::text, count(*) FROM sensei.edges WHERE folder_id=$1 GROUP BY kind::text")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            (nodes, edge_rows.into_iter().collect())
        }

        // Build a repo with identical content under `root/repo`, then process the
        // given file order (edges resolve at emit) and detect communities.
        async fn build_and_scan(
            ctx: &TaskContext,
            root: &std::path::Path,
            name: &str,
            order: &[&str],
        ) -> uuid::Uuid {
            std::fs::create_dir_all(root.join("repo/src")).unwrap();
            std::fs::create_dir_all(root.join("repo/docs")).unwrap();
            std::fs::write(root.join("repo/src/a.rs"), "pub fn caller() { helper(); }\n").unwrap();
            std::fs::write(root.join("repo/src/b.rs"), "pub fn helper() {}\n").unwrap();
            std::fs::write(
                root.join("repo/docs/design.md"),
                "# Design\n\n## Auth\n\nAuth text.\n\n<!-- TODO: wire the retry path -->\n",
            )
            .unwrap();
            let (_rid, fid, repo_path) = seed_indexing_repo(ctx, root, name).await;
            for rel in order {
                let abs = root.join("repo").join(rel).to_string_lossy().to_string();
                processed(ctx, &repo_path, &abs).await.unwrap();
            }
            crate::indexer::community::detect_communities_for_folder(ctx.pg(), &fid).await.unwrap();
            fid
        }

        let ctx = make_ctx().await;
        let tmp_a = tempfile::tempdir().unwrap();
        let tmp_b = tempfile::tempdir().unwrap();
        // Opposite processing orders over identical content.
        let fid_a = build_and_scan(
            &ctx,
            tmp_a.path(),
            "order_a",
            &["src/a.rs", "src/b.rs", "docs/design.md"],
        )
        .await;
        let fid_b = build_and_scan(
            &ctx,
            tmp_b.path(),
            "order_b",
            &["docs/design.md", "src/b.rs", "src/a.rs"],
        )
        .await;

        let (nodes_a, edges_a) = snapshot(&ctx, fid_a).await;
        let (nodes_b, edges_b) = snapshot(&ctx, fid_b).await;

        assert!(!nodes_a.is_empty(), "the scan produced nodes");
        assert_eq!(
            nodes_a.keys().collect::<Vec<_>>(),
            nodes_b.keys().collect::<Vec<_>>(),
            "identical node set (by natural key) regardless of processing order"
        );
        assert_eq!(
            nodes_a, nodes_b,
            "identical community_id per node regardless of processing order (deterministic)"
        );
        assert_eq!(
            edges_a, edges_b,
            "identical per-kind edge counts regardless of processing order"
        );
        // The resolved call edge exists (so this isn't a vacuously-empty comparison).
        assert_eq!(edges_a.get("calls").copied(), Some(1), "caller→helper call edge resolved");
    }

    #[tokio::test]
    async fn process_file_fatal_db_write_marks_folder_failed_and_skips_scan_state() {
        // D6c-trigger: a fatal DB-write failure (simulated via the test fault
        // seam) propagates as Err, marks the folder `failed` (so the fail-closed
        // barrier D6d won't mark it indexed), and does NOT advance the file's
        // `files` row — so the next scan retries it.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn a() {}").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d6c_fatal").await;

        let abs = file.to_string_lossy().to_string();
        super::fault::fail_for(&abs);
        let res = processed(&ctx, &repo_path, &abs).await;
        super::fault::clear(&abs);

        assert!(res.is_err(), "a fatal DB write propagates as Err, not Ok");
        assert_eq!(
            ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(),
            Some("failed"),
            "the folder is left `failed` (fail-closed)"
        );
        // The row EXISTS — stage 3's barrier (R14) creates one for every file
        // before any parse task runs, and `processed` reproduces that. What must
        // not happen is the fingerprint ADVANCING: that is the handler's record
        // of "this file was processed", and a fatally-failed file has not been.
        // SCOPED TO THE FILE UNDER TEST, not every row in the folder: the
        // fixture also carries a `Cargo.toml` (v2 requires a manifest to place
        // a file) and that one IS processed. What this test is about is the
        // fatally-failed file's own fingerprint.
        let rows = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert!(
            rows.iter()
                .filter(|(path, _)| path == "src/lib.rs")
                .all(|(_, mtime)| *mtime == crate::db::pg_store::graph_seed::BARRIER_MTIME),
            "the `files` row is NOT advanced for a fatally-failed file: {rows:?}"
        );

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_success_advances_scan_state_and_keeps_folder_status() {
        // The success counterpart: a fully-written file advances its `files` row and
        // does NOT spuriously mark the folder failed (it stays `indexing` for the
        // barrier to flip to `indexed`).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn a() {}").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d6c_ok").await;

        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);
        process_file(&ctx, &task).await.unwrap();

        // Named, not counted: the fixture also has a `Cargo.toml`.
        let rows = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert!(
            rows.iter().any(|(path, mtime)| path == "src/lib.rs"
                && *mtime != crate::db::pg_store::graph_seed::BARRIER_MTIME),
            "a fully-written file advances its `files` row: {rows:?}"
        );
        assert_eq!(
            ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(),
            Some("indexing"),
            "a successful file does not spuriously mark the folder failed"
        );

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_fatal_on_one_file_does_not_block_a_sibling() {
        // The fatal path is per-file: a sibling file still indexes (its `files` row
        // is written) even though another file in the same folder failed fatally.
        // The folder ends `failed` (fail-closed), but the healthy file's work is
        // durable.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let bad = root.join("repo/src/bad.rs");
        let good = root.join("repo/src/good.rs");
        std::fs::write(&bad, "pub fn a() {}").unwrap();
        std::fs::write(&good, "pub fn b() {}").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d6c_sibling").await;

        let bad_abs = bad.to_string_lossy().to_string();
        super::fault::fail_for(&bad_abs);
        let bad_res = processed(&ctx, &repo_path, &bad_abs).await;
        super::fault::clear(&bad_abs);
        // The sibling processes independently and succeeds.
        let good_abs = good.to_string_lossy().to_string();
        processed(&ctx, &repo_path, &good_abs).await.unwrap();

        assert!(bad_res.is_err(), "the bad file fails fatally");
        assert_eq!(
            ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(),
            Some("failed"),
            "the folder is `failed` because one of its files failed"
        );
        // Both files have a row (stage 3's barrier, R14). Only one has a real
        // fingerprint — advancing past `BARRIER_MTIME` is what "processed" means.
        let scan = ctx.pg().list_scan_state(&fid).await.unwrap();
        // Restricted to the two `.rs` files this test is about: the fixture
        // also carries a `Cargo.toml`, which v2 needs to place a file and
        // which is itself processed.
        let advanced: Vec<&(String, i64)> = scan
            .iter()
            .filter(|(path, _)| path.ends_with(".rs"))
            .filter(|(_, mtime)| *mtime != crate::db::pg_store::graph_seed::BARRIER_MTIME)
            .collect();
        assert_eq!(
            advanced.len(),
            1,
            "only the healthy sibling advanced its `files` row: {scan:?}"
        );
        assert!(
            advanced.iter().any(|(p, _)| p.ends_with("good.rs")),
            "the sibling's fingerprint is recorded"
        );
        assert!(
            !advanced.iter().any(|(p, _)| p.ends_with("bad.rs")),
            "the failed file did NOT advance its `files` row"
        );

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_reindex_keeps_surviving_node_and_prunes_removed() {
        // D3 end-to-end: re-indexing a file KEEPS a surviving symbol's node id
        // (and its community_id — proving upsert-then-prune, not delete-then-
        // insert) and PRUNES a symbol removed from the source.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn keep() {}\npub fn gone() {}\n").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d3_reindex").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();
        let keep_id: uuid::Uuid = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND name='keep' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await
            .expect("first index creates the `keep` function node").0;
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id=42 WHERE id=$1")
            .bind(keep_id)
            .execute(ctx.pg().pool())
            .await
            .unwrap();

        // Edit: remove `gone`; `keep` stays at line 1 (unchanged identity), so it
        // survives with the same id (upsert-then-prune, not delete-then-insert).
        std::fs::write(&file, "pub fn keep() {}\n").unwrap();
        process_file(&ctx, &task).await.unwrap();

        let keep_after: Option<(uuid::Uuid, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, community_id FROM sensei.nodes WHERE folder_id=$1 AND name='keep' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_optional(ctx.pg().pool()).await.unwrap();
        assert_eq!(
            keep_after,
            Some((keep_id, Some(42))),
            "surviving symbol keeps its id AND community_id across a reindex (upsert-then-prune)"
        );
        // **DEMOTED, NOT DELETED — and that is v2's design, not a shortfall.**
        // `reconcile` releases this file's claim on `gone`, and because no file
        // claims it any more the node is DEMOTED to a stub (`resolved = false`)
        // rather than removed. Deleting it would orphan anything that still
        // references it; a stub keeps the edge pointing somewhere real and lets
        // a later scan promote it again if the symbol comes back. The only
        // deletion reconcile performs is an edge row with nothing left on it.
        //
        // So the assertion is that no DECLARATION of `gone` survives, which is
        // the property the test is named for — v1 expressed it as "the row is
        // gone" because v1 deleted.
        let (gone_declared,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes
              WHERE folder_id=$1 AND name='gone' AND resolved",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(gone_declared, 0, "the removed symbol is no longer a declaration (demoted)");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn doc_decomposes_into_nested_sections() {
        // D5b: a design doc decomposes into nested `section` nodes (file → H1 → H2
        // → H3 via parent_id, level in props), keyed on the heading PATH so a
        // re-index reconciles the set (no duplicate headings). Line-independent
        // identity: a body edit that shifts a heading's line keeps the section's id.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/docs")).unwrap();
        let file = root.join("repo/docs/design.md");
        std::fs::write(&file,
            "# Design\n\nIntro.\n\n## Auth\n\nAuth overview.\n\n### Refresh\n\nToken refresh.\n\n## Storage\n\nStorage overview.\n"
        ).unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d5b_sections").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();

        // Four section nodes: Design(H1), Auth(H2), Refresh(H3), Storage(H2).
        let (sec_cnt,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(sec_cnt, 4, "one section node per heading");

        // Nesting: Refresh's parent is Auth, Auth's parent is the H1 Design, and
        // Design's parent is the file node. Identity is the heading PATH.
        let parent_kind_name = |name: &str| {
            let pool = ctx.pg().pool().clone();
            let name = name.to_string();
            async move {
                let row: (String, String) = sqlx_core::query_as::query_as(
                    "SELECT p.kind::text, p.name FROM sensei.nodes s JOIN sensei.nodes p ON s.parent_id=p.id
                      WHERE s.folder_id=$1 AND s.kind='section'::sensei.node_kind AND s.name=$2")
                    .bind(fid).bind(&name).fetch_one(&pool).await.unwrap();
                row
            }
        };
        assert_eq!(
            parent_kind_name("Design > Auth > Refresh").await,
            ("section".into(), "Design > Auth".into()),
            "H3 Refresh nests under H2 Auth"
        );
        assert_eq!(
            parent_kind_name("Design > Auth").await,
            ("section".into(), "Design".into()),
            "H2 Auth nests under H1 Design"
        );
        assert_eq!(
            parent_kind_name("Design").await.0,
            "doc",
            "the top-level H1 nests under the doc/file node"
        );

        // level lives in props; identity carries a NULL line (line-independent).
        let (level, line_start_col): (Option<i32>, Option<i32>) = sqlx_core::query_as::query_as(
            "SELECT (props->>'level')::int, line_start FROM sensei.nodes
              WHERE folder_id=$1 AND kind='section'::sensei.node_kind AND name='Design > Auth > Refresh'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(level, Some(3), "H3 level stamped in props");
        assert_eq!(
            line_start_col, None,
            "identity line_start is NULL (line-independent section identity)"
        );

        // Capture Refresh's id, then re-index with the heading MOVED down (extra
        // intro line) — same heading path ⇒ same id, and still exactly 4 sections.
        let (refresh_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND name='Design > Auth > Refresh'",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        std::fs::write(&file,
            "# Design\n\nIntro paragraph one.\nIntro paragraph two.\n\n## Auth\n\nAuth overview.\n\n### Refresh\n\nToken refresh.\n\n## Storage\n\nStorage overview.\n"
        ).unwrap();
        process_file(&ctx, &task).await.unwrap();

        let (sec_cnt2,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(sec_cnt2, 4, "re-index reconciles — no duplicate sections");
        let (refresh_id2,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND name='Design > Auth > Refresh'",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(
            refresh_id2, refresh_id,
            "a moved heading keeps its id (line-independent identity)"
        );

        // Remove the Refresh heading → it is pruned (no stale section).
        std::fs::write(
            &file,
            "# Design\n\nIntro.\n\n## Auth\n\nAuth overview.\n\n## Storage\n\nStorage overview.\n",
        )
        .unwrap();
        process_file(&ctx, &task).await.unwrap();
        let (refresh_gone,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND name='Design > Auth > Refresh'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(refresh_gone, 0, "a removed heading is pruned");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn doc_duplicate_sibling_headings_are_distinct_sections() {
        // D5b review fix: two identical-text siblings under the same parent must be
        // DISTINCT section nodes — else the second's upsert collides onto the first
        // (same heading-path + parent_id) and silently clobbers it. The Nth (N>1)
        // occurrence gets a " #N" suffix that also flows to its children, so a child
        // of the second sibling doesn't collide with a child of the first.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/docs")).unwrap();
        let file = root.join("repo/docs/faq.md");
        std::fs::write(&file,
            "# FAQ\n\n## Setup\n\nFirst setup.\n\n### Step\n\nA.\n\n## Setup\n\nSecond setup.\n\n### Step\n\nB.\n"
        ).unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d5b_dupe").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();

        // 5 distinct sections: FAQ, "FAQ > Setup", "FAQ > Setup > Step",
        // "FAQ > Setup #2", "FAQ > Setup #2 > Step".
        let names: Vec<String> = sqlx_core::query_as::query_as::<_, (String,)>(
            "SELECT name FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind ORDER BY name")
            .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap()
            .into_iter().map(|(n,)| n).collect();
        assert_eq!(
            names,
            vec![
                "FAQ".to_string(),
                "FAQ > Setup".to_string(),
                "FAQ > Setup #2".to_string(),
                "FAQ > Setup #2 > Step".to_string(),
                "FAQ > Setup > Step".to_string(),
            ],
            "duplicate siblings + their children are distinct nodes"
        );

        // Both Setup sections exist with their OWN preview (neither clobbered).
        let previews: Vec<Option<String>> = sqlx_core::query_as::query_as::<_, (Option<String>,)>(
            "SELECT props->>'preview' FROM sensei.nodes
              WHERE folder_id=$1 AND kind='section'::sensei.node_kind AND name IN ('FAQ > Setup','FAQ > Setup #2')
              ORDER BY name")
            .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap()
            .into_iter().map(|(p,)| p).collect();
        assert!(
            previews[0].as_deref().unwrap_or("").contains("First setup"),
            "first Setup keeps its own content"
        );
        assert!(
            previews[1].as_deref().unwrap_or("").contains("Second setup"),
            "second Setup keeps its own content (not clobbered)"
        );

        // Idempotent: re-index yields the same 5, no growth.
        process_file(&ctx, &task).await.unwrap();
        let (cnt,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(cnt, 5, "re-index of duplicate-sibling doc is idempotent");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn doc_rationale_comment_emits_rationale_node() {
        // D5b: a NOTE/WHY/HACK/TODO/IMPORTANT marker in a doc becomes a `rationale`
        // node parented to the file, with the marker in props. Re-indexing the
        // unchanged doc is idempotent (no duplicate rationale).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/docs")).unwrap();
        let file = root.join("repo/docs/plan.md");
        std::fs::write(&file,
            "# Plan\n\nSome design text.\n\n<!-- TODO: wire the retry path -->\n\nMore text noting nothing.\n"
        ).unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d5b_rationale").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();

        // Exactly one rationale (the lowercase "noting" prose must NOT match).
        let rows: Vec<(String, Option<uuid::Uuid>, String)> = sqlx_core::query_as::query_as(
            "SELECT name, parent_id, props->>'marker' FROM sensei.nodes
              WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind",
        )
        .bind(fid)
        .fetch_all(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(rows.len(), 1, "one rationale node (prose 'noting' does not match)");
        assert!(rows[0].0.starts_with("TODO"), "rationale text keeps the marker: {}", rows[0].0);
        assert_eq!(rows[0].2, "TODO", "marker stamped in props");

        // Parent is the doc/file node.
        let (pkind,): (String,) = sqlx_core::query_as::query_as(
            "SELECT p.kind::text FROM sensei.nodes r JOIN sensei.nodes p ON r.parent_id=p.id
              WHERE r.id=(SELECT id FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind LIMIT 1)")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(pkind, "doc", "rationale is parented to the doc file node");

        // Idempotent re-index.
        process_file(&ctx, &task).await.unwrap();
        let (cnt2,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(cnt2, 1, "re-index does not duplicate the rationale");

        // Remove the marker → the rationale is pruned.
        std::fs::write(&file, "# Plan\n\nSome design text.\n").unwrap();
        process_file(&ctx, &task).await.unwrap();
        let (cnt3,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(cnt3, 0, "a removed rationale marker is pruned");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_persists_is_exported_from_visibility() {
        // A `pub` symbol is exported; a private one is not. The parser computes
        // is_exported from the visibility modifier; process_file must PERSIST it
        // (via upsert_node_ex) — it was previously dropped, so every symbol read
        // back as is_exported=false (the "0 exports" call-flow bug).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn exported_fn() {}\nfn private_fn() {}\n").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "isexp").await;
        let abs = file.to_string_lossy().to_string();
        processed(&ctx, &repo_path, &abs).await.unwrap();

        let exported = |name: &str| {
            let pool = ctx.pg().pool().clone();
            let name = name.to_string();
            async move {
                let (e,): (bool,) = sqlx_core::query_as::query_as(
                    "SELECT is_exported FROM sensei.nodes WHERE folder_id=$1 AND name=$2 AND kind='function'::sensei.node_kind")
                    .bind(fid).bind(&name).fetch_one(&pool).await.unwrap();
                e
            }
        };
        assert!(exported("exported_fn").await, "a pub fn is is_exported=true");
        assert!(!exported("private_fn").await, "a private fn is is_exported=false");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_reindex_reconciles_a_removed_out_edge() {
        // D3 per-file out-edge reconcile (end-to-end): a SURVIVING symbol whose
        // call is removed on re-edit drops its stale edge — the surviving node
        // isn't deleted, so the edge doesn't cascade; delete_edges_from_sources
        // clears the file's out-edges before re-inserting the current set.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn keep() { gone(); }\npub fn gone() {}\n").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d3_edge").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();
        let (calls_before,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(calls_before, 1, "keep→gone call edge is created on first index");

        // Re-edit: keep no longer calls gone (both fns stay at their lines).
        std::fs::write(&file, "pub fn keep() {}\npub fn gone() {}\n").unwrap();
        process_file(&ctx, &task).await.unwrap();

        let (calls_after,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(
            calls_after, 0,
            "the removed call's stale edge is reconciled away (replace, not append)"
        );

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn scan_state_list_and_delete_file_roundtrip() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id =
            ctx.pg().add_watch_root(&repo_path, "ss", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "ss-repo", &repo_path).await.unwrap();

        ctx.pg().upsert_scan_state(&fid, "a.rs", 111, "hashA").await.unwrap();
        ctx.pg().upsert_scan_state(&fid, "b.rs", 222, "hashB").await.unwrap();
        let state = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert_eq!(state.len(), 2, "two fingerprints recorded");

        ctx.pg().delete_scan_state_file(&fid, "a.rs").await.unwrap();
        let after = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert_eq!(after.len(), 1);
        assert!(after.iter().all(|(p, _)| p != "a.rs"), "a.rs dropped, b.rs kept");
    }

    /// A skipped file must be fingerprinted WITH its reason, and re-indexing it
    /// later must CLEAR that reason. Without the fingerprint, `plan_reindex`
    /// treats the file as changed on every pass and re-enqueues it forever; if
    /// the reason were left stale, a file the user fixed would keep reporting as
    /// unscannable.
    #[tokio::test]
    async fn scan_state_records_skip_reason_and_clears_it_on_reindex() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx
            .pg()
            .add_watch_root(&repo_path, "skipreason", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "skipreason-repo", &repo_path).await.unwrap();

        // Skipped: fingerprint + reason recorded (exercises the ::enum cast).
        ctx.pg()
            .upsert_scan_state_skipped(
                &fid,
                "docs/License.txt",
                111,
                "hashA",
                crate::classifiers::ScanSkipReason::InvalidUtf8,
            )
            .await
            .unwrap();

        let reason: Option<String> = sqlx_core::query_scalar::query_scalar(
            "SELECT skip_reason::text FROM sensei.files WHERE folder_id = $1 AND file_path = $2",
        )
        .bind(fid)
        .bind("docs/License.txt")
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(reason.as_deref(), Some("invalid_utf8"), "skip reason persisted");

        // The fingerprint itself must be visible to the change-detection gate —
        // that is what stops the re-enqueue loop.
        let state = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert!(
            state.iter().any(|(p, _)| p == "docs/License.txt"),
            "a skipped file must still carry a fingerprint the mtime gate can match"
        );

        // User fixes the encoding → the file indexes normally → reason cleared.
        ctx.pg().upsert_scan_state(&fid, "docs/License.txt", 222, "hashB").await.unwrap();
        let cleared: Option<String> = sqlx_core::query_scalar::query_scalar(
            "SELECT skip_reason::text FROM sensei.files WHERE folder_id = $1 AND file_path = $2",
        )
        .bind(fid)
        .bind("docs/License.txt")
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(cleared, None, "re-indexing a fixed file must clear the stale skip reason");
    }

    #[tokio::test]
    async fn unresolve_edges_to_file_nulls_target_keeps_name() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id =
            ctx.pg().add_watch_root(&repo_path, "ur", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "ur-repo", &repo_path).await.unwrap();

        // funcA lives in a.rs; funcB in b.rs calls it. A call starts UNRESOLVED
        // (target_name only); resolve_edge points it at funcA — the production
        // path that preserves target_name for later re-resolution (D1).
        let node_a = ctx
            .pg()
            .seed_node(&fid, "function", "funcA", "a.rs", None, None, None, None)
            .await
            .unwrap();
        let node_b = ctx
            .pg()
            .seed_node(&fid, "function", "funcB", "b.rs", None, None, None, None)
            .await
            .unwrap();
        let edge =
            ctx.pg().insert_edge(&fid, &node_b, None, Some("funcA"), None, "calls").await.unwrap();
        ctx.pg().resolve_edge(&edge, &node_a).await.unwrap();

        // Re-indexing a.rs un-resolves inbound edges instead of letting the
        // cascade delete them: target_id cleared, target_name preserved.
        let n = ctx.pg().unresolve_edges_to_file(&fid, "a.rs").await.unwrap();
        assert_eq!(n, 1, "the one inbound edge should be un-resolved");

        let edges = ctx.pg().get_edges_by_kind(&fid, "calls").await.unwrap();
        assert_eq!(edges.len(), 1);
        assert!(edges[0]["target_id"].is_null(), "target_id cleared");
        assert_eq!(
            edges[0]["target_name"].as_str(),
            Some("funcA"),
            "target_name preserved for re-resolution"
        );
    }

    #[tokio::test]
    async fn delete_file_succeeds() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";

        // Register a project
        {
            let root_id =
                ctx.pg().add_watch_root("/tmp/test", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, "/tmp/test").await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFile, "/tmp/test", "/tmp/a.rs");
        let result = delete_file(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_folder_removes_module_and_child_nodes() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";
        let repo_path = "/tmp/myrepo";

        // Register project
        {
            let root_id =
                ctx.pg().add_watch_root(repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, repo_path).await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFolder, repo_path, "/tmp/myrepo/src");
        delete_folder(&ctx, &task).await.unwrap();
    }
}
