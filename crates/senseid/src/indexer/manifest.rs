use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use crate::types::ManifestEntry;

/// Manifest tracks file states for incremental indexing.
/// Files with matching mtime+hash are skipped on re-index.
pub struct Manifest {
    entries: HashMap<String, ManifestEntry>,
    path: PathBuf,
}

impl Manifest {
    /// Load manifest from disk (or empty if missing).
    pub fn load(repo_id: &str) -> Self {
        let path = manifest_path(repo_id);
        let entries = match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => HashMap::new(),
        };
        Self { entries, path }
    }

    /// Check if a file has changed since last index.
    pub fn is_changed(&self, abs_path: &str, mtime: u64) -> bool {
        match self.entries.get(abs_path) {
            Some(entry) => entry.mtime != mtime,
            None => true, // new file
        }
    }

    /// Check by hash (for cases where mtime is unreliable, e.g. git checkout).
    pub fn is_hash_changed(&self, abs_path: &str, hash: &str) -> bool {
        match self.entries.get(abs_path) {
            Some(entry) => entry.hash != hash,
            None => true,
        }
    }

    /// Record a file as indexed.
    pub fn record(&mut self, abs_path: &str, mtime: u64, hash: String) {
        self.entries.insert(abs_path.to_string(), ManifestEntry { mtime, hash });
    }

    /// Remove a file from the manifest (deleted from disk).
    pub fn remove(&mut self, abs_path: &str) {
        self.entries.remove(abs_path);
    }

    /// Get all tracked paths.
    pub fn tracked_paths(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// Persist to disk.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.path, json)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn manifest_path(repo_id: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".sensei")
        .join("projects")
        .join(repo_id)
        .join("manifest.json")
}

/// Compute SHA-256 hash of a file (first 16 hex chars).
pub fn file_hash(path: &Path) -> std::io::Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(hex::encode(&result[..8])) // 16 hex chars = 8 bytes
}

/// Get file mtime as milliseconds since epoch.
pub fn file_mtime(path: &Path) -> std::io::Result<u64> {
    let metadata = fs::metadata(path)?;
    let mtime = metadata.modified()?;
    Ok(mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_manifest_is_empty() {
        let m = Manifest { entries: HashMap::new(), path: PathBuf::from("/tmp/test") };
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn is_changed_for_new_file() {
        let m = Manifest { entries: HashMap::new(), path: PathBuf::from("/tmp/test") };
        assert!(m.is_changed("/foo.ts", 1000));
    }

    #[test]
    fn is_changed_for_same_mtime() {
        let mut m = Manifest { entries: HashMap::new(), path: PathBuf::from("/tmp/test") };
        m.record("/foo.ts", 1000, "abc123".into());
        assert!(!m.is_changed("/foo.ts", 1000));
        assert!(m.is_changed("/foo.ts", 2000));
    }

    #[test]
    fn hash_check() {
        let mut m = Manifest { entries: HashMap::new(), path: PathBuf::from("/tmp/test") };
        m.record("/foo.ts", 1000, "abc123".into());
        assert!(!m.is_hash_changed("/foo.ts", "abc123"));
        assert!(m.is_hash_changed("/foo.ts", "xyz789"));
    }

    #[test]
    fn record_and_remove() {
        let mut m = Manifest { entries: HashMap::new(), path: PathBuf::from("/tmp/test") };
        m.record("/a.ts", 100, "h1".into());
        m.record("/b.ts", 200, "h2".into());
        assert_eq!(m.len(), 2);
        m.remove("/a.ts");
        assert_eq!(m.len(), 1);
        assert!(m.is_changed("/a.ts", 100));
    }

    #[test]
    fn file_hash_works() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world").unwrap();
        let hash = file_hash(&file).unwrap();
        assert_eq!(hash.len(), 16); // 8 bytes = 16 hex chars

        // Same content = same hash
        let file2 = dir.path().join("test2.txt");
        fs::write(&file2, "hello world").unwrap();
        assert_eq!(file_hash(&file2).unwrap(), hash);

        // Different content = different hash
        let file3 = dir.path().join("test3.txt");
        fs::write(&file3, "goodbye").unwrap();
        assert_ne!(file_hash(&file3).unwrap(), hash);
    }

    #[test]
    fn save_and_load() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("manifest.json");

        let mut m = Manifest { entries: HashMap::new(), path: path.clone() };
        m.record("/a.ts", 100, "h1".into());
        m.record("/b.ts", 200, "h2".into());
        m.save().unwrap();

        // Reload
        let json = fs::read_to_string(&path).unwrap();
        let entries: HashMap<String, ManifestEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries["/a.ts"].hash, "h1");
    }
}
