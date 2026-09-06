//! S13: output envelope and exit codes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use serde_json::Value;

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

#[test]
fn s13_bare_quarry_prints_help_and_exits_zero() {
    let w = world();
    let out = w.run(&[]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    for verb in ["init", "add", "update", "sync", "remove", "check", "docs"] {
        assert!(text.contains(verb), "{text}");
    }
    assert!(text.contains("Support quarry"), "{text}");
}

#[test]
fn s13_bare_docs_prints_the_subcommand_help() {
    let w = world();
    let out = w.run(&["docs"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    for verb in ["list", "show", "section", "search", "deps", "path", "index"] {
        assert!(text.contains(verb), "{text}");
    }
}

#[test]
fn s13_an_unknown_command_exits_one() {
    let w = world();
    let out = w.run(&["nonsense"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stdout(&out).is_empty(), "{}", stdout(&out));
}

#[test]
fn s13_version_exits_zero() {
    let w = world();
    let out = w.run(&["--version"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("quarry 0.1.0"), "{}", stdout(&out));
}

#[test]
fn s13_every_answer_carries_the_index_stamps() {
    let it = wired();
    let out = it.w.run(&["--json", "docs", "list"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let value = json(&out);
    assert_eq!(value["ok"], Value::Bool(true));
    assert!(value["built_at_commit"].is_string(), "{value}");
    assert!(value["synced_at"].is_string(), "{value}");
    let rows = value["result"].as_array().expect("rows");
    assert_eq!(rows.len(), 3);
    assert!(rows[0]["newest_generated_date"].is_string(), "{value}");
}

#[test]
fn s13_a_refusal_is_a_json_document_on_stdout_with_code_one() {
    let it = wired();
    let out = it.w.run(&["--json", "docs", "show", "ghost"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    let value = json(&out);
    assert_eq!(value["ok"], Value::Bool(false));
    assert_eq!(value["code"], Value::from(1));
    assert_eq!(value["error"], Value::from("unknown repo ghost"));
}

#[test]
fn s13_an_external_failure_is_code_two_in_json() {
    let w = world();
    let out = w.run(&["--json", "init", "--url", "file:///nonexistent/docs.git"]);
    assert_eq!(code(&out), 2, "{}", stdout(&out));
    let value = json(&out);
    assert_eq!(value["code"], Value::from(2));
}

#[test]
fn s13_write_commands_report_their_result() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    let added = w.run(&["--json", "add"]);
    assert_eq!(code(&added), 0, "{}", stdout(&added));
    let value = json(&added);
    assert_eq!(value["result"]["repo"], Value::from("ingest-api"));
    assert_eq!(value["result"]["result"], Value::from("imported"));
    assert_eq!(value["result"]["files"], Value::from(1));

    let again = json(&w.run(&["--json", "update"]));
    assert_eq!(again["result"]["result"], Value::from("current"));
}

#[test]
fn s13_deps_json_names_the_declaring_page() {
    let it = wired();
    let value = json(
        &it.w
            .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]),
    );
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges[0]["repo"], Value::from("record-store"));
    assert_eq!(edges[0]["kind"], Value::from("sqs"));
    assert_eq!(edges[0]["depth"], Value::from(1));
    let via = edges[0]["via"].as_array().expect("via");
    assert!(
        via.iter()
            .any(|v| v.as_str() == Some("record-store/09-interfaces.md")),
        "{via:?}"
    );
}

#[test]
fn s13_section_json_carries_the_body_and_date() {
    let it = wired();
    let value = json(&it.w.run(&[
        "--json",
        "docs",
        "section",
        "record-store",
        "file-ingest (v2)",
    ]));
    assert_eq!(value["result"]["heading"], Value::from("file-ingest (v2)"));
    assert_eq!(value["result"]["generated_date"], Value::from("2026-09-03"));
    assert!(
        value["result"]["body"]
            .as_str()
            .unwrap_or_default()
            .contains("file_id"),
        "{value}"
    );
}

#[test]
fn s13_json_never_prompts() {
    let w = world();
    let out = w.run(&["--json", "init"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let value = json(&out);
    assert!(
        value["error"]
            .as_str()
            .unwrap_or_default()
            .contains("--url"),
        "{value}"
    );
}

#[test]
fn s13_verbose_echoes_git_to_stderr() {
    let w = world();
    let out = w.run(&["--verbose", "init", "--url", &w.docs_url]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stderr(&out).contains("+ git clone"), "{}", stderr(&out));
}
