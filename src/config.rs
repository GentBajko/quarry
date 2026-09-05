//! `.quarry/.config` and `.quarry/.gitignore`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{QuarryError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) default_branch: String,
    pub(crate) docs_dir: String,
    #[serde(default)]
    pub(crate) permalink_template: Option<String>,
    pub(crate) url: String,
}

pub(crate) const DEFAULT_DOCS_DIR: &str = "docs/capstone";

pub(crate) fn quarry_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".quarry")
}

fn config_path(repo_root: &Path) -> PathBuf {
    quarry_dir(repo_root).join(".config")
}

pub(crate) fn load(repo_root: &Path) -> Result<Option<Config>> {
    let path = config_path(repo_root);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)?;
    let config: Config = serde_json::from_str(&text)
        .map_err(|e| QuarryError::refusal(format!("{} is not readable: {e}", path.display())))?;
    Ok(Some(config))
}

pub(crate) fn write(repo_root: &Path, config: &Config) -> Result<()> {
    fs::create_dir_all(quarry_dir(repo_root))?;
    let mut text = serde_json::to_string_pretty(config)?;
    text.push('\n');
    fs::write(config_path(repo_root), text)?;
    Ok(())
}

pub(crate) fn write_gitignore(repo_root: &Path, clone_name: &str) -> Result<()> {
    fs::create_dir_all(quarry_dir(repo_root))?;
    let body = format!("{clone_name}/\n.docs-index.sqlite\n.docs-index.sqlite.tmp\n.build-*/\n");
    fs::write(quarry_dir(repo_root).join(".gitignore"), body)?;
    Ok(())
}

pub(crate) fn resolve(
    url_flag: Option<String>,
    docs_dir_flag: Option<String>,
    existing: Option<&Config>,
    default_branch: String,
) -> Result<Config> {
    let url = url_flag
        .or_else(|| existing.map(|c| c.url.clone()))
        .ok_or_else(|| {
            QuarryError::refusal(
                "no docs repo URL: pass --url or set QUARRY_DOCS_REPO (no terminal to ask on)",
            )
        })?;
    let docs_dir = docs_dir_flag
        .or_else(|| existing.map(|c| c.docs_dir.clone()))
        .unwrap_or_else(|| DEFAULT_DOCS_DIR.to_string());
    validate_docs_dir(&docs_dir)?;
    Ok(Config {
        default_branch,
        docs_dir,
        permalink_template: existing.and_then(|c| c.permalink_template.clone()),
        url,
    })
}

fn validate_docs_dir(docs_dir: &str) -> Result<()> {
    let path = Path::new(docs_dir);
    if path.is_absolute() || docs_dir.split('/').any(|c| c == "..") {
        return Err(QuarryError::refusal(format!(
            "docs dir must be a relative path inside the repo: {docs_dir}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn config(url: &str) -> Config {
        Config {
            default_branch: "main".into(),
            docs_dir: "docs/capstone".into(),
            permalink_template: None,
            url: url.into(),
        }
    }

    #[test]
    fn s12_flag_beats_existing_config() {
        let existing = config("git@host:a/old.git");
        let c = resolve(
            Some("git@host:a/new.git".into()),
            None,
            Some(&existing),
            "main".into(),
        )
        .unwrap();
        assert_eq!(c.url, "git@host:a/new.git");
        assert_eq!(c.docs_dir, "docs/capstone");
    }

    #[test]
    fn s12_no_url_anywhere_refuses() {
        assert!(resolve(None, None, None, "main".into()).is_err());
    }

    #[test]
    fn s12_docs_dir_escaping_the_repo_refuses() {
        assert!(
            resolve(
                Some("u".into()),
                Some("../elsewhere".into()),
                None,
                "main".into()
            )
            .is_err()
        );
        assert!(resolve(Some("u".into()), Some("/etc".into()), None, "main".into()).is_err());
    }

    #[test]
    fn s12_config_round_trips_with_sorted_keys() {
        let dir = tempfile::tempdir().unwrap();
        let c = config("git@host:a/docs.git");
        write(dir.path(), &c).unwrap();
        let text = fs::read_to_string(dir.path().join(".quarry/.config")).unwrap();
        assert!(text.find("default_branch").unwrap() < text.find("docs_dir").unwrap());
        assert!(text.find("permalink_template").unwrap() < text.find("url").unwrap());
        assert_eq!(load(dir.path()).unwrap(), Some(c));
    }
}
