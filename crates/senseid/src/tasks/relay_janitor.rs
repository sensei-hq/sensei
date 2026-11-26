//! Relay janitor (#444 §10.2) — deterministic workspace hygiene.
//!
//! The four-stage cascade: prune dangling worktrees, scavenge stale scratch,
//! run manifest clean targets, compact old action logs. Runs on task
//! done/cancelled (via the sweep), hourly, and at the health sentinel's
//! warning watermark. Each stage reports what it reclaimed; a stage that
//! fails logs and the cascade continues — hygiene never breaks work.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// What one cascade pass reclaimed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct JanitorReport {
    pub pruned_worktrees: u64,
    pub scavenged_bytes: u64,
    pub manifest_cleans: u64,
    pub compacted_actions: u64,
}

impl std::ops::AddAssign for JanitorReport {
    fn add_assign(&mut self, other: Self) {
        self.pruned_worktrees += other.pruned_worktrees;
        self.scavenged_bytes += other.scavenged_bytes;
        self.manifest_cleans += other.manifest_cleans;
        self.compacted_actions += other.compacted_actions;
    }
}

/// One manifest clean target: what to run, where.
#[derive(Debug, PartialEq, Eq)]
pub struct CleanCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// The clean targets a repo root defines (#444 §10.2.3), in run order:
/// `make clean-temp` (else `clean-test`), then the node cache prune.
/// Cargo is deliberately absent: `cargo clean -p` needs a crate allowlist
/// nobody maintains, and blanket cleans defeat the shared cache (§10.3);
/// stale `target/debug/incremental` is scavenged by stage 2 instead.
pub fn clean_plan(root: &Path) -> Vec<CleanCommand> {
    let mut out = Vec::new();
    let makefile = root.join("Makefile");
    if let Ok(text) = std::fs::read_to_string(&makefile) {
        let has = |t: &str| text.lines().any(|l| l.starts_with(&format!("{t}:")));
        if has("clean-temp") {
            out.push(CleanCommand { program: "make".into(), args: vec!["clean-temp".into()] });
        } else if has("clean-test") {
            out.push(CleanCommand { program: "make".into(), args: vec!["clean-test".into()] });
        }
    }
    if root.join("package.json").exists() {
        if which("pnpm") {
            out.push(CleanCommand {
                program: "pnpm".into(),
                args: vec!["store".into(), "prune".into()],
            });
        } else if which("npm") {
            out.push(CleanCommand {
                program: "npm".into(),
                args: vec!["cache".into(), "verify".into()],
            });
        }
    }
    out
}

fn which(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join(program).is_file()))
}

/// Parse `git worktree list --porcelain` (`worktree <path>` lines) against
/// the worktrees live lanes hold: `(to_prune, to_remove)`. Missing dirs
/// are dangling admin entries for `git worktree prune`; existing dirs no
/// live lane claims go to `git worktree remove --force`. Only listed paths
/// are ever touched — anything else under the repo is not ours to delete.
pub fn dead_worktrees(
    porcelain: &str,
    live: &std::collections::HashSet<PathBuf>,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut prune = Vec::new();
    let mut remove = Vec::new();
    for line in porcelain.lines().filter_map(|l| l.strip_prefix("worktree ")) {
        let p = PathBuf::from(line);
        if !p.exists() {
            prune.push(p);
        } else if !live.contains(&p) {
            remove.push(p);
        }
    }
    (prune, remove)
}

/// Delete entries under `dir` older than `older_than`, returning bytes
/// reclaimed. Never touches anything younger; a missing dir reclaims zero.
pub fn scavenge_dir(dir: &Path, older_than: Duration) -> u64 {
    fn bytes_of(path: &Path) -> u64 {
        if path.is_file() {
            return std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        }
        let Ok(read) = std::fs::read_dir(path) else { return 0 };
        read.filter_map(|e| e.ok()).map(|e| bytes_of(&e.path())).sum()
    }
    fn remove(path: &Path) {
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
    let Ok(read) = std::fs::read_dir(dir) else { return 0 };
    let mut reclaimed = 0u64;
    for entry in read.filter_map(|e| e.ok()) {
        let path = entry.path();
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| SystemTime::now().duration_since(t).unwrap_or_default() > older_than)
            .unwrap_or(false);
        if stale {
            reclaimed += bytes_of(&path);
            remove(&path);
        }
    }
    reclaimed
}

/// Run one full cascade, every stage guarded: a stage that fails logs and
/// the cascade continues. Returns what the pass reclaimed.
pub async fn run(pg: &crate::db::pg_store::PgStore) -> JanitorReport {
    let mut report = JanitorReport::default();
    let roots: Vec<PathBuf> = pg
        .janitor_git_roots()
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "janitor: git roots unread; fs stages skipped");
            Vec::new()
        })
        .into_iter()
        .map(PathBuf::from)
        .collect();
    let live: std::collections::HashSet<PathBuf> = pg
        .janitor_live_worktrees()
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "janitor: live worktrees unread; pruning skipped");
            Vec::new()
        })
        .into_iter()
        .map(PathBuf::from)
        .collect();

    // Stage 1: dangling worktree pruning.
    for root in &roots {
        match prune_root(root, &live).await {
            Ok(n) => report.pruned_worktrees += n,
            Err(e) => tracing::warn!(root = %root.display(), error = %e, "janitor: prune failed"),
        }
    }
    // Stage 2: scratch scavenging (24 h).
    let day = Duration::from_secs(86_400);
    for root in &roots {
        report.scavenged_bytes += scavenge_dir(&root.join("target/debug/incremental"), day);
        report.scavenged_bytes += scavenge_dir(&root.join(".sensei/scratch"), day);
    }
    for entry in std::fs::read_dir(std::env::temp_dir()).into_iter().flatten().flatten() {
        if entry.file_name().to_string_lossy().starts_with("sensei-") {
            report.scavenged_bytes += scavenge_dir(&entry.path(), day);
        }
    }
    // Stage 3: manifest clean targets.
    for root in &roots {
        for cmd in clean_plan(root) {
            match run_clean(root, &cmd).await {
                Ok(()) => report.manifest_cleans += 1,
                Err(e) => {
                    tracing::warn!(root = %root.display(), cmd = %cmd.program, error = %e, "janitor: clean failed")
                }
            }
        }
    }
    // Stage 4: action-log compaction.
    match pg.janitor_compact_action_logs(chrono::Utc::now()).await {
        Ok(n) => report.compacted_actions = n,
        Err(e) => tracing::warn!(error = %e, "janitor: compaction failed"),
    }
    if report != JanitorReport::default() {
        tracing::info!(
            pruned = report.pruned_worktrees,
            scavenged_bytes = report.scavenged_bytes,
            cleans = report.manifest_cleans,
            compacted = report.compacted_actions,
            "relay_janitor: pass reclaimed",
        );
    }
    report
}

async fn prune_root(root: &Path, live: &std::collections::HashSet<PathBuf>) -> Result<u64, String> {
    let out = tokio::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| format!("git worktree list: {e}"))?;
    if !out.status.success() {
        return Err(format!("git worktree list exited {}", out.status));
    }
    let (prune, remove) = dead_worktrees(&String::from_utf8_lossy(&out.stdout), live);
    let mut n = 0u64;
    if !prune.is_empty() {
        let st = tokio::process::Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(root)
            .status()
            .await
            .map_err(|e| format!("git worktree prune: {e}"))?;
        if st.success() {
            n += prune.len() as u64;
        }
    }
    for path in remove {
        let st = tokio::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&path)
            .current_dir(root)
            .status()
            .await
            .map_err(|e| format!("git worktree remove: {e}"))?;
        if st.success() {
            n += 1;
        }
    }
    Ok(n)
}

async fn run_clean(root: &Path, cmd: &CleanCommand) -> Result<(), String> {
    let st = tokio::time::timeout(
        Duration::from_secs(300),
        tokio::process::Command::new(&cmd.program).args(&cmd.args).current_dir(root).status(),
    )
    .await
    .map_err(|_| format!("{} timed out", cmd.program))?
    .map_err(|e| format!("{}: {e}", cmd.program))?;
    if st.success() { Ok(()) } else { Err(format!("{} exited {st}", cmd.program)) }
}

/// One pass at wall-clock now, for the `relay_janitor` schedule (hourly)
/// and the sentinel's watermark trigger.
pub async fn tick(pg: std::sync::Arc<crate::db::pg_store::PgStore>) -> Result<(), String> {
    run(&pg).await;
    Ok(())
}

/// Spawn the janitor for the daemon's lifetime.
pub fn spawn(pg: std::sync::Arc<crate::db::pg_store::PgStore>) {
    tokio::spawn(serve(pg));
}

async fn serve(pg: std::sync::Arc<crate::db::pg_store::PgStore>) {
    let store = pg.clone();
    crate::tasks::ticker::run_scheduled(
        pg,
        "relay_janitor",
        std::sync::Arc::new(crate::notifications::DesktopNotifier),
        move || {
            let store = store.clone();
            async move { tick(store).await }
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn clean_plan_prefers_clean_temp_then_node_prune() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Makefile"),
            "clean-temp:\n\trm -rf tmp\nclean-test:\n\trm -rf cov\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        let plan = clean_plan(dir.path());
        assert_eq!(plan[0].program, "make");
        assert_eq!(plan[0].args, vec!["clean-temp"]);
        // pnpm or npm, whichever the host has — or neither on a bare box.
        assert!(plan.len() <= 2);
        assert!(plan.iter().skip(1).all(|c| c.program == "pnpm" || c.program == "npm"));
    }

    #[test]
    fn clean_plan_falls_back_to_clean_test() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Makefile"), "clean-test:\n\trm -rf cov\n").unwrap();
        let plan = clean_plan(dir.path());
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].args, vec!["clean-test"]);
    }

    #[test]
    fn clean_plan_empty_without_manifests() {
        let dir = tempfile::tempdir().unwrap();
        assert!(clean_plan(dir.path()).is_empty());
    }

    #[test]
    fn dead_worktrees_keeps_missing_and_unclaimed_only() {
        let live_root = tempfile::tempdir().unwrap();
        let live_wt = live_root.path().join("lanes/L1");
        std::fs::create_dir_all(&live_wt).unwrap();
        let porcelain = format!(
            "worktree {}\nHEAD x\n\nworktree /repo/lanes/L2\nHEAD x\n\nworktree /gone/wt\nHEAD x\n",
            live_wt.display()
        );
        let mut live = HashSet::new();
        live.insert(live_wt);
        live.insert(PathBuf::from("/repo/lanes/L2"));
        let (prune, remove) = dead_worktrees(&porcelain, &live);
        // L2's dir is missing too: a missing dir prunes even for a live lane.
        assert_eq!(prune, vec![PathBuf::from("/repo/lanes/L2"), PathBuf::from("/gone/wt")]);
        assert!(remove.is_empty());
        // An existing dir no live lane claims goes to remove, not prune.
        let mut live2 = HashSet::new();
        live2.insert(live_root.path().join("lanes/L1"));
        let (prune, remove) = dead_worktrees(&porcelain, &live2);
        assert_eq!(prune.len(), 2);
        assert!(remove.is_empty(), "L2's dir is missing too");
        let l2dir = live_root.path().join("lanes/L2");
        std::fs::create_dir_all(&l2dir).unwrap();
        let porcelain2 = format!("worktree {}\n", l2dir.display());
        let (prune, remove) = dead_worktrees(&porcelain2, &HashSet::new());
        assert!(prune.is_empty());
        assert_eq!(remove, vec![l2dir]);
    }

    // mtime aging needs utimes; the scavenger itself is portable.
    #[cfg(unix)]
    #[test]
    fn scavenge_dir_removes_only_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.d");
        let fresh = dir.path().join("fresh.d");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(old.join("a.bin"), vec![7u8; 100]).unwrap();
        std::fs::write(fresh.join("b.bin"), vec![8u8; 50]).unwrap();
        // Age only the old dir (mtime, not ctime, decides).
        let past = SystemTime::now() - Duration::from_secs(100_000);
        filetime_set(&old, past);
        filetime_set(&old.join("a.bin"), past);
        let got = scavenge_dir(dir.path(), Duration::from_secs(86_400));
        assert_eq!(got, 100);
        assert!(!old.exists());
        assert!(fresh.exists());
    }

    #[cfg(unix)]
    fn filetime_set(path: &Path, t: SystemTime) {
        let tv = libc::timeval {
            tv_sec: t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as _,
            tv_usec: 0,
        };
        let tvs = [tv, tv];
        let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        unsafe {
            assert_eq!(libc::utimes(c.as_ptr(), tvs.as_ptr()), 0);
        }
    }
}
