//! The dependency direction the architecture chapter fixes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

fn sources() -> BTreeMap<String, String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir).expect("src") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            out.insert(name, std::fs::read_to_string(&path).expect("read"));
        }
    }
    out
}

#[test]
fn nothing_below_the_cli_uses_the_cli_or_the_commands() {
    for (name, text) in sources() {
        if name == "main" || name == "cli" {
            continue;
        }
        assert!(!text.contains("crate::cli"), "{name} uses crate::cli");
        assert!(
            !text.contains("crate::commands"),
            "{name} uses crate::commands"
        );
    }
}

#[test]
fn only_gitcmd_spawns_a_process() {
    for (name, text) in sources() {
        if name == "gitcmd" {
            continue;
        }
        assert!(
            !text.contains("process::Command"),
            "{name} spawns a process; every git call belongs in gitcmd"
        );
    }
}

#[test]
fn only_frontmatter_parses_yaml() {
    for (name, text) in sources() {
        if name == "frontmatter" {
            continue;
        }
        assert!(
            !text.contains("serde_saphyr"),
            "{name} parses YAML outside the frontmatter boundary"
        );
    }
}

#[test]
fn only_the_index_talks_to_sqlite() {
    for (name, text) in sources() {
        if matches!(name.as_str(), "index" | "query" | "errors") {
            continue;
        }
        assert!(
            !text.contains("rusqlite"),
            "{name} talks to SQLite directly"
        );
    }
}
