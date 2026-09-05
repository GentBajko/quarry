//! The inputs every command shares.

use std::path::PathBuf;

use crate::config::{self, Config};
use crate::errors::{QuarryError, Result};
use crate::gitcmd::{self, Git};
use crate::identity::{self, RepoIdentity};

#[derive(Debug, Clone, Default)]
pub(crate) struct ContextArgs {
    pub(crate) verbose: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Context {
    pub(crate) repo_root: PathBuf,
    pub(crate) config: Option<Config>,
    pub(crate) identity: Option<RepoIdentity>,
    pub(crate) verbose: bool,
}

impl Context {
    pub(crate) fn build(args: &ContextArgs) -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let repo_root = gitcmd::repo_root(&cwd, args.verbose)?;
        let config = config::load(&repo_root)?;
        let git = Git::new(&repo_root, args.verbose);
        let identity = match gitcmd::origin_url(&git) {
            Ok(url) => identity::from_origin_url(&url).ok(),
            Err(_) => None,
        };
        Ok(Self {
            repo_root,
            config,
            identity,
            verbose: args.verbose,
        })
    }

    pub(crate) fn config(&self) -> Result<&Config> {
        self.config
            .as_ref()
            .ok_or_else(|| QuarryError::refusal("not initialised here; run quarry init"))
    }

    pub(crate) fn identity(&self) -> Result<&RepoIdentity> {
        self.identity.as_ref().ok_or_else(|| {
            QuarryError::refusal("no origin remote; quarry derives the repo name from it")
        })
    }

    pub(crate) fn quarry_dir(&self) -> PathBuf {
        config::quarry_dir(&self.repo_root)
    }

    pub(crate) fn clone_path(&self) -> Result<PathBuf> {
        let name = identity::clone_name(&self.config()?.url)?;
        Ok(self.quarry_dir().join(name))
    }

    pub(crate) fn index_path(&self) -> PathBuf {
        self.quarry_dir().join(".docs-index.sqlite")
    }

    pub(crate) fn repo_git(&self) -> Git {
        Git::new(&self.repo_root, self.verbose)
    }

    pub(crate) fn clone_git(&self) -> Result<Git> {
        Ok(Git::new(self.clone_path()?, self.verbose))
    }

    pub(crate) fn require_clone(&self) -> Result<PathBuf> {
        let path = self.clone_path()?;
        if !path.join(".git").exists() {
            return Err(QuarryError::refusal(
                "docs repo clone missing; run quarry init",
            ));
        }
        Ok(path)
    }
}
