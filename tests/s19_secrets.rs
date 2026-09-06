//! S19: the secret scan on the import path, and the workflow templates.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::{Path, PathBuf};

use common::*;
use serde_json::Value;

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

fn head_of(w: &World, repo: &Path) -> String {
    stdout(&w.git(repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string()
}

fn clone_path(w: &World) -> PathBuf {
    w.source.join(".quarry/docs-quarry")
}

/// ingest-api registered from a clean index page, nothing else imported yet.
fn registered() -> World {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-05"))]);
    w.commit_push("docs");
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    w
}

/// Adds an operations chapter quoting `value` and pushes it.
fn leak(w: &World, value: &str) {
    w.write_docs(&[("07-operations.md", &operations_page("2026-09-05", value))]);
    w.commit_push("operations");
}

#[test]
fn s19_strict_update_refuses_a_page_with_a_secret() {
    let w = registered();
    let stamped = w.remote_stamp("ingest-api");
    leak(&w, FAKE_GITHUB_TOKEN);
    let out = w.run(&["update", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("docs/capstone/07-operations.md contains a github-token"),
        "{err}"
    );
    assert!(
        err.contains("remove them from the docs before importing"),
        "{err}"
    );
    assert!(!err.contains(FAKE_GITHUB_TOKEN), "{err}");
    assert!(
        !w.remote_files()
            .contains(&"ingest-api/07-operations.md".to_string()),
        "{:?}",
        w.remote_files()
    );
    assert_eq!(w.remote_stamp("ingest-api"), stamped);
}

#[test]
fn s19_strict_add_refuses_before_the_first_import() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-05")),
        (
            "07-operations.md",
            &operations_page("2026-09-05", FAKE_PRIVATE_KEY_HEADER),
        ),
    ]);
    w.commit_push("docs");
    let out = w.run(&["add", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("docs/capstone/07-operations.md contains a private-key"),
        "{}",
        stderr(&out)
    );
    let files = w.remote_files();
    assert!(
        files.iter().all(|f| !f.starts_with("ingest-api/")),
        "{files:?}"
    );
    assert!(!files.contains(&"00-index.md".to_string()), "{files:?}");
}

#[test]
fn s19_non_strict_update_imports_with_a_note() {
    let w = registered();
    leak(&w, FAKE_GITHUB_TOKEN);
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("docs/capstone/07-operations.md contains a github-token"),
        "{text}"
    );
    assert!(text.contains("files imported"), "{text}");
    assert!(
        w.remote_file("ingest-api/07-operations.md")
            .contains(FAKE_GITHUB_TOKEN)
    );
}

#[test]
fn s19_json_note_names_the_pattern_never_the_text() {
    let w = registered();
    leak(&w, FAKE_GITHUB_TOKEN);
    let out = w.run(&["--json", "update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let value = json(&out);
    let notes = value["result"]["notes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        notes.contains(&Value::String(
            "docs/capstone/07-operations.md contains a github-token".to_string()
        )),
        "{notes:?}"
    );
    assert!(
        !stdout(&out).contains(FAKE_GITHUB_TOKEN),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s19_a_json_strict_refusal_is_one_envelope() {
    let w = registered();
    leak(&w, FAKE_GITHUB_TOKEN);
    let out = w.run(&["--json", "update", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let value = json(&out);
    assert_eq!(value["ok"], Value::Bool(false));
    assert_eq!(value["code"], Value::from(1));
    assert!(
        value["error"]
            .as_str()
            .unwrap_or_default()
            .contains("docs/capstone/07-operations.md contains a github-token"),
        "{value}"
    );
    assert!(
        !stdout(&out).contains(FAKE_GITHUB_TOKEN),
        "{}",
        stdout(&out)
    );
    assert_eq!(stderr(&out), "");
}

#[test]
fn s19_non_markdown_files_are_scanned() {
    let w = registered();
    w.write_docs(&[("example.env", &format!("STRIPE={FAKE_STRIPE_KEY}\n"))]);
    w.commit_push("example env");
    let out = w.run(&["update", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("docs/capstone/example.env contains a stripe-key"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s19_strict_with_a_clean_tree_imports() {
    let w = registered();
    leak(&w, "<redacted>");
    let out = w.run(&["update", "--strict"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(!stdout(&out).contains("contains a"), "{}", stdout(&out));
    assert!(
        w.remote_files()
            .contains(&"ingest-api/07-operations.md".to_string())
    );
}

#[test]
fn s19_two_patterns_in_one_file_are_two_lines() {
    let w = registered();
    leak(
        &w,
        &format!("{FAKE_GITHUB_TOKEN} then {FAKE_PRIVATE_KEY_HEADER}"),
    );
    let out = w.run(&["update", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let err = stderr(&out);
    let token = err
        .find("docs/capstone/07-operations.md contains a github-token")
        .unwrap_or_else(|| panic!("{err}"));
    let key = err
        .find("docs/capstone/07-operations.md contains a private-key")
        .unwrap_or_else(|| panic!("{err}"));
    assert!(token < key, "{err}");
}

#[test]
fn s19_a_secret_in_the_second_target_leaves_the_first_unwritten() {
    let Mono { w, .. } = monorepo();
    assert_eq!(code(&w.run(&["add"])), 0);
    let before = head_of(&w, &clone_path(&w));
    let billing_index = w.remote_file("billing/00-index.md");

    // The secret is in the second target; the first target's own change must
    // not reach the docs repo either.
    w.write_files_in(
        &w.source,
        &[
            (
                "services/billing/docs/capstone/00-index.md",
                &index_page("2026-09-06"),
            ),
            (
                "services/orders/docs/capstone/07-operations.md",
                &operations_page("2026-09-06", FAKE_GITHUB_TOKEN),
            ),
        ],
    );
    w.commit_push("docs");
    let out = w.run(&["update", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out)
            .contains("services/orders/docs/capstone/07-operations.md contains a github-token"),
        "{}",
        stderr(&out)
    );
    assert_eq!(w.remote_file("billing/00-index.md"), billing_index);
    assert!(
        !w.remote_files()
            .contains(&"orders/07-operations.md".to_string())
    );
    assert_eq!(head_of(&w, &clone_path(&w)), before);
    assert_eq!(
        stdout(&w.git(&clone_path(&w), &["status", "--porcelain"])),
        ""
    );

    // Both targets leaking lists both hits, in target order.
    w.write_files_in(
        &w.source,
        &[(
            "services/billing/docs/capstone/07-operations.md",
            &operations_page("2026-09-06", FAKE_PRIVATE_KEY_HEADER),
        )],
    );
    w.commit_push("docs");
    let out = w.run(&["update", "--strict"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    let err = stderr(&out);
    let billing = err
        .find("services/billing/docs/capstone/07-operations.md contains a private-key")
        .unwrap_or_else(|| panic!("{err}"));
    let orders = err
        .find("services/orders/docs/capstone/07-operations.md contains a github-token")
        .unwrap_or_else(|| panic!("{err}"));
    assert!(billing < orders, "{err}");
    assert_eq!(head_of(&w, &clone_path(&w)), before);
}

fn template(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("templates")
        .join(name);
    let text = std::fs::read_to_string(&path).expect("template");
    serde_saphyr::from_str::<Value>(&text).expect("template is not valid YAML")
}

// The reusable workflow is the whole of a repo's adoption, so its call surface
// is pinned here rather than discovered by a failing job in someone's CI.
#[test]
fn s19_the_update_template_takes_the_documented_inputs_and_secrets() {
    let update = template("quarry-update.yml");
    let call = update
        .get("on")
        .and_then(|on| on.get("workflow_call"))
        .unwrap_or_else(|| panic!("no workflow_call in {update}"));
    for input in ["docs-repo", "quarry-version", "strict"] {
        assert!(call["inputs"].get(input).is_some(), "no input {input}");
    }
    assert_eq!(call["inputs"]["strict"]["default"], Value::Bool(true));
    // The pinned tag is spelled in three places: this assertion, the
    // template's `default:`, and README's "defaults to `v0.1.0`" sentence.
    // The version reconciliation step moves all three to v0.2.0 together.
    assert_eq!(
        call["inputs"]["quarry-version"]["default"],
        Value::String("v0.1.0".to_string())
    );
    for secret in ["app-id", "app-private-key"] {
        assert!(call["secrets"].get(secret).is_some(), "no secret {secret}");
    }
    assert!(update["jobs"]["update"].is_object(), "{update}");
}

#[test]
fn s19_the_audit_template_runs_both_jobs() {
    let audit = template("quarry-audit.yml");
    for job in ["committer-folders", "unregistered-repos"] {
        assert!(audit["jobs"].get(job).is_some(), "no job {job}");
    }
}

/// Runs git in an audit fixture as `who`, with the ambient git config ignored.
fn audit_git(dir: &Path, who: &str, args: &[&str]) -> std::process::Output {
    let empty = dir.join("../gitconfig");
    std::fs::write(&empty, "").expect("gitconfig");
    std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", who)
        .env("GIT_COMMITTER_NAME", who)
        .env("GIT_AUTHOR_EMAIL", "audit@example.invalid")
        .env("GIT_COMMITTER_EMAIL", "audit@example.invalid")
        .env("GIT_CONFIG_GLOBAL", &empty)
        .env("GIT_CONFIG_SYSTEM", &empty)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run git")
}

/// Lays down a docs repo and commits `folders` (path, body) as `who`.
fn audit_commit(dir: &Path, who: &str, folders: &[(&str, &str)]) {
    for (path, body) in folders {
        let file = dir.join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).expect("fixture dir");
        }
        std::fs::write(file, body).expect("fixture file");
    }
    assert!(audit_git(dir, who, &["add", "-A"]).status.success());
    let out = audit_git(dir, who, &["commit", "-m", "import"]);
    assert!(out.status.success(), "{}", stderr(&out));
}

fn stamp(origin: &str, docs_dir: &str) -> String {
    format!("{{\"commit\":\"0000000\",\"origin\":\"{origin}\",\"docs_dir\":\"{docs_dir}\"}}\n")
}

/// Extracts job 1's script from the template and runs it over `repo`.
fn run_audit_job1(work: &Path, repo: &Path) -> std::process::Output {
    let audit = template("quarry-audit.yml");
    let script = audit["jobs"]["committer-folders"]["steps"]
        .as_array()
        .and_then(|steps| steps.iter().find_map(|step| step.get("run")))
        .and_then(Value::as_str)
        .expect("committer-folders has a run step")
        .to_string();
    let path = work.join("job1.sh");
    std::fs::write(&path, script).expect("write job1.sh");
    std::process::Command::new("bash")
        .arg("-e")
        .arg(&path)
        .current_dir(repo)
        .env("RUNNER_TEMP", work)
        .env("GITHUB_STEP_SUMMARY", work.join("summary.md"))
        .env("SINCE", "24 hours ago")
        .output()
        .expect("run bash")
}

// The audit is a shell script no Rust path calls, so it is executed here
// rather than read. The origin's last segment is lowercased by
// `identity::origin` while GIT_COMMITTER_NAME carries the repo's own case, so
// a mixed-case monorepo is the case an all-lowercase fixture cannot see.
#[test]
fn s19_the_audit_allows_a_mixed_case_monorepo() {
    let work = tempfile::tempdir().expect("tempdir");
    let repo = work.path().join("docs");
    std::fs::create_dir_all(&repo).expect("repo dir");
    assert!(
        audit_git(&repo, "setup", &["init", "-q", "."])
            .status
            .success()
    );
    audit_commit(
        &repo,
        "Acme-Platform",
        &[
            ("00-index.md", "# index\n"),
            (
                "billing/.quarry-stamp",
                &stamp(
                    "github.com/acme/acme-platform",
                    "services/billing/docs/capstone",
                ),
            ),
            ("billing/00-index.md", "# billing\n"),
            (
                "Acme-Platform/.quarry-stamp",
                &stamp("github.com/acme/acme-platform", "docs/capstone"),
            ),
            ("Acme-Platform/00-index.md", "# platform\n"),
        ],
    );
    let out = run_audit_job1(work.path(), &repo);
    assert_eq!(
        code(&out),
        0,
        "{}{}",
        stdout(&out),
        String::from_utf8_lossy(&out.stderr)
    );
}

// The case fold must not widen the match: H9 replaced a regex that let
// `docsXsite` claim `docs.site/`, and the literal comparison still holds.
#[test]
fn s19_the_audit_still_flags_a_near_miss_folder_name() {
    let work = tempfile::tempdir().expect("tempdir");
    let repo = work.path().join("docs");
    std::fs::create_dir_all(&repo).expect("repo dir");
    assert!(
        audit_git(&repo, "setup", &["init", "-q", "."])
            .status
            .success()
    );
    audit_commit(&repo, "docsXsite", &[("00-index.md", "# index\n")]);
    audit_commit(
        &repo,
        "docsXsite",
        &[
            (
                "docs.site/.quarry-stamp",
                &stamp("github.com/acme/docs.site", "docs/capstone"),
            ),
            ("docs.site/00-index.md", "# site\n"),
        ],
    );
    let out = run_audit_job1(work.path(), &repo);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stdout(&out).contains("(docsXsite) touched docs.site/00-index.md"),
        "{}",
        stdout(&out)
    );
}
