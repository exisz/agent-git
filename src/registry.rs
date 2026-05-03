/// Registry management for tracked git repositories.
/// Stores repo metadata in ~/.agentgit (TOML format).
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoEntry {
    pub url: String,
    pub path: String,
    pub cloned_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub banned_paths: Vec<String>,
}

/// Result of `Registry::take_alive_by_url`.
#[derive(Debug, Clone)]
pub enum AliveLookup {
    /// No registry entry for this URL.
    Missing,
    /// Entry exists and the on-disk clone is alive.
    Alive(RepoEntry),
    /// Entry existed but the on-disk path was stale; entry has been removed
    /// from the in-memory registry. Caller should `save()` and proceed as if
    /// no clone existed.
    Pruned(RepoEntry),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Registry {
    #[serde(default)]
    pub config: Config,
    #[serde(default)]
    pub repos: Vec<RepoEntry>,
}

impl Registry {
    /// Get the registry file path (~/.agentgit)
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".agentgit")
    }

    /// Get banned paths from config, ensuring trailing slashes.
    pub fn banned_paths(&self) -> Vec<String> {
        self.config.banned_paths.iter().map(|p| {
            if p.ends_with('/') { p.clone() } else { format!("{}/", p) }
        }).collect()
    }

    /// Load the registry from disk. Returns empty registry if file doesn't exist.
    pub fn load() -> Self {
        Self::load_from(&Self::path())
    }

    /// Load from a specific path (useful for testing).
    pub fn load_from(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Registry::default(),
        }
    }

    /// Save the registry to disk.
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&Self::path())
    }

    /// Save to a specific path (useful for testing).
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }

    /// Find a repo by normalized URL.
    pub fn find_by_url(&self, normalized_url: &str) -> Option<&RepoEntry> {
        self.repos.iter().find(|r| r.url == normalized_url)
    }

    /// Check whether a registered repo path is still a live git repo.
    /// Returns true iff `<path>/.git` exists (covers regular repos AND worktrees,
    /// where `.git` is a file pointing at the gitdir).
    pub fn path_is_alive(path: &str) -> bool {
        Path::new(path).join(".git").exists()
    }

    /// Find a repo by URL, but auto-prune the entry if its on-disk path
    /// is no longer a git repo. Returns `Some(entry_clone)` only when the
    /// registered clone is still alive on disk.
    ///
    /// On stale-entry pruning, the caller is told via `pruned`. The registry
    /// is mutated in-memory; persist with `save()` to make it stick.
    pub fn take_alive_by_url(&mut self, normalized_url: &str) -> AliveLookup {
        let pos = self.repos.iter().position(|r| r.url == normalized_url);
        match pos {
            None => AliveLookup::Missing,
            Some(i) => {
                if Self::path_is_alive(&self.repos[i].path) {
                    AliveLookup::Alive(self.repos[i].clone())
                } else {
                    let pruned = self.repos.remove(i);
                    AliveLookup::Pruned(pruned)
                }
            }
        }
    }

    /// Find a repo by path.
    pub fn find_by_path(&self, path: &str) -> Option<&RepoEntry> {
        self.repos.iter().find(|r| r.path == path)
    }

    /// Register a new repo. Returns Err if URL already exists at a still-alive path.
    /// If the existing entry is stale (on-disk path gone), it is silently pruned
    /// and replaced with the new one — this is the self-healing path that lets
    /// users recover from `mv`/`rm -rf` of a registered clone without manual
    /// `agent-git unregister`.
    pub fn register(&mut self, url: String, path: String) -> Result<(), String> {
        if let Some(pos) = self.repos.iter().position(|r| r.url == url) {
            let existing_path = self.repos[pos].path.clone();
            if Self::path_is_alive(&existing_path) {
                return Err(format!(
                    "Repository '{}' is already cloned at: {}",
                    url, existing_path
                ));
            }
            // Stale entry — prune it and fall through to register fresh.
            self.repos.remove(pos);
            eprintln!(
                "agent-git: pruned stale registry entry — '{}' was registered at '{}' but the directory is gone",
                url, existing_path
            );
        }
        self.repos.push(RepoEntry {
            url,
            path,
            cloned_at: Utc::now(),
        });
        Ok(())
    }

    /// Unregister a repo by path. Returns true if found and removed.
    pub fn unregister_by_path(&mut self, path: &str) -> bool {
        let len_before = self.repos.len();
        self.repos.retain(|r| r.path != path);
        self.repos.len() < len_before
    }

    /// Unregister a repo by normalized URL. Returns true if found and removed.
    pub fn unregister_by_url(&mut self, url: &str) -> bool {
        let len_before = self.repos.len();
        self.repos.retain(|r| r.url != url);
        self.repos.len() < len_before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_empty() {
        let tmp = NamedTempFile::new().unwrap();
        let registry = Registry::load_from(tmp.path());
        assert!(registry.repos.is_empty());
    }

    #[test]
    fn test_load_nonexistent() {
        let registry = Registry::load_from(Path::new("/tmp/nonexistent_agentgit_test"));
        assert!(registry.repos.is_empty());
    }

    #[test]
    fn test_register_and_find() {
        let mut registry = Registry::default();
        registry
            .register(
                "github.com/user/repo".to_string(),
                "/Users/c/repos/repo".to_string(),
            )
            .unwrap();

        assert!(registry.find_by_url("github.com/user/repo").is_some());
        assert!(registry.find_by_path("/Users/c/repos/repo").is_some());
        assert!(registry.find_by_url("github.com/other/repo").is_none());
    }

    #[test]
    fn test_register_duplicate_url() {
        // Use a path we know is alive (the cargo manifest dir is itself a git repo).
        let alive_path = env!("CARGO_MANIFEST_DIR").to_string();
        let mut registry = Registry::default();
        registry
            .register(
                "github.com/user/repo".to_string(),
                alive_path.clone(),
            )
            .unwrap();

        let result = registry.register(
            "github.com/user/repo".to_string(),
            "/Users/c/repos/repo2".to_string(),
        );
        assert!(result.is_err(), "duplicate against ALIVE path must error");
        assert!(result.unwrap_err().contains("already cloned"));
    }

    #[test]
    fn test_register_self_heals_stale_entry() {
        // A previous clone got rm -rf'd outside of agent-git’s knowledge.
        // The next `register` (or `clone`) for the same URL should auto-prune
        // the stale entry and accept the new path.
        let mut registry = Registry::default();
        registry
            .register(
                "github.com/user/repo".to_string(),
                "/nonexistent/agentgit_stale_path/repo".to_string(),
            )
            .unwrap();

        // Same URL, new (also nonexistent for the test — we only care that it’s
        // accepted because the *previous* path is dead).
        let result = registry.register(
            "github.com/user/repo".to_string(),
            "/another/path/repo".to_string(),
        );
        assert!(result.is_ok(), "stale entry should be pruned, got: {:?}", result);
        assert_eq!(registry.repos.len(), 1);
        assert_eq!(registry.repos[0].path, "/another/path/repo");
    }

    #[test]
    fn test_take_alive_by_url_missing() {
        let mut registry = Registry::default();
        match registry.take_alive_by_url("github.com/user/none") {
            AliveLookup::Missing => {}
            other => panic!("expected Missing, got {:?}", other),
        }
    }

    #[test]
    fn test_take_alive_by_url_prunes_dead_path() {
        let mut registry = Registry::default();
        registry.repos.push(RepoEntry {
            url: "github.com/user/repo".to_string(),
            path: "/definitely/does/not/exist/repo".to_string(),
            cloned_at: Utc::now(),
        });

        match registry.take_alive_by_url("github.com/user/repo") {
            AliveLookup::Pruned(e) => assert_eq!(e.path, "/definitely/does/not/exist/repo"),
            other => panic!("expected Pruned, got {:?}", other),
        }
        assert!(registry.repos.is_empty(), "stale entry should be removed");
    }

    #[test]
    fn test_take_alive_by_url_keeps_live_path() {
        // The cargo manifest dir is itself a git repo (has .git).
        let here = env!("CARGO_MANIFEST_DIR").to_string();
        let mut registry = Registry::default();
        registry.repos.push(RepoEntry {
            url: "github.com/agent-git/self".to_string(),
            path: here.clone(),
            cloned_at: Utc::now(),
        });

        match registry.take_alive_by_url("github.com/agent-git/self") {
            AliveLookup::Alive(e) => assert_eq!(e.path, here),
            other => panic!("expected Alive, got {:?}", other),
        }
        assert_eq!(registry.repos.len(), 1, "live entry should be kept");
    }

    #[test]
    fn test_unregister_by_path() {
        let mut registry = Registry::default();
        registry
            .register(
                "github.com/user/repo".to_string(),
                "/Users/c/repos/repo".to_string(),
            )
            .unwrap();

        assert!(registry.unregister_by_path("/Users/c/repos/repo"));
        assert!(registry.repos.is_empty());
        assert!(!registry.unregister_by_path("/Users/c/repos/repo"));
    }

    #[test]
    fn test_unregister_by_url() {
        let mut registry = Registry::default();
        registry
            .register(
                "github.com/user/repo".to_string(),
                "/Users/c/repos/repo".to_string(),
            )
            .unwrap();

        assert!(registry.unregister_by_url("github.com/user/repo"));
        assert!(registry.repos.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let mut registry = Registry::default();
        registry
            .register(
                "github.com/user/repo".to_string(),
                "/Users/c/repos/repo".to_string(),
            )
            .unwrap();
        registry
            .register(
                "gitlab.com/org/project".to_string(),
                "/Users/c/repos/project".to_string(),
            )
            .unwrap();

        registry.save_to(tmp.path()).unwrap();

        let loaded = Registry::load_from(tmp.path());
        assert_eq!(loaded.repos.len(), 2);
        assert_eq!(loaded.repos[0].url, "github.com/user/repo");
        assert_eq!(loaded.repos[1].url, "gitlab.com/org/project");
    }

    #[test]
    fn test_load_existing_toml() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            r#"[[repos]]
url = "github.com/user/repo"
path = "/Users/c/repos/repo"
cloned_at = "2026-04-16T01:09:00Z"
"#
        )
        .unwrap();

        let registry = Registry::load_from(tmp.path());
        assert_eq!(registry.repos.len(), 1);
        assert_eq!(registry.repos[0].url, "github.com/user/repo");
    }
}
