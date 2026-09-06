//! S21: four repos' committed pages, driven end to end.
//!
//! Every other suite builds its pages from strings in the test file, so
//! nothing checks that the shape Capstone prescribes is the shape quarry
//! parses. These fixtures are that shape, written once under
//! `tests/fixtures/estate/` and read by both tools.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::{Path, PathBuf};

use common::*;

const REPOS: [&str; 4] = [
    "identity-api",
    "ingest-api",
    "record-store",
    "report-builder",
];

pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("estate")
}

fn copy_docs(from: &Path, into: &Path) {
    let dest = into.join("docs/capstone");
    std::fs::create_dir_all(&dest).expect("docs dir");
    for entry in std::fs::read_dir(from.join("docs/capstone")).expect("fixture docs") {
        let page = entry.expect("entry").path();
        let name = page.file_name().expect("name");
        std::fs::copy(&page, dest.join(name)).expect("copy page");
    }
}

/// The four repos, each imported at its own commit, and the index rebuilt.
struct Estate {
    w: World,
    repos: Vec<PathBuf>,
}

impl Estate {
    fn repo(&self, name: &str) -> &Path {
        let index = REPOS.iter().position(|r| *r == name).expect("known repo");
        &self.repos[index]
    }

    fn run_in_repo(&self, name: &str, args: &[&str]) -> std::process::Output {
        self.w.run_in(self.repo(name), args)
    }
}

fn estate() -> Estate {
    let w = world();
    let base = w.base();
    let root = fixture_root();
    let mut repos = Vec::new();
    for name in REPOS {
        // `world()` already made ingest-api; the rest need their own origin.
        let path = if name == "ingest-api" {
            w.source.clone()
        } else {
            common::new_source(&base, name)
        };
        let init = w.run_in(&path, &["init", "--url", &w.docs_url]);
        assert_eq!(code(&init), 0, "{}", stderr(&init));
        // The sites the pages cite have to be in the tree, or the import
        // reports every one of them as unverifiable.
        for file in [
            "routes.rs",
            "publish.rs",
            "auth.rs",
            "consumer.rs",
            "events.rs",
            "client.rs",
        ] {
            w.write_file(&path, &format!("src/{file}"), "// fixture\n");
        }
        copy_docs(&root.join(name), &path);
        w.commit_push_in(&path, "docs");
        let added = w.run_in(&path, &["add"]);
        assert_eq!(code(&added), 0, "{}", stderr(&added));
        repos.push(path);
    }
    // Each repo holds its own clone, and every one of them imported before the
    // repos after it in this loop had pushed.
    for path in &repos {
        assert!(w.run_in(path, &["sync"]).status.success());
    }
    Estate { w, repos }
}

#[test]
fn s21_the_estate_imports_with_no_warnings() {
    let it = estate();
    let out = it.run_in_repo("ingest-api", &["docs", "index", "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("4 repos"), "{text}");
    assert!(
        !text.contains("warning:"),
        "the prescribed shape should parse cleanly: {text}"
    );
    assert!(
        !text.contains("unresolved:"),
        "every far end should resolve: {text}"
    );
}

#[test]
fn s21_a_queue_reaches_both_of_its_consumers() {
    let it = estate();
    let out = it.run_in_repo(
        "ingest-api",
        &["docs", "deps", "ingest-api", "--downstream"],
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("sqs file-ingest -> record-store (resolved by name)"),
        "{text}"
    );
    assert!(
        text.contains("sqs file-ingest -> report-builder (resolved by name)"),
        "{text}"
    );
    assert!(text.contains("2 repos"), "{text}");
}

#[test]
fn s21_an_alias_and_a_route_spelling_still_meet() {
    let it = estate();
    // The consumer names the producer by its deploy name and writes the path
    // parameter as `:id`; the producer's page says `identity-api` and `{id}`.
    let out = it.run_in_repo(
        "ingest-api",
        &["docs", "deps", "identity-api", "--downstream"],
    );
    let text = stdout(&out);
    assert!(text.contains("-> ingest-api"), "{text}");
    assert!(text.contains("declared as identity.internal"), "{text}");
}

#[test]
fn s21_the_http_contract_joins_its_consumer() {
    let it = estate();
    let out = it.run_in_repo(
        "record-store",
        &["docs", "deps", "record-store", "--downstream"],
    );
    let text = stdout(&out);
    assert!(
        text.contains("http GET /records -> report-builder (resolved by name)"),
        "{text}"
    );
}

#[test]
fn s21_the_chain_runs_from_the_publisher_to_the_report() {
    let it = estate();
    let out = it.run_in_repo(
        "ingest-api",
        &["docs", "path", "ingest-api", "report-builder"],
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("ingest-api"), "{}", stdout(&out));
}

#[test]
fn s21_check_is_clean_across_the_estate() {
    let it = estate();
    for name in REPOS {
        let out = it.run_in_repo(name, &["check"]);
        assert_eq!(code(&out), 0, "{name}: {}", stdout(&out));
        assert!(
            !stdout(&out).contains("break:"),
            "{name} should be clean: {}",
            stdout(&out)
        );
    }
}

#[test]
fn s21_dropping_a_field_two_consumers_read_is_a_break() {
    let it = estate();
    let page = it.repo("ingest-api").join("docs/capstone/02-models.md");
    let models = std::fs::read_to_string(&page).expect("models");
    std::fs::write(&page, models.replace("| content_type | enum | yes |\n", ""))
        .expect("write models");
    let out = it.run_in_repo("ingest-api", &["check"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let text = stdout(&out);
    assert!(
        text.contains("break: content_type no longer produced"),
        "{text}"
    );
    assert!(text.contains("record-store reads"), "{text}");
}

#[test]
fn s21_a_consumer_reading_a_subset_is_not_a_break() {
    let it = estate();
    // report-builder's IngestEvent lists file_id alone; the producer publishes
    // more than that, and a field nobody reads is nobody's problem.
    let out = it.run_in_repo("ingest-api", &["check"]);
    let text = stdout(&out);
    assert!(text.contains("report-builder reads file_id"), "{text}");
    assert!(!text.contains("break:"), "{text}");
}
