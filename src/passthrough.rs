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
    // Intercept raw `git clone <url> [dest]` so /tmp targets are refused
    // even when callers bypass the `agent-git clone` subcommand.
    if let Some(refusal) = guard_clone(args) {
        return refusal;
    }

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

/// If args look like `git clone <url> [dest]` and the resolved destination
/// would land under an ephemeral location, refuse (unless caller passed
/// `--allow-tmp`, which we strip before exec — the real git doesn't know it).
fn guard_clone(args: &[String]) -> Option<ExitCode> {
    use crate::ephemeral::{is_banned, is_ephemeral, refuse_banned, refuse_ephemeral};
    use crate::normalize::normalize_url;
    use crate::registry::Registry;

    // Find subcommand position (skip leading `-c key=val`, `--git-dir=...`, etc.)
    let mut sub_idx = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-c" || a == "--exec-path" || a == "--git-dir" || a == "--work-tree" || a == "-C" {
            i += 2;
            continue;
        }
        if a.starts_with("--git-dir=") || a.starts_with("--work-tree=") || a.starts_with("-C=") {
            i += 1;
            continue;
        }
        if a.starts_with('-') {
            i += 1;
            continue;
        }
        sub_idx = Some(i);
        break;
    }
    let sub_idx = sub_idx?;
    if args[sub_idx] != "clone" {
        return None;
    }

    // Allow-tmp escape hatch (we strip it from real-git args downstream — but
    // since we just refuse here, we don't need to actually strip).
    let allow_tmp = args.iter().any(|a| a == "--allow-tmp");

    // Walk past `clone` and its flags to find positional <url> [dest].
    let mut positionals: Vec<&String> = Vec::new();
    let mut j = sub_idx + 1;
    while j < args.len() {
        let a = &args[j];
        // Flags that take a value
        if matches!(a.as_str(), "--branch" | "-b" | "--depth" | "--origin" | "-o" | "--reference" | "--reference-if-able" | "--separate-git-dir" | "--shallow-since" | "--shallow-exclude" | "--recurse-submodules" | "-j" | "--jobs" | "--filter" | "--template" | "-c" | "--config" | "--server-option" | "-u" | "--upload-pack") {
            j += 2;
            continue;
        }
        if a.starts_with('-') {
            j += 1;
            continue;
        }
        positionals.push(a);
        j += 1;
    }
    if positionals.is_empty() {
        return None;
    }
    let url = positionals[0].clone();
    let dest = if positionals.len() > 1 {
        positionals[1].clone()
    } else {
        // git's default: derive from URL basename, stripping .git
        let base = url.rsplit('/').next().unwrap_or("repo").to_string();
        base.trim_end_matches(".git").to_string()
    };

    // 1) Duplicate-clone guard (registry hit) — even via raw `git clone`.
    let normalized = normalize_url(&url);
    let registry = Registry::load();
    if let Some(existing) = registry.find_by_url(&normalized) {
        eprintln!(
            "error: Repository '{}' is already cloned at: {}",
            normalized, existing.path
        );
        eprintln!("hint: Use 'agent-git whereis {}' to find it", normalized);
        return Some(ExitCode::from(1));
    }

    // 2) Ephemeral-target guard.
    if !allow_tmp && is_ephemeral(&dest) {
        return Some(refuse_ephemeral(&dest, "clone"));
    }

    // 3) Banned-path guard (agent workspaces, etc.).
    if is_banned(&dest) {
        return Some(refuse_banned(&dest, "clone"));
    }

    None
}
