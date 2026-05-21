/// Clone interception logic.
/// Checks registry before cloning, registers after successful clone.
use crate::ephemeral::{is_banned, is_ephemeral, refuse_banned, refuse_ephemeral};
use crate::normalize::normalize_url;
use crate::passthrough::find_real_git;
use crate::registry::{AliveLookup, Registry};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Handle `agent-git clone <url> [path]`.
/// Returns the exit code to use.
pub fn handle_clone(url: &str, dest: Option<&str>, allow_tmp: bool, extra: &[String]) -> ExitCode {
    let normalized = normalize_url(url);
    let mut registry = Registry::load();

    // Check if already cloned (auto-prune stale entries pointing at gone dirs).
    match registry.take_alive_by_url(&normalized) {
        AliveLookup::Alive(existing) => {
            eprintln!(
                "error: Repository '{}' is already cloned at: {}",
                normalized, existing.path
            );
            eprintln!("hint: Use 'agent-git whereis {}' to find it", normalized);
            return ExitCode::from(1);
        }
        AliveLookup::Pruned(stale) => {
            eprintln!(
                "agent-git: pruned stale registry entry — '{}' was registered at '{}' but the directory is gone",
                normalized, stale.path
            );
            if let Err(e) = registry.save() {
                eprintln!("warning: failed to persist pruned registry: {}", e);
            }
        }
        AliveLookup::Missing => {}
    }

    // Determine destination path
    let dest_path = match dest {
        Some(d) => d.to_string(),
        None => {
            // Extract repo name from URL for default path
            normalized.rsplit('/').next().unwrap_or("repo").to_string()
        }
    };

    // Reject ephemeral target locations (/tmp, /private/tmp, /var/tmp).
    // Subagents auto-cd'ing to /tmp/<project> is the #1 footgun this guards.
    if !allow_tmp && is_ephemeral(&dest_path) {
        return refuse_ephemeral(&dest_path, "clone");
    }

    // Reject banned target locations (agent workspaces, etc.).
    if is_banned(&dest_path) {
        return refuse_banned(&dest_path, "clone");
    }

    // Find real git and run clone
    let real_git = match find_real_git() {
        Some(g) => g,
        None => {
            eprintln!("error: Could not find real git binary");
            return ExitCode::from(1);
        }
    };

    let status = Command::new(&real_git)
        .arg("clone")
        .args(extra)
        .arg(url)
        .arg(&dest_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            // Register the clone by canonical path, not the symlink spelling the
            // caller used. This keeps the registry honest for disk-location
            // audits: a clone through ~/2t/foo must be recorded as /Volumes/2t/foo.
            let abs_path = canonical_clone_path(&dest_path);

            // Register the clone
            if let Err(e) = registry.register(normalized, abs_path) {
                eprintln!("warning: Clone succeeded but failed to register: {}", e);
            } else if let Err(e) = registry.save() {
                eprintln!(
                    "warning: Clone succeeded but failed to save registry: {}",
                    e
                );
            } else {
                eprintln!("agent-git: Registered in ~/.agentgit");
            }
            ExitCode::SUCCESS
        }
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("error: Failed to run git clone: {}", e);
            ExitCode::from(1)
        }
    }
}

fn absolute_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

fn canonical_clone_path(path: &str) -> String {
    let abs = absolute_path(path);
    abs.canonicalize()
        .unwrap_or(abs)
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{absolute_path, canonical_clone_path};
    use std::fs;

    #[test]
    fn canonical_clone_path_resolves_symlink_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real-repo");
        let link = tmp.path().join("link-repo");
        fs::create_dir(&real).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        assert_eq!(
            canonical_clone_path(link.to_str().unwrap()),
            real.to_string_lossy()
        );
    }

    #[test]
    fn absolute_path_keeps_absolute_paths_absolute() {
        let p = absolute_path("/tmp/agent-git-test");
        assert_eq!(p.to_string_lossy(), "/tmp/agent-git-test");
    }
}
