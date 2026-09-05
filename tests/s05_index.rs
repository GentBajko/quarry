//! S5: index freshness, sync, and `docs list`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s5_list_shows_every_repo_after_a_sync() {
    let it = wired();
    let out = it.w.run(&["docs", "list"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    for repo in ["ingest-api", "record-store", "report-builder"] {
        assert!(text.contains(repo), "{text}");
    }
}

#[test]
fn s5_list_one_repo_shows_its_files_and_dates() {
    let it = wired();
    let out = it.w.run(&["docs", "list", "ingest-api"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("00-index.md"), "{text}");
    assert!(text.contains("09-interfaces.md"), "{text}");
    assert!(text.contains("2026-09-04"), "{text}");
}

#[test]
fn s5_an_unknown_repo_refuses_with_exit_one() {
    let it = wired();
    let out = it.w.run(&["docs", "list", "nope"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("unknown repo nope"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s5_queries_never_pull_but_sync_does() {
    let it = wired();
    let other = it.w.other_repo("identity-api");
    it.w.write_docs_in(&other, &[("00-index.md", &index_page("2026-09-02"))]);
    it.w.commit_push_in(&other, "docs");
    assert!(it.w.run_in(&other, &["add"]).status.success());
    let before = it.w.run(&["docs", "list"]);
    assert!(
        !stdout(&before).contains("identity-api"),
        "a query pulled: {}",
        stdout(&before)
    );
    assert!(it.w.run(&["sync"]).status.success());
    let after = it.w.run(&["docs", "list"]);
    assert!(
        stdout(&after).contains("identity-api"),
        "{}",
        stdout(&after)
    );
}

#[test]
fn s5_a_corrupt_index_is_rebuilt_without_force() {
    let it = wired();
    let index = it.w.source.join(".quarry/.docs-index.sqlite");
    std::fs::write(&index, b"not a database").expect("corrupt");
    let out = it.w.run(&["docs", "list"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("ingest-api"), "{}", stdout(&out));
}

#[test]
fn s5_index_reports_current_then_rebuilds_on_force() {
    let it = wired();
    let current = it.w.run(&["docs", "index"]);
    assert_eq!(code(&current), 0, "{}", stderr(&current));
    assert!(
        stdout(&current).contains("index current"),
        "{}",
        stdout(&current)
    );
    let forced = it.w.run(&["docs", "index", "--force"]);
    assert!(
        stdout(&forced).contains("index rebuilt"),
        "{}",
        stdout(&forced)
    );
    assert!(stdout(&forced).contains("3 repos"), "{}", stdout(&forced));
}

#[test]
fn s5_sync_pulls_another_machines_import() {
    let it = wired();
    let base = it.w.base();
    std::fs::create_dir_all(base.join("elsewhere")).expect("dir");
    let far = common::new_source(&base.join("elsewhere"), "far-service");
    assert!(
        it.w.run_in(&far, &["init", "--url", &it.w.docs_url])
            .status
            .success()
    );
    it.w.write_docs_in(&far, &[("00-index.md", &index_page("2026-09-05"))]);
    it.w.commit_push_in(&far, "docs");
    assert!(it.w.run_in(&far, &["add"]).status.success());

    let out = it.w.run(&["sync"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("docs repo: updated"),
        "{}",
        stdout(&out)
    );
    assert!(stdout(&out).contains("index: rebuilt"), "{}", stdout(&out));
    assert!(stdout(&it.w.run(&["docs", "list"])).contains("far-service"));
}

#[test]
fn s5_sync_on_an_unchanged_docs_repo_says_up_to_date() {
    let it = wired();
    let out = it.w.run(&["sync"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("up to date"), "{}", stdout(&out));
}

#[test]
fn s5_queries_work_without_the_network() {
    let it = wired();
    let remote = it.w.base().join("remotes/docs-quarry.git");
    let moved = it.w.base().join("remotes/docs-quarry.gone");
    std::fs::rename(&remote, &moved).expect("move remote");
    let out = it.w.run(&["docs", "list"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("ingest-api"), "{}", stdout(&out));
}
