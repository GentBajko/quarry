//! The rebuild budget from the architecture chapter's quality attributes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
#[ignore = "builds a 5,000-page docs repo"]
fn rebuild_of_five_thousand_pages_is_under_ten_seconds() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    let mut pages: Vec<(String, String)> =
        vec![("00-index.md".to_string(), index_page("2026-09-04"))];
    for i in 0..5_000 {
        pages.push((
            format!("chapters/{i:05}.md"),
            format!(
                "---\ngenerated_date: 2026-09-04\n---\n\n## Section {i}\n\nSome prose about component {i} and its neighbours.\n\n## Other {i}\n\nMore prose.\n"
            ),
        ));
    }
    let refs: Vec<(&str, &str)> = pages
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    w.write_docs(&refs);
    w.commit_push("many docs");
    assert!(w.run(&["add"]).status.success());

    let started = std::time::Instant::now();
    let out = w.run(&["docs", "index", "--force"]);
    let elapsed = started.elapsed();
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "rebuild took {elapsed:?}: {}",
        stdout(&out)
    );
}
