//! Canonical remote URL normalization for matching git remotes and GitHub inventory.

use url::Url;

/// Lowercase `host/path` form suitable for equality checks (HTTPS, HTTP, and `git@host:path`).
pub fn normalize_remote_url(input: &str) -> String {
    let trimmed = input.trim();
    if let Ok(url) = Url::parse(trimmed) {
        let host = url.host_str().unwrap_or_default().to_lowercase();
        let path = url
            .path()
            .trim_end_matches(".git")
            .trim_matches('/')
            .to_lowercase();
        if host.is_empty() {
            return fallback(trimmed);
        }
        return format!("{host}/{path}");
    }

    if let Some(stripped) = trimmed.strip_prefix("git@") {
        let normalized = stripped.replace(':', "/");
        return normalized.trim_end_matches(".git").to_lowercase();
    }

    fallback(trimmed)
}

fn fallback(s: &str) -> String {
    s.trim_end_matches(".git").trim_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_github() {
        assert_eq!(
            normalize_remote_url("https://github.com/Foo/Bar.git"),
            "github.com/foo/bar"
        );
    }

    #[test]
    fn ssh_github() {
        assert_eq!(
            normalize_remote_url("git@github.com:Foo/Bar.git"),
            "github.com/foo/bar"
        );
    }
}
