/// Detect ephemeral or banned filesystem locations that should never host a git clone.
/// - Ephemeral: macOS / Linux periodically purge /tmp; git pack files corrupt;
///   subagents that auto-cd to /tmp/<project> bypass the canonical clone.
/// - Banned: agent workspace directories are for config/scripts only, not repos.
use std::path::{Path, PathBuf};

const EPHEMERAL_PREFIXES: &[&str] = &["/tmp/", "/private/tmp/", "/var/tmp/", "/private/var/tmp/"];
const EPHEMERAL_EXACT: &[&str] = &["/tmp", "/private/tmp", "/var/tmp", "/private/var/tmp"];

/// Directories where cloning is permanently banned (agent workspaces, etc.).
/// Loaded from ~/.config/agent-git/banned_paths (one path per line, # comments).
/// Falls back to built-in defaults if file doesn't exist.
const BANNED_PATHS_BUILTIN: &[&str] = &[
    // Agent workspaces are for config/scripts — repos go in project roots
    "/Users/c/.openclaw/workspaces/",
];

fn load_banned_paths() -> Vec<String> {
    let config_path = dirs::home_dir()
        .map(|h| h.join(".config/agent-git/banned_paths"))
        .unwrap_or_default();
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            let mut paths: Vec<String> = content
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(|l| {
                    // Ensure trailing slash for prefix matching
                    if l.ends_with('/') { l.to_string() } else { format!("{}/", l) }
                })
                .collect();
            // Always include builtins
            for b in BANNED_PATHS_BUILTIN {
                if !paths.iter().any(|p| p == *b) {
                    paths.push(b.to_string());
                }
            }
            paths
        }
        Err(_) => BANNED_PATHS_BUILTIN.iter().map(|s| s.to_string()).collect(),
    }
}

/// Resolve a (possibly relative, possibly non-existent) path to an absolute
/// canonical-ish form for prefix comparison. Walks parents until something
/// resolvable is found, then re-appends the missing tail.
fn resolve_for_check(input: &str) -> PathBuf {
    let p = Path::new(input);
    let abs: PathBuf = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|c| c.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    };

    // Try canonicalize the longest existing prefix; append the missing tail back.
    let mut probe = abs.clone();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(c) = probe.canonicalize() {
            let mut out = c;
            for seg in tail.iter().rev() {
                out.push(seg);
            }
            return out;
        }
        match (probe.parent(), probe.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name.to_os_string());
                probe = parent.to_path_buf();
            }
            _ => return abs,
        }
    }
}

pub fn is_ephemeral(path: &str) -> bool {
    let resolved = resolve_for_check(path);
    let s = resolved.to_string_lossy().to_string();
    if EPHEMERAL_EXACT.iter().any(|p| s == *p) {
        return true;
    }
    EPHEMERAL_PREFIXES.iter().any(|p| s.starts_with(p))
}

/// Check if path falls under a banned directory.
pub fn is_banned(path: &str) -> bool {
    let resolved = resolve_for_check(path);
    let s = resolved.to_string_lossy().to_string();
    let banned = load_banned_paths();
    banned.iter().any(|prefix| s.starts_with(prefix.as_str()))
}

/// Which banned prefix matched (for error messages).
pub fn matched_banned_prefix(path: &str) -> Option<String> {
    let resolved = resolve_for_check(path);
    let s = resolved.to_string_lossy().to_string();
    let banned = load_banned_paths();
    banned.into_iter().find(|prefix| s.starts_with(prefix.as_str()))
}

/// Print a uniform refusal message for banned paths and return a non-zero ExitCode.
pub fn refuse_banned(path: &str, action: &str) -> std::process::ExitCode {
    let prefix = matched_banned_prefix(path)
        .unwrap_or_else(|| "<banned>".to_string());
    eprintln!(
        "error: refusing to {action} a git repo under a banned location: {path}"
    );
    eprintln!("hint: '{}' is a banned clone target (agent workspace / config-only directory).", prefix.trim_end_matches('/'));
    eprintln!("hint: clone to the project root instead (e.g. ~/starmap/<project>, ~/dev/<project>, or /Volumes/2t/agents/<agent>/<project>).");
    eprintln!("hint: configure banned paths in ~/.config/agent-git/banned_paths (one path per line).");
    std::process::ExitCode::from(3)
}

/// Print a uniform refusal message and return a non-zero ExitCode.
pub fn refuse_ephemeral(path: &str, action: &str) -> std::process::ExitCode {
    eprintln!(
        "error: refusing to {action} a git repo under an ephemeral location: {path}"
    );
    eprintln!("hint: /tmp, /private/tmp, /var/tmp are auto-purged by the OS — git pack files corrupt and subagents bypass canonical clones.");
    eprintln!("hint: use the agent's project root instead (e.g. ~/dev/<project> on this host, or /Volumes/2t/agents/<agent>/<project> for shared agents).");
    eprintln!("hint: pass --allow-tmp if you really mean it (e.g. throwaway diagnostic clone).");
    std::process::ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tmp() {
        assert!(is_ephemeral("/tmp/foo"));
        assert!(is_ephemeral("/private/tmp/foo"));
        assert!(is_ephemeral("/var/tmp/foo"));
        assert!(is_ephemeral("/tmp"));
    }

    #[test]
    fn ignores_non_tmp() {
        assert!(!is_ephemeral("/Users/c/dev/peopleclaw"));
        assert!(!is_ephemeral("/Volumes/2t/agents/lexis/bitgit"));
        assert!(!is_ephemeral("/home/user/repos/foo"));
    }

    #[test]
    fn detects_banned_workspace() {
        assert!(is_banned("/Users/c/.openclaw/workspaces/starmap/repos/foo"));
        assert!(is_banned("/Users/c/.openclaw/workspaces/nebula/something"));
    }

    #[test]
    fn ignores_non_banned() {
        assert!(!is_banned("/Users/c/starmap/some-repo"));
        assert!(!is_banned("/Users/c/dev/project"));
        assert!(!is_banned("/Volumes/2t/agents/starmap/repo"));
    }
}
