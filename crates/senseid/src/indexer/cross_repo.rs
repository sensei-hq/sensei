/// Detect git subtrees by finding directories that were merged via `git subtree add`.
pub fn detect_git_subtrees_pub(repo_path: &std::path::Path) -> Vec<(String, String)> {
    detect_git_subtrees(repo_path)
}

fn detect_git_subtrees(repo_path: &std::path::Path) -> Vec<(String, String)> {
    let mut subtrees = Vec::new();

    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "--all", "--grep=git-subtree-dir:"])
        .current_dir(repo_path)
        .output();

    if let Ok(out) = output {
        let log = String::from_utf8_lossy(&out.stdout);
        for line in log.lines() {
            if let Some(start) = line.find("Squashed '") {
                let rest = &line[start + 10..];
                if let Some(end) = rest.find("/'") {
                    let dir = &rest[..end];
                    let full_path = repo_path.join(dir);
                    if full_path.is_dir() {
                        subtrees.push((dir.to_string(), full_path.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }

    if subtrees.is_empty() {
        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "--merges", "-20"])
            .current_dir(repo_path)
            .output();

        if let Ok(out) = output {
            let log = String::from_utf8_lossy(&out.stdout);
            for line in log.lines() {
                if let Some(start) = line.find("as '") {
                    let rest = &line[start + 4..];
                    if let Some(end) = rest.find('\'') {
                        let dir = &rest[..end];
                        let full_path = repo_path.join(dir);
                        if full_path.is_dir() && !subtrees.iter().any(|(n, _)| n == dir) {
                            subtrees.push((dir.to_string(), full_path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    subtrees
}
