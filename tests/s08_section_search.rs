//! S8: section and search matching.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s8_an_exact_heading_returns_one_body() {
    let it = wired();
    let out =
        it.w.run(&["docs", "section", "record-store", "file-ingest (v2)"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("09-interfaces.md § file-ingest (v2)"),
        "{text}"
    );
    assert!(text.contains("| file_id | string | yes |"), "{text}");
    assert!(text.contains("2026-09-03"), "{text}");
    assert!(
        !text.contains("## Consumes"),
        "the body leaked its parent: {text}"
    );
}

#[test]
fn s8_matching_is_case_and_whitespace_insensitive() {
    let it = wired();
    let out =
        it.w.run(&["docs", "section", "record-store", "  FILE-INGEST   (V2) "]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("file_id"), "{}", stdout(&out));
}

#[test]
fn s8_a_prefix_matches_when_nothing_is_exact() {
    let it = wired();
    let out =
        it.w.run(&["docs", "section", "record-store", "file-ingest"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("file_id"), "{}", stdout(&out));
}

#[test]
fn s8_an_ambiguous_heading_lists_candidates_and_exits_one() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "01-architecture.md",
            "---\ngenerated_date: 2026-09-04\n---\n\n## Layers\n\na\n",
        ),
        (
            "02-models.md",
            "---\ngenerated_date: 2026-09-04\n---\n\n## Layers\n\nb\n",
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let out = w.run(&["docs", "section", "ingest-api", "Layers"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let text = stderr(&out);
    assert!(text.contains("matches 2 sections"), "{text}");
    assert!(text.contains("01-architecture.md § Layers"), "{text}");
}

#[test]
fn s8_a_missing_heading_offers_the_nearest() {
    let it = wired();
    let out =
        it.w.run(&["docs", "section", "record-store", "file-ingest (v3)"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let text = stderr(&out);
    assert!(text.contains("no section"), "{text}");
    assert!(text.contains("nearest:"), "{text}");
    assert!(text.contains("file-ingest (v2)"), "{text}");
}

#[test]
fn s8_search_finds_the_consumer_by_name() {
    let it = wired();
    let out = it.w.run(&["docs", "search", "file-ingest"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("record-store"), "{text}");
    assert!(text.contains("ingest-api"), "{text}");
    assert!(text.contains("hits"), "{text}");
}

#[test]
fn s8_search_can_be_limited_to_one_repo() {
    let it = wired();
    let out =
        it.w.run(&["docs", "search", "file-ingest", "--repo", "record-store"]);
    let text = stdout(&out);
    assert!(text.contains("record-store"), "{text}");
    assert!(!text.contains("ingest-api"), "{text}");
}

#[test]
fn s8_search_with_no_hits_is_exit_zero() {
    let it = wired();
    let out = it.w.run(&["docs", "search", "nothing-matches-this"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("0 hits"), "{}", stdout(&out));
}

#[test]
fn s8_fts_syntax_in_the_term_is_not_interpreted() {
    let it = wired();
    let out = it.w.run(&["docs", "search", "file-ingest OR (broken"]);
    assert_eq!(code(&out), 0, "{}\n{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("0 hits"), "{}", stdout(&out));
}

#[test]
fn s8_a_limit_caps_the_rows() {
    let it = wired();
    let out = it.w.run(&["docs", "search", "file-ingest", "--limit", "1"]);
    assert!(stdout(&out).contains("1 hits"), "{}", stdout(&out));
}
