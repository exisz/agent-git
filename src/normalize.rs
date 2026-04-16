/// URL normalization for git repository URLs.
/// Converts various git URL formats to a canonical form: `host/user/repo`

/// Normalize a git URL to canonical form `host/user/repo`.
///
/// Handles:
/// - `https://github.com/user/repo.git` → `github.com/user/repo`
/// - `git@github.com:user/repo.git` → `github.com/user/repo`
/// - `https://github.com/user/repo` → `github.com/user/repo`
pub fn normalize_url(url: &str) -> String {
    let url = url.trim();

    // Handle SSH format: git@host:user/repo.git
    if let Some(rest) = url.strip_prefix("git@") {
        let normalized = rest.replace(':', "/");
        return strip_dot_git(&normalized);
    }

    // Handle HTTPS/HTTP format
    if url.starts_with("https://") || url.starts_with("http://") {
        let without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        return strip_dot_git(without_scheme);
    }

    // Already normalized or unknown format — strip .git just in case
    strip_dot_git(url)
}

fn strip_dot_git(s: &str) -> String {
    let s = s.strip_suffix('/').unwrap_or(s);
    s.strip_suffix(".git").unwrap_or(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_https_with_git_suffix() {
        assert_eq!(
            normalize_url("https://github.com/user/repo.git"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_https_without_git_suffix() {
        assert_eq!(
            normalize_url("https://github.com/user/repo"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_ssh_format() {
        assert_eq!(
            normalize_url("git@github.com:user/repo.git"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_ssh_without_git_suffix() {
        assert_eq!(
            normalize_url("git@github.com:user/repo"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_http_format() {
        assert_eq!(
            normalize_url("http://github.com/user/repo.git"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_already_normalized() {
        assert_eq!(
            normalize_url("github.com/user/repo"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_trailing_slash() {
        assert_eq!(
            normalize_url("https://github.com/user/repo/"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(
            normalize_url("  https://github.com/user/repo  "),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_gitlab_ssh() {
        assert_eq!(
            normalize_url("git@gitlab.com:org/project.git"),
            "gitlab.com/org/project"
        );
    }
}
