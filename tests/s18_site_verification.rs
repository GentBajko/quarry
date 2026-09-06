//! S18: site verification.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use serde_json::Value;

const GOOD: &str = "src/publish.py:1";
const GONE: &str = "src/gone.py:12";

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

fn head(w: &World) -> String {
    stdout(&w.git(&w.source, &["rev-parse", "HEAD"]))
        .trim()
        .to_string()
}

/// ingest-api with `src/publish.py` tracked and one produces row whose site is `site`.
fn with_site(site: &str, in_table: bool) -> World {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_file(&w.source, "src/publish.py", "print(1)\n");
    let page = if in_table {
        produces_table_page("2026-09-04", "record-store", "sqs", "file-ingest", site)
    } else {
        produces_page_with_site("2026-09-04", "record-store", "sqs", "file-ingest", site)
    };
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        ("09-interfaces.md", &page),
    ]);
    w.commit_push("docs and code");
    w
}

#[test]
fn s18_a_tracked_site_leaves_the_stamp_unchanged() {
    let w = with_site(GOOD, true);
    let head = head(&w);
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        !stdout(&out).contains("not in the tree"),
        "{}",
        stdout(&out)
    );
    assert_eq!(
        w.remote_stamp("ingest-api"),
        format!(
            "{{\"commit\":\"{head}\",\"origin\":\"localhost/remotes/ingest-api\",\"docs_dir\":\"docs/capstone\"}}\n"
        )
    );
}

#[test]
fn s18_an_untracked_site_is_a_note_and_a_stamp_entry() {
    let w = with_site(GONE, true);
    let short: String = head(&w).chars().take(7).collect();
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains(&format!(
            "site src/gone.py:12 for sqs file-ingest is not in the tree at {short}"
        )),
        "{}",
        stdout(&out)
    );
    let stamp = w.remote_stamp("ingest-api");
    assert!(
        stamp.contains("\"unverified\":[\"sqs file-ingest\"]"),
        "{stamp}"
    );
    assert!(
        w.remote_files()
            .contains(&"ingest-api/09-interfaces.md".to_string()),
        "the import did not land"
    );
}

#[test]
fn s18_json_notes_carry_the_unverified_site() {
    let w = with_site(GONE, true);
    let out = w.run(&["--json", "add"]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    let value = json(&out);
    assert_eq!(value["result"]["result"], Value::from("imported"));
    let notes = value["result"]["notes"].as_array().expect("notes");
    assert_eq!(notes.len(), 1, "{value}");
    assert!(
        notes[0]
            .as_str()
            .unwrap_or_default()
            .starts_with("site src/gone.py:12 for sqs file-ingest"),
        "{value}"
    );
}

#[test]
fn s18_a_range_site_is_verified_by_its_path() {
    let w = with_site("src/publish.py:10-20", true);
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        !stdout(&out).contains("not in the tree"),
        "{}",
        stdout(&out)
    );
    let stamp = w.remote_stamp("ingest-api");
    assert!(!stamp.contains("unverified"), "{stamp}");
}

#[test]
fn s18_a_frontmatter_site_is_verified_too() {
    let w = with_site(GONE, false);
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("site src/gone.py:12 for sqs file-ingest is not in the tree at"),
        "{}",
        stdout(&out)
    );
    let stamp = w.remote_stamp("ingest-api");
    assert!(
        stamp.contains("\"unverified\":[\"sqs file-ingest\"]"),
        "{stamp}"
    );
}

#[test]
fn s18_deps_marks_both_ends_of_the_edge() {
    let w = with_site(GONE, true);
    assert!(w.run(&["add"]).status.success());
    let data = w.other_repo("record-store");
    w.write_docs_in(
        &data,
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &consumes_page("2026-09-03", "ingest-api", "sqs", "file-ingest"),
            ),
        ],
    );
    w.commit_push_in(&data, "docs");
    assert!(w.run_in(&data, &["add"]).status.success());
    assert!(w.run(&["sync"]).status.success());

    let down = w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert!(
        stdout(&down).contains("sqs file-ingest -> record-store (site unverified)"),
        "{}",
        stdout(&down)
    );
    let up = w.run(&["docs", "deps", "record-store", "--upstream"]);
    assert!(
        stdout(&up).contains("sqs file-ingest <- ingest-api (site unverified)"),
        "{}",
        stdout(&up)
    );
    let value = json(&w.run(&["--json", "docs", "deps", "ingest-api", "--downstream"]));
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges[0]["site_unverified"], Value::Bool(true), "{value}");
    assert_eq!(edges[0]["declared_by"], Value::from("both"), "{value}");
}

#[test]
fn s18_a_verified_edge_carries_no_mark() {
    let it = wired();
    let value = json(
        &it.w
            .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]),
    );
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges[0]["site_unverified"], Value::Bool(false), "{value}");
    let human = it.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert!(
        !stdout(&human).contains("(site unverified)"),
        "{}",
        stdout(&human)
    );
    let show = json(&it.w.run(&["--json", "docs", "show", "record-store"]));
    assert_eq!(
        show["result"]["produces"][0]["site_unverified"],
        Value::Bool(false),
        "{show}"
    );
}

#[test]
fn s18_strict_add_refuses_before_writing() {
    let w = with_site(GONE, true);
    let out = w.run(&["add", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("site src/gone.py:12 for sqs file-ingest is not in the tree at"),
        "{err}"
    );
    assert!(
        err.trim()
            .ends_with("fix 09-interfaces.md or run without --strict"),
        "{err}"
    );
    assert!(
        !w.remote_files()
            .iter()
            .any(|f| f.starts_with("ingest-api/")),
        "{:?}",
        w.remote_files()
    );
    let plain = w.run(&["add"]);
    assert_eq!(code(&plain), 0, "{}", stderr(&plain));
}

#[test]
fn s18_strict_update_refuses_and_keeps_the_old_stamp() {
    let w = with_site(GOOD, true);
    assert!(w.run(&["add"]).status.success());
    let first = head(&w);
    w.write_docs(&[(
        "09-interfaces.md",
        &produces_table_page("2026-09-05", "record-store", "sqs", "file-ingest", GONE),
    )]);
    let second = w.commit_push("site moved");
    let refused = w.run(&["update", "--strict"]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(w.remote_stamp("ingest-api").contains(&first));

    let plain = w.run(&["update"]);
    assert_eq!(code(&plain), 0, "{}", stderr(&plain));
    let stamp = w.remote_stamp("ingest-api");
    assert!(stamp.contains(&second), "{stamp}");
    assert!(
        stamp.contains("\"unverified\":[\"sqs file-ingest\"]"),
        "{stamp}"
    );
}

#[test]
fn s18_strict_passes_when_every_site_is_tracked() {
    let w = with_site(GOOD, true);
    let out = w.run(&["add", "--strict"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("files imported"), "{}", stdout(&out));
}

#[test]
fn s18_strict_json_refusal_is_one_document() {
    let w = with_site(GONE, true);
    let out = w.run(&["--json", "add", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    let value = json(&out);
    assert_eq!(value["ok"], Value::Bool(false));
    assert_eq!(value["code"], Value::from(1));
    let error = value["error"].as_str().unwrap_or_default();
    assert!(error.contains("src/gone.py:12"), "{value}");
    assert!(
        error.contains("fix 09-interfaces.md or run without --strict"),
        "{value}"
    );
}

#[test]
fn s18_a_site_that_returns_clears_the_mark() {
    let w = with_site(GONE, true);
    assert!(w.run(&["add"]).status.success());
    let before = w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert!(
        stdout(&before).contains("(site unverified)"),
        "{}",
        stdout(&before)
    );
    w.write_file(&w.source, "src/gone.py", "print(2)\n");
    w.commit_push("the site returns");
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let stamp = w.remote_stamp("ingest-api");
    assert!(!stamp.contains("unverified"), "{stamp}");
    let after = w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert!(
        !stdout(&after).contains("(site unverified)"),
        "{}",
        stdout(&after)
    );
}

#[test]
fn s18_a_second_row_for_the_same_edge_is_one_stamp_key() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "09-interfaces.md",
            "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | To | Site |\n|---|---|---|---|\n| sqs | file-ingest | record-store | `src/gone.py:1` |\n| sqs | file-ingest | record-store | `src/gone.py:2` |\n",
        ),
    ]);
    w.commit_push("docs");
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert_eq!(
        stdout(&out).matches("is not in the tree").count(),
        2,
        "{}",
        stdout(&out)
    );
    let stamp = w.remote_stamp("ingest-api");
    assert_eq!(stamp.matches("sqs file-ingest").count(), 1, "{stamp}");
}

#[test]
fn s18_a_prescriptive_page_is_not_site_checked() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "09-interfaces.md",
            &format!(
                "---\ngenerated_date: 2026-09-04\nmode: prescriptive\n---\n\n## Produces\n\n| Kind | Name | To | Site |\n|---|---|---|---|\n| sqs | file-ingest | record-store | `{GONE}` |\n"
            ),
        ),
    ]);
    w.commit_push("planned docs");
    let out = w.run(&["add", "--strict"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        !stdout(&out).contains("not in the tree"),
        "{}",
        stdout(&out)
    );
    let stamp = w.remote_stamp("ingest-api");
    assert!(!stamp.contains("unverified"), "{stamp}");
}

#[test]
fn s18_a_consumer_side_site_marks_the_edge_from_both_walks() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "09-interfaces.md",
            &produces_page("2026-09-04", "record-store", "sqs", "file-ingest"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());

    let data = w.other_repo("record-store");
    w.write_docs_in(
        &data,
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &consumes_page_with_site(
                    "2026-09-03",
                    "ingest-api",
                    "sqs",
                    "file-ingest",
                    "src/client.py:3",
                ),
            ),
        ],
    );
    w.commit_push_in(&data, "docs");
    let added = w.run_in(&data, &["add"]);
    assert_eq!(code(&added), 0, "{}", stderr(&added));
    assert!(
        stdout(&added).contains("site src/client.py:3 for sqs file-ingest is not in the tree at"),
        "{}",
        stdout(&added)
    );
    assert!(w.run(&["sync"]).status.success());

    let down = w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert!(
        stdout(&down).contains("(site unverified)"),
        "{}",
        stdout(&down)
    );
    let up = w.run(&["docs", "deps", "record-store", "--upstream"]);
    assert!(stdout(&up).contains("(site unverified)"), "{}", stdout(&up));
    let value = json(&w.run(&["--json", "docs", "deps", "ingest-api", "--downstream"]));
    assert_eq!(
        value["result"]["edges"][0]["site_unverified"],
        Value::Bool(true),
        "{value}"
    );
    let stamp = w.remote_stamp("ingest-api");
    assert!(!stamp.contains("unverified"), "{stamp}");
    let consumer_stamp = w.remote_stamp("record-store");
    assert!(
        consumer_stamp.contains("\"unverified\":[\"sqs file-ingest\"]"),
        "{consumer_stamp}"
    );
}

#[test]
fn s18_strict_update_on_a_current_stamp_is_a_no_op() {
    let w = with_site(GONE, true);
    assert!(w.run(&["add"]).status.success());
    let before = w.remote_stamp("ingest-api");
    assert!(before.contains("unverified"), "{before}");
    let out = w.run(&["update", "--strict"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("current"), "{}", stdout(&out));
    assert_eq!(w.remote_stamp("ingest-api"), before);
}
