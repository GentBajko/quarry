//! The repo name and normalized origin, derived from the `origin` remote.

use crate::errors::{QuarryError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepoIdentity {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) origin: String,
}

pub(crate) fn from_origin_url(url: &str) -> Result<RepoIdentity> {
    let raw = url.trim();
    let (host, path) = split_host_path(raw)
        .ok_or_else(|| QuarryError::refusal(format!("cannot derive owner/repo from {raw}")))?;
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return Err(QuarryError::refusal(format!(
            "cannot derive owner/repo from {raw}"
        )));
    }
    let repo = segments[segments.len() - 1].to_string();
    let owner = segments[segments.len() - 2].to_string();
    let host = host.to_ascii_lowercase();
    let origin = format!(
        "{host}/{}/{}",
        owner.to_ascii_lowercase(),
        repo.to_ascii_lowercase()
    );
    Ok(RepoIdentity {
        name: repo.clone(),
        host,
        owner,
        repo,
        origin,
    })
}

pub(crate) fn clone_name(url: &str) -> Result<String> {
    let raw = url.trim().trim_end_matches('/');
    let raw = raw.strip_suffix(".git").unwrap_or(raw);
    let last = raw
        .rsplit(['/', ':'])
        .find(|s| !s.is_empty())
        .ok_or_else(|| QuarryError::refusal(format!("cannot derive a folder name from {url}")))?;
    Ok(last.to_string())
}

fn split_host_path(url: &str) -> Option<(String, &str)> {
    if let Some(rest) = url.strip_prefix("ssh://") {
        return split_authority(rest);
    }
    if let Some(rest) = url.strip_prefix("https://") {
        return split_authority(rest);
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return split_authority(rest);
    }
    if let Some(rest) = url.strip_prefix("git://") {
        return split_authority(rest);
    }
    if let Some(rest) = url.strip_prefix("file://") {
        return Some(("localhost".to_string(), rest));
    }
    if let Some((authority, path)) = url.split_once(':') {
        // scp-like: git@host:owner/repo
        let host = authority.rsplit('@').next()?;
        if host.is_empty() || path.is_empty() {
            return None;
        }
        return Some((host.to_string(), path));
    }
    if url.starts_with('/') || url.starts_with('.') {
        return Some(("localhost".to_string(), url));
    }
    None
}

fn split_authority(rest: &str) -> Option<(String, &str)> {
    let (authority, path) = rest.split_once('/')?;
    let host = authority.rsplit('@').next()?;
    let host = host.split(':').next()?;
    if host.is_empty() {
        return None;
    }
    Some((host.to_string(), path))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn s9_scp_form() {
        let id = from_origin_url("git@github.com:acme/ingest-api.git").unwrap();
        assert_eq!(id.name, "ingest-api");
        assert_eq!(id.owner, "acme");
        assert_eq!(id.host, "github.com");
        assert_eq!(id.origin, "github.com/acme/ingest-api");
    }

    #[test]
    fn s9_https_form_without_git_suffix() {
        let id = from_origin_url("https://gitlab.com/Acme/Ingest-API").unwrap();
        assert_eq!(id.name, "Ingest-API");
        assert_eq!(id.origin, "gitlab.com/acme/ingest-api");
    }

    #[test]
    fn s9_ssh_url_form_with_port() {
        let id = from_origin_url("ssh://git@git.internal:2222/team/svc.git").unwrap();
        assert_eq!(id.host, "git.internal");
        assert_eq!(id.name, "svc");
    }

    #[test]
    fn s9_single_segment_refuses() {
        assert!(from_origin_url("https://github.com/repo").is_err());
    }

    #[test]
    fn s9_case_is_preserved_in_the_name() {
        let id = from_origin_url("git@github.com:Acme/Foo.git").unwrap();
        assert_eq!(id.name, "Foo");
        assert_eq!(id.origin, "github.com/acme/foo");
    }

    #[test]
    fn clone_name_strips_git_suffix() {
        assert_eq!(
            clone_name("git@github.com:acme/docs-quarry.git").unwrap(),
            "docs-quarry"
        );
        assert_eq!(clone_name("file:///tmp/x/remote.git").unwrap(), "remote");
    }
}
