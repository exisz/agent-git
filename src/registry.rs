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

    /// Find a repo by path.
    pub fn find_by_path(&self, path: &str) -> Option<&RepoEntry> {
        self.repos.iter().find(|r| r.path == path)
    }

    /// Register a new repo. Returns Err if URL already exists.
    pub fn register(&mut self, url: String, path: String) -> Result<(), String> {
        if let Some(existing) = self.find_by_url(&url) {
            return Err(format!(
                "Repository '{}' is already cloned at: {}",
                url, existing.path
            ));
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
        let mut registry = Registry::default();
        registry
            .register(
                "github.com/user/repo".to_string(),
                "/Users/c/repos/repo".to_string(),
            )
            .unwrap();

        let result = registry.register(
            "github.com/user/repo".to_string(),
            "/Users/c/repos/repo2".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already cloned"));
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
