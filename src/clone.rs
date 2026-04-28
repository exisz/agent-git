/// Clone interception logic.
/// Checks registry before cloning, registers after successful clone.
use crate::ephemeral::{is_banned, is_ephemeral, refuse_banned, refuse_ephemeral};
use crate::normalize::normalize_url;
use crate::passthrough::find_real_git;
use crate::registry::Registry;
use std::path::Path;
use std::process::{Command, ExitCode};

/// Handle `agent-git clone <url> [path]`.
/// Returns the exit code to use.
pub fn handle_clone(url: &str, dest: Option<&str>, allow_tmp: bool) -> ExitCode {
    let normalized = normalize_url(url);
    let mut registry = Registry::load();

    // Check if already cloned
    if let Some(existing) = registry.find_by_url(&normalized) {
        eprintln!(
            "error: Repository '{}' is already cloned at: {}",
            normalized, existing.path
        );
        eprintln!("hint: Use 'agent-git whereis {}' to find it", normalized);
        return ExitCode::from(1);
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
        .arg(url)
        .arg(&dest_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            // Resolve to absolute path
            let abs_path = if Path::new(&dest_path).is_absolute() {
                dest_path.clone()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(&dest_path).to_string_lossy().to_string())
                    .unwrap_or(dest_path.clone())
            };

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
