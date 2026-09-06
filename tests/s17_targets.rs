//! S17: monorepo targets.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use serde_json::Value;

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

/// The registered monorepo with all three folders imported.
fn imported() -> Mono {
    let mono = monorepo();
    let out = mono.w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}\n{}", stdout(&out), stderr(&out));
    mono
}

fn config_text(mono: &Mono) -> String {
    std::fs::read_to_string(mono.w.source.join(".quarry/.config")).expect("config")
}

#[test]
fn s17_init_name_and_docs_dir_append_targets() {
    let mono = monorepo();
    let config = config_text(&mono);
    assert!(config.contains("\"targets\": ["), "{config}");
    assert!(
        config.find("\"name\": \"billing\"").unwrap()
            < config.find("\"name\": \"orders\"").unwrap(),
        "{config}"
    );
    assert!(
        config.contains("\"docs_dir\": \"docs/capstone\""),
        "the root docs dir moved: {config}"
    );
    let again = mono.w.run(&[
        "init",
        "--name",
        "orders",
        "--docs-dir",
        "services/orders/docs/capstone",
    ]);
    assert!(
        stdout(&again).contains("target orders: services/orders/docs/capstone"),
        "{}",
        stdout(&again)
    );
}

#[test]
fn s17_init_name_without_docs_dir_refuses() {
    let w = world();
    let out = w.run(&["init", "--url", &w.docs_url, "--name", "billing"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("--docs-dir"), "{}", stderr(&out));
}

#[test]
fn s17_json_init_without_docs_dir_is_a_refusal_envelope() {
    let w = world();
    let out = w.run(&["--json", "init", "--url", &w.docs_url, "--name", "x"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    let value = json(&out);
    assert_eq!(value["ok"], Value::Bool(false));
    assert_eq!(value["code"], Value::from(1));
    assert!(
        value["error"]
            .as_str()
            .unwrap_or_default()
            .contains("--docs-dir"),
        "{value}"
    );
}

#[test]
fn s17_init_rejects_a_multi_segment_target_name() {
    let w = world();
    let out = w.run(&[
        "init",
        "--url",
        &w.docs_url,
        "--name",
        "a/b",
        "--docs-dir",
        "services/a/docs",
    ]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("target name must be one path segment"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s17_init_name_equal_to_the_repo_name_refuses() {
    let w = world();
    let out = w.run(&[
        "init",
        "--url",
        &w.docs_url,
        "--name",
        "ingest-api",
        "--docs-dir",
        "x",
    ]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("own name"), "{}", stderr(&out));
}

#[test]
fn s17_docs_dir_without_a_name_sets_the_root() {
    let w = world();
    let out = w.run(&["init", "--url", &w.docs_url, "--docs-dir", "documentation"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let config = std::fs::read_to_string(w.source.join(".quarry/.config")).expect("config");
    assert!(
        config.contains("\"docs_dir\": \"documentation\""),
        "{config}"
    );
    assert!(!config.contains("targets"), "{config}");
}

#[test]
fn s17_a_reinit_without_name_keeps_the_targets() {
    let mono = monorepo();
    let out = mono.w.run(&["init", "--url", &mono.w.docs_url]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let config = config_text(&mono);
    assert!(config.contains("\"name\": \"billing\""), "{config}");
    assert!(config.contains("\"name\": \"orders\""), "{config}");
}

#[test]
fn s17_a_hand_edited_target_name_is_refused_before_any_write() {
    for (name, docs_dir, expected) in [
        (
            "..",
            "services/billing/docs/capstone",
            "target name must be one path segment",
        ),
        (
            "../../x",
            "services/billing/docs/capstone",
            "target name must be one path segment",
        ),
        (
            "a/b",
            "services/billing/docs/capstone",
            "target name must be one path segment",
        ),
        (
            "billing",
            "../x",
            "docs dir must be a relative path inside the repo",
        ),
        (
            // Two targets under one name: billing's block is renamed onto
            // orders', so both would import into the `orders` folder.
            "orders",
            "services/billing/docs/capstone",
            "targets orders is listed twice",
        ),
    ] {
        let mono = imported();
        let path = mono.w.source.join(".quarry/.config");
        let text = std::fs::read_to_string(&path).expect("config");
        let edited = text
            .replace("\"name\": \"billing\"", &format!("\"name\": \"{name}\""))
            .replace(
                "\"docs_dir\": \"services/billing/docs/capstone\"",
                &format!("\"docs_dir\": \"{docs_dir}\""),
            );
        std::fs::write(&path, edited).expect("write config");
        let before = mono.w.remote_files();
        for verb in ["add", "update", "remove"] {
            let out = mono.w.run(&[verb]);
            assert_eq!(code(&out), 1, "{verb} {name}: {}", stdout(&out));
            let err = stderr(&out);
            assert!(err.contains(expected), "{verb} {name}: {err}");
        }
        assert_eq!(mono.w.remote_files(), before, "{name} wrote to the remote");
    }
}

#[test]
fn s17_add_imports_every_target_and_the_umbrella_as_siblings() {
    let mono = imported();
    let files = mono.w.remote_files();
    for want in [
        "00-index.md",
        "billing/00-index.md",
        "billing/09-interfaces.md",
        "billing/.quarry-stamp",
        "orders/00-index.md",
        "orders/09-interfaces.md",
        "ingest-api/00-index.md",
    ] {
        assert!(files.contains(&want.to_string()), "{files:?}");
    }
    let root = mono.w.remote_file("00-index.md");
    let rows: Vec<&str> = root.lines().filter(|l| l.starts_with("| [")).collect();
    assert_eq!(rows.len(), 3, "{root}");
    assert!(rows[0].contains("[billing]"), "{root}");
    assert!(rows[1].contains("[ingest-api]"), "{root}");
    assert!(rows[2].contains("[orders]"), "{root}");
}

#[test]
fn s17_umbrella_links_point_at_sibling_folders() {
    let mono = imported();
    let page = mono.w.remote_file("ingest-api/00-index.md");
    assert!(page.contains("](../billing/00-index.md)"), "{page}");
    assert!(page.contains("](../orders/00-index.md#overview)"), "{page}");
    assert!(!page.contains("services/"), "{page}");
}

#[test]
fn s17_stamps_record_each_docs_dir() {
    let mono = imported();
    let billing = mono.w.remote_stamp("billing");
    assert!(
        billing.contains("\"docs_dir\":\"services/billing/docs/capstone\""),
        "{billing}"
    );
    let umbrella = mono.w.remote_stamp("ingest-api");
    assert!(
        umbrella.contains("\"docs_dir\":\"docs/capstone\""),
        "{umbrella}"
    );
    for stamp in [&billing, &umbrella] {
        assert!(
            stamp.contains("\"origin\":\"localhost/remotes/ingest-api\""),
            "{stamp}"
        );
    }
}

#[test]
fn s17_a_multi_target_write_is_one_commit() {
    let mono = imported();
    let clone = mono.w.docs_clone("log-peek");
    let log = stdout(&mono.w.git(&clone, &["log", "--format=%s", "main"]));
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 1, "{log}");
    assert_eq!(
        lines[0],
        format!("add billing, orders, ingest-api @{}", short(&mono.head))
    );
}

#[test]
fn s17_add_json_is_one_block_per_target() {
    let mono = monorepo();
    let out = mono.w.run(&["--json", "add"]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    let value = json(&out);
    let blocks = value["result"].as_array().expect("array");
    assert_eq!(blocks.len(), 3, "{value}");
    assert_eq!(blocks[0]["repo"], Value::from("billing"));
    assert_eq!(blocks[1]["repo"], Value::from("orders"));
    assert_eq!(blocks[2]["repo"], Value::from("ingest-api"));
    for block in blocks {
        assert_eq!(block["result"], Value::from("imported"), "{value}");
    }
}

#[test]
fn s17_update_after_add_is_current_for_every_target() {
    let mono = imported();
    let out = mono.w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    for name in ["billing", "orders", "ingest-api"] {
        assert!(
            text.contains(&format!("{name} @ {}: current", short(&mono.head))),
            "{text}"
        );
    }
    let value = json(&mono.w.run(&["--json", "update"]));
    let blocks = value["result"].as_array().expect("array");
    assert_eq!(blocks.len(), 3, "{value}");
    for block in blocks {
        assert_eq!(block["result"], Value::from("current"), "{value}");
    }
    let clone = mono.w.docs_clone("sync-peek");
    let before = stdout(&mono.w.git(&clone, &["rev-parse", "main"]));
    assert!(mono.w.run(&["sync"]).status.success());
    let after = stdout(&mono.w.git(&clone, &["rev-parse", "main"]));
    assert_eq!(before, after);
}

#[test]
fn s17_a_change_in_one_workspace_reimports_all_three() {
    let mono = imported();
    mono.w.write_file(
        &mono.w.source,
        "services/billing/docs/capstone/00-index.md",
        &index_page("2026-09-06"),
    );
    let head = mono.w.commit_push("billing moved");
    let out = mono.w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    for name in ["billing", "orders", "ingest-api"] {
        let stamp = mono.w.remote_stamp(name);
        assert!(stamp.contains(&head), "{name}: {stamp}");
    }
    let text = stdout(&out);
    assert_eq!(
        text.matches("root index regenerated; pushed").count(),
        1,
        "{text}"
    );
    assert!(text.ends_with("root index regenerated; pushed\n"), "{text}");
}

#[test]
fn s17_a_rejected_multi_target_push_is_redone_and_lands() {
    let mono = imported();
    let hook = mono.w.base().join("remotes/docs-quarry.git/hooks/update");
    std::fs::write(
        &hook,
        "#!/bin/sh\nif [ ! -f \"$GIT_DIR/rejected-once\" ]; then\n  touch \"$GIT_DIR/rejected-once\"\n  echo 'rejected by test hook' >&2\n  exit 1\nfi\nexit 0\n",
    )
    .expect("write hook");
    let mut perms = std::fs::metadata(&hook).expect("meta").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&hook, perms).expect("chmod");
    mono.w.write_file(
        &mono.w.source,
        "services/billing/docs/capstone/01-architecture.md",
        "---\ngenerated_date: 2026-09-06\n---\n\n# Layers\n\nretried\n",
    );
    let head = mono.w.commit_push("retried");
    let out = mono.w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}\n{}", stdout(&out), stderr(&out));
    assert!(
        mono.w
            .remote_file("billing/01-architecture.md")
            .contains("retried")
    );
    for name in ["billing", "orders", "ingest-api"] {
        assert!(mono.w.remote_stamp(name).contains(&head), "{name}");
    }
    let clone = mono.w.docs_clone("redo-peek");
    let log = stdout(&mono.w.git(&clone, &["log", "--format=%s", "main"]));
    assert_eq!(
        log.lines().count(),
        2,
        "the redo left more than one commit: {log}"
    );
}

#[test]
fn s17_a_target_without_an_index_page_refuses_on_add() {
    let mono = monorepo();
    std::fs::remove_file(
        mono.w
            .source
            .join("services/orders/docs/capstone/00-index.md"),
    )
    .expect("remove index");
    mono.w.commit_push("drop the orders index");
    let out = mono.w.run(&["add"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("no 00-index.md in services/orders/docs/capstone"),
        "{}",
        stderr(&out)
    );
    let files = mono.w.remote_files();
    assert!(
        !files.iter().any(|f| f.starts_with("billing/")),
        "{files:?}"
    );
}

#[test]
fn s17_no_root_index_means_no_umbrella() {
    let mono = monorepo();
    std::fs::remove_file(mono.w.source.join("docs/capstone/00-index.md")).expect("remove index");
    mono.w.commit_push("drop the umbrella");
    let out = mono.w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}\n{}", stdout(&out), stderr(&out));
    let files = mono.w.remote_files();
    assert!(files.iter().any(|f| f.starts_with("billing/")), "{files:?}");
    assert!(files.iter().any(|f| f.starts_with("orders/")), "{files:?}");
    assert!(
        !files.iter().any(|f| f.starts_with("ingest-api/")),
        "{files:?}"
    );
}

#[test]
fn s17_a_nested_target_is_imported_once() {
    let w = world();
    w.write_files_in(
        &w.source,
        &[
            (
                "docs/capstone/billing/00-index.md",
                &index_page("2026-09-04"),
            ),
            (
                "docs/capstone/billing/01-architecture.md",
                "---\ngenerated_date: 2026-09-04\n---\n\n# Layers\n\nx\n",
            ),
            (
                "docs/capstone/00-index.md",
                &umbrella_index("2026-09-04", &[("billing", "billing/00-index.md")]),
            ),
        ],
    );
    let init = w.run(&[
        "init",
        "--url",
        &w.docs_url,
        "--name",
        "billing",
        "--docs-dir",
        "docs/capstone/billing",
    ]);
    assert_eq!(code(&init), 0, "{}", stderr(&init));
    w.commit_push("docs");
    let out = w.run(&["--json", "add"]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    let value = json(&out);
    let blocks = value["result"].as_array().expect("array");
    assert_eq!(blocks[0]["repo"], Value::from("billing"));
    assert_eq!(blocks[0]["files"], Value::from(2), "{value}");
    assert_eq!(blocks[1]["repo"], Value::from("ingest-api"));
    assert_eq!(
        blocks[1]["files"],
        Value::from(1),
        "the umbrella imported the nested pages again: {value}"
    );
    let files = w.remote_files();
    assert!(
        !files.iter().any(|f| f.starts_with("ingest-api/billing/")),
        "{files:?}"
    );
    assert!(
        files.contains(&"billing/01-architecture.md".to_string()),
        "{files:?}"
    );
    let page = w.remote_file("ingest-api/00-index.md");
    assert!(page.contains("](../billing/00-index.md)"), "{page}");
}

#[test]
fn s17_a_folder_claimed_by_another_docs_dir_refuses() {
    let mono = imported();
    mono.w.write_file(
        &mono.w.source,
        "services/billing2/docs/capstone/00-index.md",
        &index_page("2026-09-05"),
    );
    let moved = mono.w.run(&[
        "init",
        "--name",
        "billing",
        "--docs-dir",
        "services/billing2/docs/capstone",
    ]);
    assert_eq!(code(&moved), 0, "{}", stderr(&moved));
    mono.w.commit_push("move billing");
    let out = mono.w.run(&["update"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains(
            "name billing already used by localhost/remotes/ingest-api at services/billing/docs/capstone"
        ),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s17_an_old_stamp_without_docs_dir_still_updates() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    let first = w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());

    let clone = w.docs_clone("legacy-stamp");
    std::fs::write(
        clone.join("ingest-api/.quarry-stamp"),
        format!("{{\"commit\":\"{first}\",\"origin\":\"localhost/remotes/ingest-api\"}}\n"),
    )
    .expect("write stamp");
    w.git(&clone, &["add", "-A"]);
    w.git(&clone, &["commit", "--quiet", "-m", "legacy stamp"]);
    assert!(
        w.git(&clone, &["push", "--quiet", "origin", "main"])
            .status
            .success()
    );

    w.write_docs(&[("01-architecture.md", "# next\n")]);
    let second = w.commit_push("more docs");
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let stamp = w.remote_stamp("ingest-api");
    assert!(stamp.contains(&second), "{stamp}");
    assert!(stamp.contains("\"docs_dir\":\"docs/capstone\""), "{stamp}");
}

#[test]
fn s17_remove_drops_every_target_and_the_umbrella() {
    let mono = imported();
    let out = mono.w.run(&["remove"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    for name in ["billing", "orders", "ingest-api"] {
        assert!(text.contains(&format!("removed {name}")), "{text}");
    }
    let files = mono.w.remote_files();
    for prefix in ["billing/", "orders/", "ingest-api/"] {
        assert!(!files.iter().any(|f| f.starts_with(prefix)), "{files:?}");
    }
    let root = mono.w.remote_file("00-index.md");
    assert!(!root.contains("| ["), "{root}");
    let clone = mono.w.docs_clone("remove-peek");
    let log = stdout(&mono.w.git(&clone, &["log", "--format=%s", "main"]));
    assert!(
        log.lines()
            .any(|l| l == "remove billing, orders, ingest-api"),
        "{log}"
    );
}

#[test]
fn s17_remove_json_is_one_block_per_target() {
    let mono = imported();
    let value = json(&mono.w.run(&["--json", "remove"]));
    let blocks = value["result"].as_array().expect("array");
    assert_eq!(blocks.len(), 3, "{value}");
    for block in blocks {
        assert_eq!(block["removed"], Value::Bool(true), "{value}");
    }
}

#[test]
fn s17_remove_dangling_excludes_the_sibling_targets() {
    let mono = imported();
    let value = json(&mono.w.run(&["--json", "remove"]));
    let blocks = value["result"].as_array().expect("array");
    for block in blocks {
        let dangling = block["dangling"].as_array().expect("dangling");
        assert!(
            dangling.is_empty(),
            "a sibling removed in the same run is not dangling: {value}"
        );
    }
}

#[test]
fn s17_deps_walk_across_targets() {
    let mono = imported();
    assert!(mono.w.run(&["sync"]).status.success());
    let down = mono.w.run(&["docs", "deps", "billing", "--downstream"]);
    assert_eq!(code(&down), 0, "{}", stderr(&down));
    let text = stdout(&down);
    assert!(text.contains("http GET /invoices -> orders"), "{text}");
    assert!(!text.contains("(not in quarry)"), "{text}");
    let show = mono.w.run(&["docs", "show", "orders"]);
    assert!(
        stdout(&show).contains("consumes: http GET /invoices <- billing"),
        "{}",
        stdout(&show)
    );
}

#[test]
fn s17_check_runs_per_target() {
    let mono = monorepo();
    // billing stops producing the field orders reads.
    mono.w.write_file(
        &mono.w.source,
        "services/billing/docs/capstone/09-interfaces.md",
        &producer_page(
            "2026-09-04",
            "orders",
            "http",
            "GET /invoices",
            &[("id", "string", "yes")],
        ),
    );
    mono.w.commit_push("billing drops a field");
    assert!(mono.w.run(&["add"]).status.success());

    let out = mono.w.run(&["check"]);
    assert_eq!(code(&out), 1, "{}\n{}", stdout(&out), stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("billing produces http GET /invoices"),
        "{text}"
    );
    assert!(text.contains("break:"), "{text}");

    let value = json(&mono.w.run(&["--json", "check"]));
    let breaks = value["result"]["breaks"].as_array().expect("breaks");
    assert_eq!(breaks[0]["target"], Value::from("billing"), "{value}");
    assert_eq!(
        value["result"]["contracts"][0]["target"],
        Value::from("billing"),
        "{value}"
    );
    assert_eq!(
        value["result"]["repo"],
        Value::from("ingest-api"),
        "{value}"
    );
}

#[test]
fn s17_a_target_without_a_chapter_notes_its_docs_dir() {
    let mono = imported();
    std::fs::remove_file(
        mono.w
            .source
            .join("services/orders/docs/capstone/09-interfaces.md"),
    )
    .expect("remove chapter");

    let out = mono.w.run(&["check"]);
    assert_eq!(code(&out), 0, "{}\n{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains(
            "note: orders: no services/orders/docs/capstone/09-interfaces.md; nothing to check\n"
        ),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s17_one_configured_target_still_names_the_producer() {
    let w = world();
    w.write_files_in(
        &w.source,
        &[
            (
                "services/billing/docs/capstone/00-index.md",
                &index_page("2026-09-04"),
            ),
            (
                "services/billing/docs/capstone/09-interfaces.md",
                &producer_page(
                    "2026-09-04",
                    "orders",
                    "http",
                    "GET /invoices",
                    &[("id", "string", "yes")],
                ),
            ),
        ],
    );
    let init = w.run(&[
        "init",
        "--url",
        &w.docs_url,
        "--name",
        "billing",
        "--docs-dir",
        "services/billing/docs/capstone",
    ]);
    assert_eq!(code(&init), 0, "{}", stderr(&init));
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());

    // orders reads a field billing does not produce. There is no root index, so
    // ingest-api is not a folder in the docs repo at all.
    let orders = w.other_repo("orders");
    w.write_docs_in(
        &orders,
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &consumer_page_with(
                    "2026-09-03",
                    "billing",
                    "http",
                    "GET /invoices",
                    Some("GET /invoices"),
                    &[("id", "string", "yes"), ("total", "number", "yes")],
                ),
            ),
        ],
    );
    w.commit_push_in(&orders, "docs");
    assert!(w.run_in(&orders, &["add"]).status.success());
    assert!(w.run(&["sync"]).status.success());

    let out = w.run(&["check"]);
    assert_eq!(code(&out), 1, "{}\n{}", stdout(&out), stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("billing produces http GET /invoices"),
        "the repo name was used instead of the target: {text}"
    );

    let value = json(&w.run(&["--json", "check"]));
    assert_eq!(
        value["result"]["breaks"][0]["target"],
        Value::from("billing"),
        "{value}"
    );
    assert_eq!(
        value["result"]["contracts"][0]["target"],
        Value::from("billing"),
        "{value}"
    );
    assert_eq!(
        value["result"]["repo"],
        Value::from("ingest-api"),
        "{value}"
    );
}

#[test]
fn s17_single_repo_check_json_has_no_target() {
    let it = contract_world();
    let value = json(&it.w.run_in(&it.producer, &["--json", "check"]));
    let contracts = value["result"]["contracts"].as_array().expect("contracts");
    assert!(!contracts.is_empty(), "{value}");
    assert!(contracts[0].get("target").is_none(), "{value}");
    for one in value["result"]["breaks"].as_array().expect("breaks") {
        assert!(one.get("target").is_none(), "{value}");
    }
}

#[test]
fn s17_one_target_and_no_umbrella_still_answers_with_an_array() {
    let w = world();
    w.write_files_in(
        &w.source,
        &[(
            "services/billing/docs/capstone/00-index.md",
            &index_page("2026-09-04"),
        )],
    );
    let init = w.run(&[
        "init",
        "--url",
        &w.docs_url,
        "--name",
        "billing",
        "--docs-dir",
        "services/billing/docs/capstone",
    ]);
    assert_eq!(code(&init), 0, "{}", stderr(&init));
    w.commit_push("docs");

    let added = json(&w.run(&["--json", "add"]));
    let blocks = added["result"]
        .as_array()
        .unwrap_or_else(|| panic!("add returned an object: {added}"));
    assert_eq!(blocks.len(), 1, "{added}");
    assert_eq!(blocks[0]["repo"], Value::from("billing"), "{added}");
    assert_eq!(blocks[0]["result"], Value::from("imported"), "{added}");

    let updated = json(&w.run(&["--json", "update"]));
    let blocks = updated["result"]
        .as_array()
        .unwrap_or_else(|| panic!("update returned an object: {updated}"));
    assert_eq!(blocks.len(), 1, "{updated}");
    assert_eq!(blocks[0]["result"], Value::from("current"), "{updated}");

    let removed = json(&w.run(&["--json", "remove"]));
    let blocks = removed["result"]
        .as_array()
        .unwrap_or_else(|| panic!("remove returned an object: {removed}"));
    assert_eq!(
        blocks.len(),
        1,
        "the umbrella was never imported, so it gets no block: {removed}"
    );
    assert_eq!(blocks[0]["repo"], Value::from("billing"), "{removed}");
    assert_eq!(blocks[0]["removed"], Value::Bool(true), "{removed}");
}

#[test]
fn s17_a_target_registered_later_rewrites_the_umbrella() {
    let w = world();
    write_monorepo_files(&w);
    let first = w.run(&[
        "init",
        "--url",
        &w.docs_url,
        "--name",
        "billing",
        "--docs-dir",
        "services/billing/docs/capstone",
    ]);
    assert_eq!(code(&first), 0, "{}", stderr(&first));
    w.commit_push("docs");
    let added = w.run(&["add"]);
    assert_eq!(code(&added), 0, "{}", stderr(&added));
    let before = w.remote_file("ingest-api/00-index.md");
    assert!(before.contains("](../billing/00-index.md)"), "{before}");
    assert!(
        before.contains("](../../services/orders/docs/capstone/00-index.md#overview)"),
        "orders is not a target yet: {before}"
    );

    // No new source commit: only the target list moved.
    let second = w.run(&[
        "init",
        "--name",
        "orders",
        "--docs-dir",
        "services/orders/docs/capstone",
    ]);
    assert_eq!(code(&second), 0, "{}", stderr(&second));
    let again = w.run(&["add"]);
    assert_eq!(code(&again), 0, "{}\n{}", stdout(&again), stderr(&again));

    let page = w.remote_file("ingest-api/00-index.md");
    assert!(page.contains("](../orders/00-index.md#overview)"), "{page}");
    assert!(!page.contains("services/"), "{page}");
    assert!(
        w.remote_files().contains(&"orders/00-index.md".to_string()),
        "{:?}",
        w.remote_files()
    );

    // The list matches the clone again, so the next run leaves the umbrella be.
    let third = w.run(&["--json", "update"]);
    let value = json(&third);
    let blocks = value["result"].as_array().expect("array");
    for block in blocks {
        assert_eq!(block["result"], Value::from("current"), "{value}");
    }
}
