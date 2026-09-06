//! S20: edges the name join makes from rows that name no far end.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use serde_json::Value;

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

/// The default source repo linked to the docs repo, so a read command runs.
fn quarry_world() -> World {
    let w = world();
    let out = w.run(&["init", "--url", &w.docs_url]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    w
}

/// Pulls what the other repos pushed into this repo's clone.
fn synced(w: &World) {
    let out = w.run(&["sync"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
}

/// Registers one repo whose interfaces page is `page`.
fn repo_with(w: &World, name: &str, date: &str, page: &str) -> std::path::PathBuf {
    let path = w.other_repo(name);
    w.write_docs_in(
        &path,
        &[
            ("00-index.md", &index_page(date)),
            ("09-interfaces.md", page),
        ],
    );
    w.commit_push_in(&path, "docs");
    let out = w.run_in(&path, &["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    synced(w);
    path
}

/// record-store publishes `http GET /records`; report-builder reads it. Neither
/// page names the other.
fn joined_world() -> World {
    let w = quarry_world();
    repo_with(
        &w,
        "record-store",
        "2026-09-03",
        &open_edges_page("2026-09-03", &[("http", "GET /records")], &[]),
    );
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
    );
    w
}

#[test]
fn s20_one_partner_resolves_both_directions() {
    let w = joined_world();
    let out = w.run(&["--json", "docs", "deps", "record-store", "--downstream"]);
    let value = json(&out);
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "{}", stdout(&out));
    assert_eq!(edges[0]["repo"], "report-builder");
    assert_eq!(edges[0]["declared_by"], "joined");
    assert_eq!(edges[0]["resolved_by"], "name");
    assert_eq!(edges[0]["missing"], false);
    assert_eq!(edges[0]["via"].as_array().expect("via").len(), 2);

    let up = w.run(&["docs", "deps", "report-builder", "--upstream"]);
    let text = stdout(&up);
    assert!(
        text.contains("http GET /records <- record-store (resolved by name)"),
        "{text}"
    );

    let show = w.run(&["docs", "show", "record-store"]);
    assert!(
        stdout(&show).contains("produces: http GET /records -> report-builder (resolved by name)"),
        "{}",
        stdout(&show)
    );
}

#[test]
fn s20_two_producers_are_ambiguous_and_make_no_edge() {
    let w = quarry_world();
    repo_with(
        &w,
        "record-store",
        "2026-09-03",
        &open_edges_page("2026-09-03", &[("http", "GET /records")], &[]),
    );
    repo_with(
        &w,
        "archive-api",
        "2026-09-03",
        &open_edges_page("2026-09-03", &[("http", "GET /records")], &[]),
    );
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
    );
    let index = w.run(&["docs", "index", "--force"]);
    let text = stdout(&index);
    assert!(
        text.contains(
            "unresolved: http GET /records consumed by report-builder: 2 producers, archive-api, record-store\n"
        ),
        "{text}"
    );
    assert!(text.contains("0 edges"), "{text}");

    let deps = w.run(&["docs", "deps", "report-builder", "--upstream"]);
    assert!(
        stdout(&deps).contains("no edges declared"),
        "{}",
        stdout(&deps)
    );
    let show = w.run(&["docs", "show", "record-store"]);
    assert!(
        stdout(&show).contains("produces: http GET /records -> (unknown)"),
        "{}",
        stdout(&show)
    );
}

#[test]
fn s20_one_producer_reaches_every_consumer_that_reads_it() {
    let w = quarry_world();
    repo_with(
        &w,
        "record-store",
        "2026-09-03",
        &open_edges_page("2026-09-03", &[("http", "GET /records")], &[]),
    );
    for name in ["report-builder", "billing"] {
        repo_with(
            &w,
            name,
            "2026-09-01",
            &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
        );
    }
    let out = w.run(&["--json", "docs", "deps", "record-store", "--downstream"]);
    let value = json(&out);
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 2, "{}", stdout(&out));
    let reached: Vec<&str> = edges.iter().filter_map(|e| e["repo"].as_str()).collect();
    assert_eq!(reached, ["billing", "report-builder"]);
    assert!(
        edges
            .iter()
            .all(|e| e["declared_by"] == "joined" && e["resolved_by"] == "name"),
        "{}",
        stdout(&out)
    );
    for name in ["report-builder", "billing"] {
        let up = w.run(&["docs", "deps", name, "--upstream"]);
        assert!(
            stdout(&up).contains("http GET /records <- record-store (resolved by name)"),
            "{}: {}",
            name,
            stdout(&up)
        );
    }
    let show = w.run(&["docs", "show", "record-store"]);
    assert!(!stdout(&show).contains("-> (unknown)"), "{}", stdout(&show));
}

#[test]
fn s20_two_producers_leave_every_consumer_ambiguous() {
    let w = quarry_world();
    for name in ["record-store", "archive-api"] {
        repo_with(
            &w,
            name,
            "2026-09-03",
            &open_edges_page("2026-09-03", &[("http", "GET /records")], &[]),
        );
    }
    for name in ["report-builder", "billing"] {
        repo_with(
            &w,
            name,
            "2026-09-01",
            &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
        );
    }
    let out = w.run(&["--json", "docs", "index", "--force"]);
    let value = json(&out);
    assert_eq!(value["result"]["edges"], 0, "{}", stdout(&out));
    let rows = value["result"]["ambiguous"].as_array().expect("ambiguous");
    let reported: Vec<&str> = rows.iter().filter_map(|r| r["repo"].as_str()).collect();
    assert_eq!(reported, ["billing", "report-builder"]);
    assert!(
        rows.iter()
            .all(|r| r["candidates"] == serde_json::json!(["archive-api", "record-store"])),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s20_ambiguity_reaches_json_with_its_candidates() {
    let w = quarry_world();
    for name in ["record-store", "archive-api"] {
        repo_with(
            &w,
            name,
            "2026-09-03",
            &open_edges_page("2026-09-03", &[("http", "GET /records")], &[]),
        );
    }
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
    );
    let out = w.run(&["--json", "docs", "index", "--force"]);
    let value = json(&out);
    let rows = value["result"]["ambiguous"].as_array().expect("ambiguous");
    assert_eq!(rows.len(), 1, "{}", stdout(&out));
    assert_eq!(rows[0]["direction"], "consumes");
    assert_eq!(rows[0]["kind"], "http");
    assert_eq!(rows[0]["name"], "GET /records");
    assert_eq!(rows[0]["repo"], "report-builder");
    assert_eq!(
        rows[0]["candidates"],
        serde_json::json!(["archive-api", "record-store"])
    );
}

#[test]
fn s20_a_current_index_still_reports_its_ambiguities() {
    let w = quarry_world();
    for name in ["record-store", "archive-api"] {
        repo_with(
            &w,
            name,
            "2026-09-03",
            &open_edges_page("2026-09-03", &[("http", "GET /records")], &[]),
        );
    }
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
    );
    assert_eq!(code(&w.run(&["docs", "index", "--force"])), 0);
    let out = w.run(&["docs", "index"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("index current"), "{text}");
    assert!(
        text.contains("consumed by report-builder: 2 producers"),
        "{text}"
    );
}

#[test]
fn s20_an_explicit_name_is_never_overridden() {
    let w = quarry_world();
    repo_with(
        &w,
        "record-store",
        "2026-09-03",
        &produces_page("2026-09-03", "report-builder", "http", "GET /records"),
    );
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
    );
    repo_with(
        &w,
        "chart-service",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
    );
    let out = w.run(&["--json", "docs", "deps", "record-store", "--downstream"]);
    let value = json(&out);
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "{}", stdout(&out));
    assert_eq!(edges[0]["repo"], "report-builder");
    assert_eq!(edges[0]["declared_by"], "producer");
    assert_eq!(edges[0]["resolved_by"], Value::Null);
}

#[test]
fn s20_unknown_never_joins() {
    let w = quarry_world();
    repo_with(
        &w,
        "record-store",
        "2026-09-03",
        &produces_unknown_page("2026-09-03", "http", "GET /records"),
    );
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "GET /records")]),
    );
    let index = w.run(&["docs", "index", "--force"]);
    assert!(stdout(&index).contains("0 edges"), "{}", stdout(&index));
    let show = w.run(&["docs", "show", "record-store"]);
    assert!(
        stdout(&show).contains("produces: http GET /records -> (unknown)"),
        "{}",
        stdout(&show)
    );
}

#[test]
fn s20_a_route_joins_whatever_the_parameter_is_called() {
    let w = quarry_world();
    repo_with(
        &w,
        "identity-api",
        "2026-09-03",
        &open_edges_page("2026-09-03", &[("http", "GET /users/{id}")], &[]),
    );
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("http", "get /users/:id")]),
    );
    let deps = w.run(&["docs", "deps", "identity-api", "--downstream"]);
    let text = stdout(&deps);
    assert!(
        text.contains("http GET /users/{id} -> report-builder (resolved by name)"),
        "{text}"
    );
}

#[test]
fn s20_a_kind_that_is_not_a_route_joins_on_the_name_as_written() {
    let w = quarry_world();
    repo_with(
        &w,
        "record-store",
        "2026-09-03",
        &open_edges_page("2026-09-03", &[("sqs", "File-Ingest")], &[]),
    );
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page("2026-09-01", &[], &[("sqs", "file-ingest")]),
    );
    let index = w.run(&["docs", "index", "--force"]);
    assert!(stdout(&index).contains("0 edges"), "{}", stdout(&index));
}

#[test]
fn s20_grpc_and_websocket_names_join_like_a_route() {
    let w = quarry_world();
    repo_with(
        &w,
        "record-store",
        "2026-09-03",
        &open_edges_page(
            "2026-09-03",
            &[("grpc", "UserService.GetUser"), ("ws", "WS /live-chat")],
            &[],
        ),
    );
    // A gRPC method spelled in another case, and a websocket verb the other
    // side wrote in lower case: both are routed kinds, so both fold.
    repo_with(
        &w,
        "report-builder",
        "2026-09-01",
        &open_edges_page(
            "2026-09-01",
            &[],
            &[("grpc", "userservice.getuser"), ("ws", "ws /live-chat")],
        ),
    );
    let deps = w.run(&["docs", "deps", "record-store", "--downstream"]);
    let text = stdout(&deps);
    assert!(
        text.contains("grpc UserService.GetUser -> report-builder (resolved by name)"),
        "{text}"
    );
    assert!(
        text.contains("ws WS /live-chat -> report-builder (resolved by name)"),
        "{text}"
    );
}

#[test]
fn s20_a_rebuild_answers_the_same_way_twice() {
    let w = joined_world();
    repo_with(
        &w,
        "chart-service",
        "2026-09-02",
        &open_edges_page(
            "2026-09-02",
            &[("http", "GET /charts")],
            &[("http", "GET /records")],
        ),
    );
    let first = json(&w.run(&["--json", "docs", "index", "--force"]));
    let second = json(&w.run(&["--json", "docs", "index", "--force"]));
    assert_eq!(first["result"], second["result"]);
    let deps_first = json(&w.run(&["--json", "docs", "deps", "record-store", "--downstream"]));
    let deps_second = json(&w.run(&["--json", "docs", "deps", "record-store", "--downstream"]));
    assert_eq!(deps_first["result"], deps_second["result"]);
}
