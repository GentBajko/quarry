//! Every subprocess call quarry makes. Argument vectors, never a shell.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::errors::{QuarryError, Result};

#[derive(Debug, Clone)]
pub(crate) struct Git {
    pub(crate) cwd: PathBuf,
    pub(crate) verbose: bool,
}

impl Git {
    pub(crate) fn new(cwd: impl Into<PathBuf>, verbose: bool) -> Self {
        Self {
            cwd: cwd.into(),
            verbose,
        }
    }

    pub(crate) fn run(&self, args: &[&str]) -> Result<Output> {
        let out = self.run_unchecked(args)?;
        if !out.status.success() {
            return Err(QuarryError::external(format!(
                "git {} failed: {}",
                args.first().copied().unwrap_or(""),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(out)
    }

    pub(crate) fn run_unchecked(&self, args: &[&str]) -> Result<Output> {
        if self.verbose {
            eprintln!("+ git {}", args.join(" "));
        }
        Command::new("git")
            .args(args)
            .current_dir(&self.cwd)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    QuarryError::refusal("git not found on PATH; quarry drives the git binary")
                } else {
                    QuarryError::external(format!("running git failed: {e}"))
                }
            })
    }

    pub(crate) fn stdout(&self, args: &[&str]) -> Result<String> {
        let out = self.run(args)?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    pub(crate) fn stdout_lines(&self, args: &[&str]) -> Result<Vec<String>> {
        Ok(self
            .stdout(args)?
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect())
    }

    pub(crate) fn rev_parse(&self, rev: &str) -> Result<String> {
        self.stdout(&["rev-parse", rev])
    }

    pub(crate) fn is_ancestor(&self, older: &str, newer: &str) -> Result<bool> {
        let out = self.run_unchecked(&["merge-base", "--is-ancestor", older, newer])?;
        match out.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(QuarryError::external(format!(
                "git merge-base failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
        }
    }

    pub(crate) fn rev_exists(&self, sha: &str) -> Result<bool> {
        let out = self.run_unchecked(&["cat-file", "-e", &format!("{sha}^{{commit}}")])?;
        Ok(out.status.success())
    }
}

pub(crate) fn repo_root(cwd: &Path, verbose: bool) -> Result<PathBuf> {
    let git = Git::new(cwd, verbose);
    let out = git.run_unchecked(&["rev-parse", "--show-toplevel"])?;
    if !out.status.success() {
        return Err(QuarryError::refusal(
            "not a git repository; run quarry in a repo's root",
        ));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

pub(crate) fn origin_url(git: &Git) -> Result<String> {
    let out = git.run_unchecked(&["remote", "get-url", "origin"])?;
    if !out.status.success() {
        return Err(QuarryError::refusal(
            "no origin remote; quarry derives the repo name from it",
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub(crate) fn default_branch(git: &Git) -> String {
    if let Ok(out) = git.run_unchecked(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if let Some(branch) = text.strip_prefix("origin/")
            && !branch.is_empty()
        {
            return branch.to_string();
        }
    }
    "main".to_string()
}
