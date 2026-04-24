/// Detect ephemeral filesystem locations that should never host a git clone
/// (macOS / Linux periodically purge /tmp; git pack files corrupt; subagents
/// that auto-cd to /tmp/<project> bypass the canonical clone).
use std::path::{Path, PathBuf};

const EPHEMERAL_PREFIXES: &[&str] = &["/tmp/", "/private/tmp/", "/var/tmp/", "/private/var/tmp/"];
const EPHEMERAL_EXACT: &[&str] = &["/tmp", "/private/tmp", "/var/tmp", "/private/var/tmp"];

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
}
