//! `.quarry/.config` and `.quarry/.gitignore`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{QuarryError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Target {
    pub(crate) name: String,
    pub(crate) docs_dir: String,
}

// `targets` sits between `permalink_template` and `url` so the pretty-printed
// file stays in alphabetical key order, and it disappears when empty so a
// single-repo config round-trips byte for byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) default_branch: String,
    pub(crate) docs_dir: String,
    #[serde(default)]
    pub(crate) permalink_template: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) targets: Vec<Target>,
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
    name_flag: Option<String>,
    branch_flag: Option<String>,
    existing: Option<&Config>,
    derived_branch: String,
) -> Result<Config> {
    let url = url_flag
        .or_else(|| existing.map(|c| c.url.clone()))
        .ok_or_else(|| {
            QuarryError::refusal(
                "no docs repo URL: pass --url or set QUARRY_DOCS_REPO (no terminal to ask on)",
            )
        })?;
    let mut targets = existing.map(|c| c.targets.clone()).unwrap_or_default();
    let docs_dir = match name_flag {
        Some(name) => {
            let Some(dir) = docs_dir_flag else {
                return Err(QuarryError::refusal(format!(
                    "--name {name} needs --docs-dir"
                )));
            };
            // Padding and a trailing slash would be stored, interpolated into
            // `git ls-tree <sha>:<dir>` and compared by `tidy_dir`, so the
            // stored form is tidied before validation.
            let dir = tidy_dir(&dir).to_string();
            validate_target_name(&name)?;
            validate_docs_dir(&dir)?;
            match targets.iter_mut().find(|t| t.name == name) {
                Some(existing) => existing.docs_dir = dir,
                None => targets.push(Target {
                    name,
                    docs_dir: dir,
                }),
            }
            existing
                .map(|c| c.docs_dir.clone())
                .unwrap_or_else(|| DEFAULT_DOCS_DIR.to_string())
        }
        None => docs_dir_flag
            .or_else(|| existing.map(|c| c.docs_dir.clone()))
            .unwrap_or_else(|| DEFAULT_DOCS_DIR.to_string()),
    };
    let docs_dir = tidy_dir(&docs_dir).to_string();
    validate_docs_dir(&docs_dir)?;
    targets.sort_by(|a, b| a.name.cmp(&b.name));
    validate_targets(&docs_dir, &targets)?;
    let default_branch = branch_flag
        .or_else(|| existing.map(|c| c.default_branch.clone()))
        .unwrap_or(derived_branch);
    Ok(Config {
        default_branch,
        docs_dir,
        permalink_template: existing.and_then(|c| c.permalink_template.clone()),
        targets,
        url,
    })
}

fn validate_docs_dir(docs_dir: &str) -> Result<()> {
    let path = Path::new(docs_dir);
    // An empty dir is the repository root: `git ls-tree -r <sha>:` would import
    // the whole tree as one folder, so a hand-edited config is refused here.
    if docs_dir.trim().is_empty() || path.is_absolute() || docs_dir.split('/').any(|c| c == "..") {
        return Err(QuarryError::refusal(format!(
            "docs dir must be a relative path inside the repo: {docs_dir}"
        )));
    }
    Ok(())
}

// The files the docs repo root holds beside the repo folders. A target folder
// with one of these names would collide with them: `regenerate_root_index`
// writes `00-index.md` and would fail with a bare OS error over a directory.
const DOCS_ROOT_FILES: [&str; 2] = ["00-index.md", crate::observed::OBSERVED_FILE];

// A target name becomes a folder in the docs repo, so `..` or a separator would
// let a hand-edited config write outside it. Backslash is refused too: a
// Windows-authored name would otherwise mean something else on Linux.
fn validate_target_name(name: &str) -> Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err(QuarryError::refusal(format!(
            "target name must be one path segment: {name}"
        )));
    }
    if DOCS_ROOT_FILES.contains(&name) {
        return Err(QuarryError::refusal(format!(
            "target name {name} is taken by the docs repo root"
        )));
    }
    Ok(())
}

fn validate_target(target: &Target) -> Result<()> {
    validate_target_name(&target.name)?;
    validate_docs_dir(&target.docs_dir)
}

pub(crate) fn validate_targets(root_docs_dir: &str, targets: &[Target]) -> Result<()> {
    let root = tidy_dir(root_docs_dir);
    for (i, target) in targets.iter().enumerate() {
        validate_target(target)?;
        let dir = tidy_dir(&target.docs_dir);
        if dir == root {
            return Err(QuarryError::refusal(format!(
                "target {} docs dir equals the root docs dir",
                target.name
            )));
        }
        for other in &targets[..i] {
            // Two targets under one name import two docs dirs into one folder,
            // last write wins, and the next run refuses on a stamp naming a dir
            // nobody asked for.
            if other.name == target.name {
                return Err(QuarryError::refusal(format!(
                    "targets {} is listed twice",
                    target.name
                )));
            }
            let other_dir = tidy_dir(&other.docs_dir);
            if other_dir == dir {
                return Err(QuarryError::refusal(format!(
                    "targets {} and {} share docs dir {dir}",
                    other.name, target.name
                )));
            }
            // A target inside another target's docs dir would be imported twice,
            // once under its own folder and once under the outer target's, since
            // only the umbrella unit prunes nested pages. Refuse the layout
            // rather than teach every unit about its siblings.
            if let Some((outer, inner)) = nested_pair((&other.name, other_dir), (&target.name, dir))
            {
                return Err(QuarryError::refusal(format!(
                    "target {} docs dir {} is inside target {}'s docs dir {}",
                    inner.0, inner.1, outer.0, outer.1
                )));
            }
        }
    }
    Ok(())
}

// Each argument is a target's (name, tidied docs dir); the answer is
// (outer, inner) when one docs dir contains the other.
type Named<'a> = (&'a str, &'a str);

fn nested_pair<'a>(a: Named<'a>, b: Named<'a>) -> Option<(Named<'a>, Named<'a>)> {
    if b.1.starts_with(&format!("{}/", a.1)) {
        Some((a, b))
    } else if a.1.starts_with(&format!("{}/", b.1)) {
        Some((b, a))
    } else {
        None
    }
}

// Comparisons of two docs dirs go through here, so a hand-edited config that
// pads or decorates a path still matches the stamp it wrote.
pub(crate) fn tidy_dir(dir: &str) -> &str {
    let trimmed = dir.trim().trim_end_matches('/');
    trimmed.strip_prefix("./").unwrap_or(trimmed)
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
            targets: Vec::new(),
            url: url.into(),
        }
    }

    fn with_targets(targets: Vec<Target>) -> Config {
        Config {
            targets,
            ..config("u")
        }
    }

    fn target(name: &str, docs_dir: &str) -> Target {
        Target {
            name: name.into(),
            docs_dir: docs_dir.into(),
        }
    }

    #[test]
    fn s12_flag_beats_existing_config() {
        let existing = config("git@host:a/old.git");
        let c = resolve(
            Some("git@host:a/new.git".into()),
            None,
            None,
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
        assert!(resolve(None, None, None, None, None, "main".into()).is_err());
    }

    #[test]
    fn s12_docs_dir_escaping_the_repo_refuses() {
        assert!(
            resolve(
                Some("u".into()),
                Some("../elsewhere".into()),
                None,
                None,
                None,
                "main".into()
            )
            .is_err()
        );
        assert!(
            resolve(
                Some("u".into()),
                Some("/etc".into()),
                None,
                None,
                None,
                "main".into()
            )
            .is_err()
        );
    }

    #[test]
    fn s12_an_explicit_branch_beats_the_derived_one() {
        let c = resolve(
            Some("u".into()),
            None,
            None,
            Some("trunk".into()),
            None,
            "main".into(),
        )
        .unwrap();
        assert_eq!(c.default_branch, "trunk");
    }

    #[test]
    fn s12_a_stored_branch_survives_a_reinit() {
        let existing = Config {
            default_branch: "master".into(),
            docs_dir: "docs/capstone".into(),
            permalink_template: None,
            targets: Vec::new(),
            url: "u".into(),
        };
        let c = resolve(None, None, None, None, Some(&existing), "main".into()).unwrap();
        assert_eq!(c.default_branch, "master");
    }

    #[test]
    fn s12_config_round_trips_with_sorted_keys() {
        let dir = tempfile::tempdir().unwrap();
        let c = config("git@host:a/docs.git");
        write(dir.path(), &c).unwrap();
        let text = fs::read_to_string(dir.path().join(".quarry/.config")).unwrap();
        assert!(text.find("default_branch").unwrap() < text.find("docs_dir").unwrap());
        assert!(text.find("permalink_template").unwrap() < text.find("url").unwrap());
        assert!(!text.contains("targets"), "{text}");
        assert_eq!(load(dir.path()).unwrap(), Some(c));
    }

    #[test]
    fn s17_name_without_docs_dir_refuses() {
        let err = resolve(
            Some("u".into()),
            None,
            Some("billing".into()),
            None,
            None,
            "main".into(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("--docs-dir"), "{err}");
    }

    #[test]
    fn s17_name_with_docs_dir_appends_a_target() {
        let c = resolve(
            Some("u".into()),
            Some("services/billing/docs/capstone".into()),
            Some("billing".into()),
            None,
            None,
            "main".into(),
        )
        .unwrap();
        assert_eq!(c.docs_dir, "docs/capstone");
        assert_eq!(
            c.targets,
            vec![target("billing", "services/billing/docs/capstone")]
        );
    }

    #[test]
    fn s17_a_repeated_name_updates_the_target_in_place() {
        let existing = with_targets(vec![target("billing", "a/docs")]);
        let c = resolve(
            None,
            Some("b/docs".into()),
            Some("billing".into()),
            None,
            Some(&existing),
            "main".into(),
        )
        .unwrap();
        assert_eq!(c.targets, vec![target("billing", "b/docs")]);
    }

    #[test]
    fn s17_targets_come_out_sorted_by_name() {
        let existing = with_targets(vec![target("orders", "services/orders/docs")]);
        let c = resolve(
            None,
            Some("services/billing/docs".into()),
            Some("billing".into()),
            None,
            Some(&existing),
            "main".into(),
        )
        .unwrap();
        let names: Vec<&str> = c.targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["billing", "orders"]);
    }

    #[test]
    fn s17_target_name_must_be_one_segment() {
        for name in ["a/b", ".", "..", "", "a\\b"] {
            assert!(
                resolve(
                    Some("u".into()),
                    Some("services/x/docs".into()),
                    Some(name.into()),
                    None,
                    None,
                    "main".into()
                )
                .is_err(),
                "{name} was accepted"
            );
        }
    }

    #[test]
    fn s17_a_target_named_after_a_docs_root_file_refuses() {
        for name in DOCS_ROOT_FILES {
            let err = resolve(
                Some("u".into()),
                Some("services/x/docs/capstone".into()),
                Some(name.into()),
                None,
                None,
                "main".into(),
            )
            .unwrap_err();
            assert!(
                err.to_string().contains(&format!(
                    "target name {name} is taken by the docs repo root"
                )),
                "{err}"
            );
        }
    }

    #[test]
    fn s17_a_padded_docs_dir_is_trimmed_before_it_is_stored() {
        let c = resolve(
            Some("u".into()),
            Some(" services/billing/docs/capstone ".into()),
            Some("billing".into()),
            None,
            None,
            "main".into(),
        )
        .unwrap();
        assert_eq!(
            c.targets,
            vec![target("billing", "services/billing/docs/capstone")]
        );
        let root = resolve(
            Some("u".into()),
            Some("  documentation\t".into()),
            None,
            None,
            None,
            "main".into(),
        )
        .unwrap();
        assert_eq!(root.docs_dir, "documentation");
        assert_eq!(tidy_dir(" docs/capstone/ "), "docs/capstone");
        let slashed = resolve(
            Some("u".into()),
            Some("services/billing/docs/capstone/".into()),
            Some("billing".into()),
            None,
            None,
            "main".into(),
        )
        .unwrap();
        assert_eq!(
            slashed.targets,
            vec![target("billing", "services/billing/docs/capstone")]
        );
        let slashed_root = resolve(
            Some("u".into()),
            Some("./documentation/".into()),
            None,
            None,
            None,
            "main".into(),
        )
        .unwrap();
        assert_eq!(slashed_root.docs_dir, "documentation");
    }

    #[test]
    fn s17_two_targets_under_one_name_are_refused() {
        let err = validate_targets(
            "docs/capstone",
            &[
                target("billing", "services/billing/docs/capstone"),
                target("billing", "services/ingest/docs/capstone"),
            ],
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("targets billing is listed twice"),
            "{err}"
        );
    }

    #[test]
    fn s17_target_docs_dir_is_validated_like_the_root() {
        for dir in ["../out", "/abs", "", "  "] {
            assert!(
                resolve(
                    Some("u".into()),
                    Some(dir.into()),
                    Some("x".into()),
                    None,
                    None,
                    "main".into()
                )
                .is_err(),
                "{dir} was accepted"
            );
        }
    }

    #[test]
    fn s17_an_empty_root_docs_dir_refuses() {
        let err = resolve(
            Some("u".into()),
            Some("".into()),
            None,
            None,
            None,
            "main".into(),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("docs dir must be a relative path inside the repo"),
            "{err}"
        );
    }

    #[test]
    fn s17_a_target_inside_another_target_refuses() {
        let targets = vec![
            Target {
                name: "a".into(),
                docs_dir: "docs/capstone/a".into(),
            },
            Target {
                name: "b".into(),
                docs_dir: "docs/capstone/a/b/".into(),
            },
        ];
        let err = validate_targets("docs/capstone", &targets).unwrap_err();
        assert!(
            err.to_string()
                .contains("target b docs dir docs/capstone/a/b is inside target a's docs dir"),
            "{err}"
        );
        let flipped = vec![targets[1].clone(), targets[0].clone()];
        assert!(
            validate_targets("docs/capstone", &flipped)
                .unwrap_err()
                .to_string()
                .contains("is inside target a's docs dir"),
            "the outer target must be named whichever order it comes in"
        );
    }

    #[test]
    fn s17_a_target_docs_dir_equal_to_the_root_refuses() {
        let err = resolve(
            Some("u".into()),
            Some("docs/capstone".into()),
            Some("x".into()),
            None,
            None,
            "main".into(),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("equals the root docs dir"),
            "{err}"
        );
    }

    #[test]
    fn s17_two_targets_sharing_a_docs_dir_refuse() {
        let existing = with_targets(vec![target("billing", "d")]);
        let err = resolve(
            None,
            Some("d/".into()),
            Some("orders".into()),
            None,
            Some(&existing),
            "main".into(),
        )
        .unwrap_err();
        assert!(
            err.to_string() == "targets billing and orders share docs dir d",
            "{err}"
        );
    }

    #[test]
    fn s17_docs_dir_without_name_still_sets_the_root() {
        let c = resolve(
            Some("u".into()),
            Some("documentation".into()),
            None,
            None,
            None,
            "main".into(),
        )
        .unwrap();
        assert_eq!(c.docs_dir, "documentation");
        assert!(c.targets.is_empty());
    }

    #[test]
    fn s17_a_config_without_targets_round_trips_byte_identically() {
        let dir = tempfile::tempdir().unwrap();
        let plain = config("git@host:a/docs.git");
        write(dir.path(), &plain).unwrap();
        let text = fs::read_to_string(dir.path().join(".quarry/.config")).unwrap();
        assert!(!text.contains("targets"), "{text}");
        assert_eq!(load(dir.path()).unwrap(), Some(plain));

        let mono = with_targets(vec![target("billing", "services/billing/docs/capstone")]);
        write(dir.path(), &mono).unwrap();
        let text = fs::read_to_string(dir.path().join(".quarry/.config")).unwrap();
        assert!(text.contains("\"targets\": ["), "{text}");
        assert!(
            text.find("permalink_template").unwrap() < text.find("\"targets\"").unwrap(),
            "{text}"
        );
        assert!(
            text.find("\"targets\"").unwrap() < text.find("\"url\"").unwrap(),
            "{text}"
        );
        assert_eq!(load(dir.path()).unwrap(), Some(mono));
    }

    #[test]
    fn s17_targets_survive_a_reinit_without_name() {
        let existing = with_targets(vec![target("billing", "services/billing/docs")]);
        let c = resolve(
            Some("u2".into()),
            None,
            None,
            None,
            Some(&existing),
            "main".into(),
        )
        .unwrap();
        assert_eq!(c.targets, existing.targets);
    }

    #[test]
    fn s17_tidy_dir_folds_a_trailing_slash_and_a_leading_dot() {
        assert_eq!(tidy_dir("docs/capstone/"), "docs/capstone");
        assert_eq!(tidy_dir("./docs/capstone"), "docs/capstone");
        assert_eq!(tidy_dir("docs/capstone"), "docs/capstone");
    }
}
