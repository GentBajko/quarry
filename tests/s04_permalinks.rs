//! S4: permalink rewrite, end to end through a real import.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s4_an_import_pins_pointers_to_the_imported_commit() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    std::fs::create_dir_all(w.source.join("src/publish")).expect("src");
    std::fs::write(w.source.join("src/publish/sqs.py"), "print(1)\n").expect("code");
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "01-architecture.md",
            "---\ngenerated_date: 2026-09-04\nsite: src/publish/sqs.py:57\n---\n\n## Entry points\n\nPublished at `src/publish/sqs.py:57`, gone at `src/gone.py:3`.\n\n```\n`src/publish/sqs.py:57`\n```\n",
        ),
    ]);
    let head = w.commit_push("docs and code");
    assert!(w.run(&["add"]).status.success());

    let page = w.remote_file("ingest-api/01-architecture.md");
    assert!(
        page.contains(&format!("[src/publish/sqs.py:57](https://localhost/remotes/ingest-api/blob/{head}/src/publish/sqs.py#L57)")),
        "{page}"
    );
    assert!(
        page.contains("`src/gone.py:3`"),
        "untracked path was rewritten: {page}"
    );
    assert!(
        page.contains("---\ngenerated_date: 2026-09-04\nsite: src/publish/sqs.py:57\n---"),
        "frontmatter was rewritten: {page}"
    );
    assert_eq!(
        page.matches("https://").count(),
        1,
        "fenced block was rewritten: {page}"
    );
}

#[test]
fn s4_a_custom_template_wins() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    let config_path = w.source.join(".quarry/.config");
    let config = std::fs::read_to_string(&config_path).expect("config");
    let config = config.replace(
        "\"permalink_template\": null",
        "\"permalink_template\": \"https://code.internal/{repo}/{sha}/{path}#{line}\"",
    );
    std::fs::write(&config_path, config).expect("write config");
    std::fs::write(w.source.join("main.rs"), "fn main() {}\n").expect("code");
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "01-architecture.md",
            "---\ngenerated_date: 2026-09-04\n---\n\n## Entry points\n\n`main.rs:1`\n",
        ),
    ]);
    let head = w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let page = w.remote_file("ingest-api/01-architecture.md");
    assert!(
        page.contains(&format!(
            "https://code.internal/ingest-api/{head}/main.rs#1"
        )),
        "{page}"
    );
}
