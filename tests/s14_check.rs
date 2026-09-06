//! S14: the contract check between a producer's payload tables and its consumers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use serde_json::Value;

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

/// Rewrites the producer's working-tree chapter without committing it.
fn set_producer(it: &Contracts, fields: &[(&str, &str, &str)]) {
    it.w.write_docs_in(
        &it.producer,
        &[(
            "09-interfaces.md",
            &producer_page(
                "2026-09-03",
                "report-builder",
                "http",
                "GET /records",
                fields,
            ),
        )],
    );
}

#[test]
fn s14_a_field_the_consumer_reads_is_a_break_and_exit_one() {
    let it = contract_world();
    set_producer(&it, &RECORD_FIELDS[..2]);
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("record-store produces http GET /records\n"),
        "{text}"
    );
    assert!(
        text.contains("  report-builder reads id, created_at, content_type   (2026-09-01)\n"),
        "{text}"
    );
    assert!(
        text.contains("  break: content_type no longer produced\n"),
        "{text}"
    );
    assert!(text.ends_with("1 break\n"), "{text}");
}

#[test]
fn s14_a_type_change_is_a_break() {
    let it = contract_world();
    set_producer(
        &it,
        &[
            ("id", "string", "yes"),
            ("created_at", "string", "yes"),
            ("content_type", "string", "yes"),
        ],
    );
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("  break: content_type type changed: string (consumer reads enum)\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s14_a_required_flip_is_a_warning_and_exit_zero() {
    let it = contract_world();
    set_producer(
        &it,
        &[
            ("id", "string", "yes"),
            ("created_at", "string", "yes"),
            ("content_type", "enum", "no"),
        ],
    );
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("  warning: content_type required flipped: no (consumer reads yes)\n"),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_a_field_only_the_producer_lists_is_nothing() {
    let it = contract_world();
    set_producer(
        &it,
        &[
            ("id", "string", "yes"),
            ("created_at", "string", "yes"),
            ("content_type", "enum", "yes"),
            ("size", "int", "no"),
        ],
    );
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("break:"), "{text}");
    assert!(!text.contains("warning:"), "{text}");
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_an_unchanged_tree_is_clean() {
    let it = contract_world();
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("record-store produces http GET /records\n"),
        "{text}"
    );
    assert!(
        text.contains("  report-builder reads id, created_at, content_type   (2026-09-01)\n"),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_json_carries_breaks_and_keeps_ok_true() {
    let it = contract_world();
    set_producer(&it, &RECORD_FIELDS[..2]);
    let out = it.w.run_in(&it.producer, &["--json", "check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let value = json(&out);
    assert_eq!(value["ok"], Value::Bool(true));
    assert!(value["built_at_commit"].is_string(), "{value}");
    let result = &value["result"];
    assert_eq!(result["repo"], Value::from("record-store"));
    let broken = &result["breaks"][0];
    assert_eq!(broken["consumer"], Value::from("report-builder"));
    assert_eq!(broken["kind"], Value::from("http"));
    assert_eq!(broken["name"], Value::from("GET /records"));
    assert_eq!(broken["field"], Value::from("content_type"));
    assert_eq!(broken["reason"], Value::from("no longer produced"));
    let consumer = &result["contracts"][0]["consumers"][0];
    assert_eq!(
        consumer["fields"],
        serde_json::json!(["id", "created_at", "content_type"])
    );
    assert_eq!(consumer["generated_date"], Value::from("2026-09-01"));
    assert_eq!(result["warnings"], serde_json::json!([]));
    assert_eq!(result["notes"], serde_json::json!([]));
}

#[test]
fn s14_a_producer_without_a_payload_table_is_a_warning() {
    let it = contract_world();
    set_producer(&it, &[]);
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("warning: no payload table for http GET /records; nothing to compare\n"),
        "{text}"
    );
    assert!(
        text.contains("  report-builder reads id, created_at, content_type   (2026-09-01)\n"),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_a_consumer_without_a_contract_section_is_a_note() {
    let it = contract_world();
    it.w.write_docs_in(
        &it.consumer,
        &[(
            "09-interfaces.md",
            &consumer_page_with(
                "2026-09-01",
                "record-store",
                "http",
                "GET /records",
                None,
                &[],
            ),
        )],
    );
    it.w.commit_push_in(&it.consumer, "drop the section");
    assert!(it.w.run_in(&it.consumer, &["update"]).status.success());
    assert!(it.w.run_in(&it.producer, &["sync"]).status.success());
    set_producer(&it, &RECORD_FIELDS[..2]);
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "note: report-builder declares http GET /records but records no contract section\n"
        ),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_a_consumer_section_without_a_field_table_is_a_note() {
    let it = contract_world();
    it.w.write_docs_in(
        &it.consumer,
        &[(
            "09-interfaces.md",
            &consumer_page_with(
                "2026-09-01",
                "record-store",
                "http",
                "GET /records",
                Some("GET /records (v2)"),
                &[],
            ),
        )],
    );
    it.w.commit_push_in(&it.consumer, "prose only");
    assert!(it.w.run_in(&it.consumer, &["update"]).status.success());
    assert!(it.w.run_in(&it.producer, &["sync"]).status.success());
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("note: report-builder lists no fields for GET /records\n"),
        "{text}"
    );
    assert!(
        text.contains("  report-builder reads no recorded fields   (2026-09-01)\n"),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_a_contract_removed_from_produces_breaks_every_field_read() {
    let it = contract_world();
    it.w.write_docs_in(
        &it.producer,
        &[(
            "09-interfaces.md",
            "---\ngenerated_date: 2026-09-03\n---\n\n## Produces\n\n| Kind | Name | To |\n|---|---|---|\n",
        )],
    );
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("warning: http GET /records missing from Produces; treated as removed\n"),
        "{text}"
    );
    assert_eq!(text.matches("  break: ").count(), 3, "{text}");
    assert!(text.ends_with("3 breaks\n"), "{text}");
}

#[test]
fn s14_an_unresolved_consumer_edge_is_not_checked() {
    let it = contract_world();
    it.w.write_docs_in(
        &it.consumer,
        &[(
            "09-interfaces.md",
            &consumer_page_with(
                "2026-09-01",
                "ghost-store",
                "http",
                "GET /records",
                Some("GET /records (v2)"),
                RECORD_FIELDS,
            ),
        )],
    );
    it.w.commit_push_in(&it.consumer, "point at a ghost");
    assert!(it.w.run_in(&it.consumer, &["update"]).status.success());
    it.w.write_docs_in(
        &it.producer,
        &[(
            "09-interfaces.md",
            &produces_unknown_page("2026-09-03", "http", "GET /records"),
        )],
    );
    it.w.commit_push_in(&it.producer, "consumers unknown");
    assert!(it.w.run_in(&it.producer, &["update"]).status.success());
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("note: no consumers of record-store in the quarry; nothing to compare\n"),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_an_observed_only_edge_is_not_checked() {
    let it = contract_world();
    let chart = it.w.other_repo("chart-service");
    it.w.write_docs_in(&chart, &[("00-index.md", &index_page("2026-09-02"))]);
    it.w.commit_push_in(&chart, "docs");
    assert!(it.w.run_in(&chart, &["add"]).status.success());
    it.w.push_docs_root_file(
        "observed-edges.json",
        &observed_file(
            "2026-09-05",
            &[(
                "record-store",
                "chart-service",
                "http",
                "GET /records",
                "2026-09-05",
            )],
        ),
    );
    assert!(it.w.run_in(&it.producer, &["sync"]).status.success());
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("chart-service"), "{text}");
    assert_eq!(text.matches("record-store produces ").count(), 1, "{text}");
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_blocks_are_ordered_by_kind_name_then_consumer() {
    let w = world();
    let producer = w.other_repo("record-store");
    w.write_docs_in(
        &producer,
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                "---\ngenerated_date: 2026-09-03\n---\n\n## Produces\n\n| Kind | Name | To |\n|---|---|---|\n| http | GET /records | [report-builder](../report-builder/09-interfaces.md) |\n| http | GET /records | [chart-service](../chart-service/09-interfaces.md) |\n| sqs | audit-log | [report-builder](../report-builder/09-interfaces.md) |\n\n### GET /records\n\n| Field | Type | Required |\n|---|---|---|\n| id | string | yes |\n\n### audit-log\n\n| Field | Type | Required |\n|---|---|---|\n| id | string | yes |\n",
            ),
        ],
    );
    w.commit_push_in(&producer, "docs");
    assert!(w.run_in(&producer, &["add"]).status.success());

    let report = w.other_repo("report-builder");
    w.write_docs_in(
        &report,
        &[
            ("00-index.md", &index_page("2026-09-01")),
            (
                "09-interfaces.md",
                "---\ngenerated_date: 2026-09-01\n---\n\n## Consumes\n\n| Kind | Name | From |\n|---|---|---|\n| http | GET /records | [record-store](../record-store/09-interfaces.md) |\n| sqs | audit-log | [record-store](../record-store/09-interfaces.md) |\n\n### GET /records\n\n| Field | Type | Required |\n|---|---|---|\n| id | string | yes |\n\n### audit-log\n\n| Field | Type | Required |\n|---|---|---|\n| id | string | yes |\n",
            ),
        ],
    );
    w.commit_push_in(&report, "docs");
    assert!(w.run_in(&report, &["add"]).status.success());

    let chart = w.other_repo("chart-service");
    w.write_docs_in(
        &chart,
        &[
            ("00-index.md", &index_page("2026-09-02")),
            (
                "09-interfaces.md",
                &consumer_page_with(
                    "2026-09-02",
                    "record-store",
                    "http",
                    "GET /records",
                    Some("GET /records"),
                    &[("id", "string", "yes")],
                ),
            ),
        ],
    );
    w.commit_push_in(&chart, "docs");
    assert!(w.run_in(&chart, &["add"]).status.success());
    assert!(w.run_in(&producer, &["sync"]).status.success());

    let out = w.run_in(&producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("missing {needle}\n{text}"))
    };
    let http = at("record-store produces http GET /records\n");
    let chart_line = at("  chart-service reads id   (2026-09-02)\n");
    let report_line = at("  report-builder reads id   (2026-09-01)\n");
    let sqs = at("record-store produces sqs audit-log\n");
    assert!(http < chart_line, "{text}");
    assert!(chart_line < report_line, "{text}");
    assert!(report_line < sqs, "{text}");
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_check_before_add_is_a_note_not_a_refusal() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    let out = w.run(&["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("note: ingest-api is not in the docs repo; run quarry add\n"),
        "{text}"
    );
    // The unregistered-repo note is the whole answer: no second note about the
    // chapter the working tree also lacks.
    assert!(!text.contains("nothing to check"), "{text}");
    assert!(!text.contains("run Capstone map first"), "{text}");
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_check_without_init_refuses() {
    let w = world();
    let out = w.run(&["check"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("not initialised here"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s14_check_without_a_working_tree_chapter_breaks_every_consumer() {
    let it = contract_world();
    std::fs::remove_file(it.producer.join("docs/capstone/09-interfaces.md")).expect("remove");
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "note: no docs/capstone/09-interfaces.md in the working tree; run Capstone map first\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("warning: http GET /records missing from Produces; treated as removed\n"),
        "{text}"
    );
    assert!(text.ends_with("3 breaks\n"), "{text}");
}

#[test]
fn s14_check_without_a_chapter_and_without_consumers_is_a_note() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let out = w.run(&["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("note: no 09-interfaces.md in docs/capstone; nothing to check\n"),
        "{text}"
    );
    // One note for this case: the chapter note answers it, so check::run's
    // no-consumers note is dropped.
    assert!(!text.contains("nothing to compare"), "{text}");
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_a_consumer_only_edge_without_a_chapter_says_run_map() {
    let w = world();
    let consumer = w.other_repo("report-builder");
    w.write_docs_in(
        &consumer,
        &[
            ("00-index.md", &index_page("2026-09-01")),
            (
                "09-interfaces.md",
                &consumer_page_with(
                    "2026-09-01",
                    "record-store",
                    "http",
                    "GET /records",
                    Some("GET /records"),
                    RECORD_FIELDS,
                ),
            ),
        ],
    );
    w.commit_push_in(&consumer, "docs");
    assert!(w.run_in(&consumer, &["add"]).status.success());

    let producer = w.other_repo("record-store");
    w.write_docs_in(&producer, &[("00-index.md", &index_page("2026-09-03"))]);
    w.commit_push_in(&producer, "docs");
    assert!(w.run_in(&producer, &["add"]).status.success());
    assert!(w.run_in(&producer, &["sync"]).status.success());

    let out = w.run_in(&producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "note: no docs/capstone/09-interfaces.md in the working tree; run Capstone map first\n"
        ),
        "{text}"
    );
    assert!(!text.contains("nothing to check"), "{text}");
    assert!(
        text.contains(
            "note: report-builder declares http GET /records from record-store; not in record-store's Produces table\n"
        ),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_add_appends_advisory_notes() {
    let w = world();
    let consumer = w.other_repo("report-builder");
    w.write_docs_in(
        &consumer,
        &[
            ("00-index.md", &index_page("2026-09-01")),
            (
                "09-interfaces.md",
                &consumer_page_with(
                    "2026-09-01",
                    "record-store",
                    "http",
                    "GET /records",
                    Some("GET /records (v2)"),
                    RECORD_FIELDS,
                ),
            ),
        ],
    );
    w.commit_push_in(&consumer, "docs");
    assert!(w.run_in(&consumer, &["add"]).status.success());

    let producer = w.other_repo("record-store");
    w.write_docs_in(
        &producer,
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &producer_page(
                    "2026-09-03",
                    "report-builder",
                    "http",
                    "GET /records",
                    &RECORD_FIELDS[..2],
                ),
            ),
        ],
    );
    w.commit_push_in(&producer, "docs");
    let out = w.run_in(&producer, &["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "contract check: report-builder reads content_type from http GET /records; no longer produced\n"
        ),
        "{text}"
    );
    assert!(text.contains("contract check: 1 break\n"), "{text}");
}

#[test]
fn s14_update_appends_advisory_notes_and_exits_zero() {
    let it = contract_world();
    set_producer(&it, &RECORD_FIELDS[..2]);
    it.w.commit_push_in(&it.producer, "drop");
    let out = it.w.run_in(&it.producer, &["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "contract check: report-builder reads content_type from http GET /records; no longer produced\n"
        ),
        "{text}"
    );
    assert!(text.contains("contract check: 1 break\n"), "{text}");
    assert!(text.contains("-> 2 files imported\n"), "{text}");

    let again = json(&it.w.run_in(&it.producer, &["--json", "update"]));
    assert_eq!(again["result"]["result"], Value::from("current"));
    assert_eq!(again["result"]["notes"], Value::Null);
}

#[test]
fn s14_a_clean_update_carries_no_contract_notes() {
    let it = contract_world();
    it.w.write_file(&it.producer, "README.md", "# repo\n\nnow with prose\n");
    it.w.commit_push_in(&it.producer, "prose");
    let out = it.w.run_in(&it.producer, &["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(!stdout(&out).contains("contract check"), "{}", stdout(&out));
}

/// One `### <Entity>` section for the models chapter.
type Model<'a> = (&'a str, &'a [(&'a str, &'a str, &'a str)]);

/// Rewrites the producer's working-tree chapter to name a model, with the
/// models chapter beside it when `models` is given.
fn set_producer_model(
    it: &Contracts,
    schema: &str,
    inline: &[(&str, &str, &str)],
    models: Option<Model<'_>>,
) {
    it.w.write_docs_in(
        &it.producer,
        &[(
            "09-interfaces.md",
            &producer_page_with_schema(
                "2026-09-03",
                "report-builder",
                "http",
                "GET /records",
                schema,
                inline,
            ),
        )],
    );
    if let Some((entity, fields)) = models {
        it.w.write_docs_in(
            &it.producer,
            &[("02-models.md", &models_page("2026-09-03", entity, fields))],
        );
    }
}

#[test]
fn s14_a_model_reference_resolves_against_the_models_chapter() {
    let it = contract_world();
    set_producer_model(
        &it,
        "FileRecord",
        &[],
        Some(("FileRecord", &RECORD_FIELDS[..2])),
    );
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "record-store produces http GET /records (fields from 02-models.md § FileRecord)\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("  break: content_type no longer produced\n"),
        "{text}"
    );
}

#[test]
fn s14_a_list_suffix_names_the_same_model() {
    let it = contract_world();
    set_producer_model(
        &it,
        "FileRecord[]",
        &[],
        Some(("FileRecord", RECORD_FIELDS)),
    );
    let out = it.w.run_in(&it.producer, &["--json", "check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let value = json(&out);
    assert_eq!(
        value["result"]["breaks"].as_array().expect("breaks").len(),
        0
    );
    assert_eq!(value["result"]["contracts"][0]["model"], "FileRecord");
}

#[test]
fn s14_a_model_the_chapter_lacks_is_a_warning_not_a_break() {
    let it = contract_world();
    set_producer_model(&it, "FileRecord", &[], None);
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "warning: 09-interfaces.md: model FileRecord not in 02-models.md; nothing to compare\n"
        ),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");

    set_producer_model(&it, "FileRecord", &[], Some(("OtherThing", RECORD_FIELDS)));
    let again = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&again), 0, "{}", stderr(&again));
    assert!(
        stdout(&again).contains("warning: 09-interfaces.md: model FileRecord not in 02-models.md"),
        "{}",
        stdout(&again)
    );
}

#[test]
fn s14_a_section_that_lists_fields_and_names_a_model_keeps_the_table() {
    let it = contract_world();
    set_producer_model(
        &it,
        "FileRecord",
        RECORD_FIELDS,
        Some(("FileRecord", &RECORD_FIELDS[..1])),
    );
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "warning: 09-interfaces.md: http GET /records lists fields and names model FileRecord; the table wins\n"
        ),
        "{text}"
    );
    assert!(!text.contains("fields from 02-models.md"), "{text}");
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_a_model_line_in_the_payload_section_resolves_too() {
    let it = contract_world();
    it.w.write_docs_in(
        &it.producer,
        &[
            (
                "09-interfaces.md",
                &format!(
                    "{}\n### GET /records\n\nModel: `FileRecord`\n",
                    producer_page("2026-09-03", "report-builder", "http", "GET /records", &[],)
                ),
            ),
            (
                "02-models.md",
                &models_page("2026-09-03", "FileRecord", &RECORD_FIELDS[..2]),
            ),
        ],
    );
    let out = it.w.run_in(&it.producer, &["check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("(fields from 02-models.md § FileRecord)"),
        "{}",
        stdout(&out)
    );
}

/// The two fields ingest-api publishes as `FileIngestMessage`.
const INGEST_FIELDS: &[(&str, &str, &str)] = &[
    ("file_id", "string", "yes"),
    ("content_type", "enum", "yes"),
];

/// ingest-api produces `sqs file-ingest` as `FileIngestMessage`; record-store
/// reads it as `schema`, declared in its own models chapter as `entity`.
/// Neither page names the other. A non-empty `inline` gives the consumer a
/// table of its own beside the model.
fn schema_world(schema: &str, entity: &str, inline: &[(&str, &str, &str)]) -> World {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-07")),
        (
            "09-interfaces.md",
            &producer_page_with_schema(
                "2026-09-07",
                "",
                "sqs",
                "file-ingest",
                "FileIngestMessage",
                &[],
            ),
        ),
        (
            "02-models.md",
            &models_page("2026-09-07", "FileIngestMessage", INGEST_FIELDS),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());

    let consumer = w.other_repo("record-store");
    w.write_docs_in(
        &consumer,
        &[
            ("00-index.md", &index_page("2026-09-07")),
            (
                "09-interfaces.md",
                &consumer_page_with_schema("2026-09-07", "sqs", "file-ingest", schema, inline),
            ),
            (
                "02-models.md",
                &models_page("2026-09-07", entity, INGEST_FIELDS),
            ),
        ],
    );
    w.commit_push_in(&consumer, "docs");
    assert!(w.run_in(&consumer, &["add"]).status.success());
    assert!(w.run(&["sync"]).status.success());
    w
}

/// Rewrites ingest-api's working-tree models chapter.
fn set_producer_fields(w: &World, fields: &[(&str, &str, &str)]) {
    w.write_docs(&[(
        "02-models.md",
        &models_page("2026-09-07", "FileIngestMessage", fields),
    )]);
}

#[test]
fn s14_a_field_the_consumers_model_reads_is_a_break() {
    let w = schema_world("IngestedFile", "IngestedFile", &[]);
    let clean = w.run(&["check"]);
    assert_eq!(code(&clean), 0, "{}", stderr(&clean));
    assert!(
        stdout(&clean).contains("  record-store reads file_id, content_type   (2026-09-07)\n"),
        "{}",
        stdout(&clean)
    );

    set_producer_fields(&w, &INGEST_FIELDS[..1]);
    let out = w.run(&["check"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "ingest-api produces sqs file-ingest (fields from 02-models.md § FileIngestMessage)\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("  break: content_type no longer produced\n"),
        "{text}"
    );
    assert!(text.ends_with("1 break\n"), "{text}");
}

#[test]
fn s14_two_models_that_agree_are_clean() {
    let w = schema_world("IngestedFile", "IngestedFile", &[]);
    let out = w.run(&["--json", "check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let value = json(&out);
    assert_eq!(
        value["result"]["breaks"].as_array().expect("breaks").len(),
        0,
        "{}",
        stdout(&out)
    );
    let consumer = &value["result"]["contracts"][0]["consumers"][0];
    assert_eq!(
        consumer["fields"],
        serde_json::json!(["file_id", "content_type"])
    );
}

#[test]
fn s14_a_consumer_model_its_own_chapter_lacks_is_a_note() {
    let w = schema_world("IngestedFile", "SomethingElse", &[]);
    set_producer_fields(&w, &INGEST_FIELDS[..1]);
    let out = w.run(&["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("note: record-store lists no fields for file-ingest\n"),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}

#[test]
fn s14_a_consumers_inline_table_wins_over_its_schema() {
    let w = schema_world("IngestedFile", "IngestedFile", &INGEST_FIELDS[..1]);
    set_producer_fields(&w, &INGEST_FIELDS[..1]);
    let out = w.run(&["check"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "warning: record-store: file-ingest lists fields and names model IngestedFile; the table wins\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("  record-store reads file_id   (2026-09-07)\n"),
        "{text}"
    );
    assert!(text.ends_with("no breaks\n"), "{text}");
}
