//! S6: the edge contract.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s6_show_lists_both_directions() {
    let it = wired();
    let out = it.w.run(&["docs", "show", "record-store"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("consumes: sqs file-ingest <- ingest-api"),
        "{text}"
    );
    assert!(
        text.contains("produces: http GET /records -> report-builder"),
        "{text}"
    );
    assert!(text.contains("# Overview"), "{text}");
}

#[test]
fn s6_a_repo_without_edges_says_none() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let out = w.run(&["docs", "show", "ingest-api"]);
    assert!(stdout(&out).contains("produces: none"), "{}", stdout(&out));
    assert!(stdout(&out).contains("consumes: none"), "{}", stdout(&out));
}

#[test]
fn s6_an_edge_both_sides_declare_is_one_edge() {
    let it = wired();
    let out =
        it.w.run(&["--json", "docs", "deps", "ingest-api", "--downstream"]);
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "{}", stdout(&out));
    assert_eq!(edges[0]["declared_by"], "both");
    assert_eq!(edges[0]["missing"], false);
    let via = edges[0]["via"].as_array().expect("via");
    assert_eq!(via.len(), 2, "{via:?}");
}

#[test]
fn s6_a_one_sided_edge_names_its_side() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "09-interfaces.md",
            &produces_page("2026-09-04", "ghost", "sqs", "file-ingest"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let out = w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    let text = stdout(&out);
    assert!(text.contains("(not in quarry)"), "{text}");
    assert!(text.contains("declared by producer only"), "{text}");
}

#[test]
fn s6_an_entry_missing_a_required_field_is_a_warning_not_a_failure() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "09-interfaces.md",
            "---\ngenerated_date: 2026-09-04\nproduces:\n  - kind: sqs\n    name: orphan\n---\n\n## Produces\n",
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let out = w.run(&["docs", "index", "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("missing 'to'"), "{}", stdout(&out));
    assert!(stdout(&out).contains("0 edges"), "{}", stdout(&out));
}

#[test]
fn s6_unparsable_frontmatter_is_a_warning_and_the_page_still_indexes() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "02-models.md",
            "---\nbroken: [1,\n---\n\n## Entities\n\nrows\n",
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let out = w.run(&["docs", "index", "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("not valid YAML"), "{}", stdout(&out));
    assert!(
        stdout(&it_list(&w)).contains("02-models.md"),
        "listing lost the page"
    );
}

fn it_list(w: &World) -> std::process::Output {
    w.run(&["docs", "list", "ingest-api"])
}
