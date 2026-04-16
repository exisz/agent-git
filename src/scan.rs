/// Directory scanner for git repositories.
/// Scans directories to find and register git repos.
use crate::normalize::normalize_url;
use crate::registry::Registry;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Scan a directory for git repositories and register them.
/// Scans one level deep by default.
pub fn scan_directory(dir: &str) -> u32 {
    let dir = if dir.is_empty() { "." } else { dir };
    let dir_path = Path::new(dir);

    if !dir_path.exists() || !dir_path.is_dir() {
        eprintln!("error: '{}' is not a valid directory", dir);
        return 0;
    }

    let mut registry = Registry::load();
    let mut count = 0u32;

    // Check if the directory itself is a git repo
    if dir_path.join(".git").exists() {
        if register_repo_at(dir_path, &mut registry) {
            count += 1;
        }
    }

    // Scan immediate subdirectories
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join(".git").exists() {
                if register_repo_at(&path, &mut registry) {
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        if let Err(e) = registry.save() {
            eprintln!("warning: Failed to save registry: {}", e);
        }
    }

    println!("Scanned '{}': {} new repos registered", dir, count);
    count
}

/// Try to register a git repo at the given path.
/// Returns true if newly registered.
fn register_repo_at(path: &Path, registry: &mut Registry) -> bool {
    let abs_path = match path.canonicalize() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return false,
    };

    // Check if already registered by path
    if registry.find_by_path(&abs_path).is_some() {
        return false;
    }

    // Try to get remote URL
    let url = get_remote_url(path);
    let normalized = match &url {
        Some(u) => normalize_url(u),
        None => {
            // No remote — use path-based key
            format!("local:{}", abs_path)
        }
    };

    // Check if URL already registered
    if registry.find_by_url(&normalized).is_some() {
        return false;
    }

    match registry.register(normalized.clone(), abs_path.clone()) {
        Ok(_) => {
            println!("  + {} → {}", normalized, abs_path);
            true
        }
        Err(_) => false,
    }
}

/// Get the remote origin URL for a git repo.
fn get_remote_url(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if url.is_empty() {
            None
        } else {
            Some(url)
        }
    } else {
        None
    }
}
