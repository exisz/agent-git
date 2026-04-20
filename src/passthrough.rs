/// Passthrough: forward unknown commands to real git.
/// Finds the real git binary (skipping self) and executes with all args.
use std::env;
use std::process::{Command, ExitCode};

/// Find the real git binary, skipping agent-git itself.
/// Uses `which -a git` to list all git binaries, then picks the first one
/// that isn't our own binary.
pub fn find_real_git() -> Option<String> {
    let self_exe = env::current_exe().ok()?;
    let self_path = self_exe.canonicalize().ok()?;

    // Try `which -a git` first
    if let Ok(output) = Command::new("which").arg("-a").arg("git").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // Resolve symlinks and compare
                if let Ok(candidate) = std::fs::canonicalize(line) {
                    if candidate != self_path {
                        return Some(line.to_string());
                    }
                } else {
                    // If we can't canonicalize, do a simple string comparison
                    if line != self_path.to_string_lossy() {
                        return Some(line.to_string());
                    }
                }
            }
        }
    }

    // Fallback: try common locations
    for path in &[
        "/usr/bin/git",
        "/usr/local/bin/git",
        "/opt/homebrew/bin/git",
    ] {
        if std::path::Path::new(path).exists() {
            if let Ok(candidate) = std::fs::canonicalize(path) {
                if candidate != self_path {
                    return Some(path.to_string());
                }
            }
        }
    }

    None
}

/// Execute a git command with passthrough, preserving exit code.
pub fn passthrough(args: &[String]) -> ExitCode {
    let real_git = match find_real_git() {
        Some(g) => g,
        None => {
            eprintln!("error: Could not find real git binary");
            return ExitCode::from(1);
        }
    };

    let status = Command::new(&real_git).args(args).status();

    match status {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("error: Failed to run git: {}", e);
            ExitCode::from(1)
        }
    }
}
