//! Real git repositories in a temp dir; the binary is driven as a subprocess.

#![allow(dead_code)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

pub struct World {
    pub root: TempDir,
    pub source: PathBuf,
    pub docs_url: String,
}

/// A source repo (`ingest-api`) with a bare origin, and a bare docs repo.
pub fn world() -> World {
    let root = tempfile::tempdir().expect("tempdir");
    let base = root.path().to_path_buf();
    std::fs::create_dir_all(base.join("remotes")).expect("remotes");
    std::fs::write(base.join("gitconfig"), "").expect("gitconfig");
    bare(&base, "docs-quarry");
    let source = new_source(&base, "ingest-api");
    World {
        root,
        source,
        docs_url: url(&base, "docs-quarry"),
    }
}

fn bare(base: &Path, name: &str) {
    bare_on(base, name, "main");
}

fn bare_on(base: &Path, name: &str, branch: &str) {
    let path = base.join("remotes").join(format!("{name}.git"));
    git(
        base,
        base,
        &[
            "init",
            "--quiet",
            "--bare",
            &format!("--initial-branch={branch}"),
            &path.to_string_lossy(),
        ],
    );
}

fn url(base: &Path, name: &str) -> String {
    format!(
        "file://{}",
        base.join("remotes")
            .join(format!("{name}.git"))
            .to_string_lossy()
    )
}

/// A fresh source repo wired to its own bare origin, with one commit on main.
pub fn new_source(base: &Path, name: &str) -> PathBuf {
    new_source_on(base, name, "main")
}

/// A source repo whose default branch is whatever the caller names.
pub fn new_source_on(base: &Path, name: &str, branch: &str) -> PathBuf {
    bare_on(base, name, branch);
    let path = base.join(name);
    std::fs::create_dir_all(&path).expect("source dir");
    git(
        base,
        &path,
        &["init", "--quiet", &format!("--initial-branch={branch}")],
    );
    std::fs::write(path.join("README.md"), "# repo\n").expect("readme");
    git(base, &path, &["add", "-A"]);
    git(base, &path, &["commit", "--quiet", "-m", "init"]);
    git(base, &path, &["remote", "add", "origin", &url(base, name)]);
    git(base, &path, &["push", "--quiet", "origin", branch]);
    git(base, &path, &["remote", "set-head", "origin", "-a"]);
    path
}

impl World {
    pub fn base(&self) -> PathBuf {
        self.root.path().to_path_buf()
    }

    /// Runs the built binary in the default source repo.
    pub fn run(&self, args: &[&str]) -> Output {
        self.run_in(&self.source, args)
    }

    /// Runs the built binary in any directory.
    pub fn run_in(&self, dir: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quarry"))
            .args(args)
            .current_dir(dir)
            .envs(env(&self.base()))
            .output()
            .expect("run quarry")
    }

    /// Runs the built binary in the default source repo with extra environment.
    pub fn run_env(&self, args: &[&str], extra: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quarry"));
        command
            .args(args)
            .current_dir(&self.source)
            .envs(env(&self.base()));
        for (key, value) in extra {
            command.env(key, value);
        }
        command.output().expect("run quarry")
    }

    /// Runs git in any directory of this world.
    pub fn git(&self, dir: &Path, args: &[&str]) -> Output {
        git(&self.base(), dir, args)
    }

    /// Writes files under the source repo's docs folder.
    pub fn write_docs(&self, files: &[(&str, &str)]) {
        self.write_docs_in(&self.source, files);
    }

    /// Writes files under any source repo's docs folder.
    pub fn write_docs_in(&self, repo: &Path, files: &[(&str, &str)]) {
        for (name, body) in files {
            let path = repo.join("docs/capstone").join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("docs dir");
            }
            std::fs::write(path, body).expect("write doc");
        }
    }

    /// Commits everything in the default source repo and pushes it to main.
    pub fn commit_push(&self, message: &str) -> String {
        self.commit_push_in(&self.source, message)
    }

    /// Commits everything in a source repo and pushes it to main.
    pub fn commit_push_in(&self, repo: &Path, message: &str) -> String {
        self.git(repo, &["add", "-A"]);
        self.git(repo, &["commit", "--quiet", "-m", message]);
        let pushed = self.git(repo, &["push", "--quiet", "origin", "main"]);
        assert!(pushed.status.success(), "{}", stderr(&pushed));
        stdout(&self.git(repo, &["rev-parse", "HEAD"]))
            .trim()
            .to_string()
    }

    /// A second working clone of the docs repo, for tests that move it behind quarry's back.
    pub fn docs_clone(&self, name: &str) -> PathBuf {
        let path = self.base().join(name);
        let out = self.git(
            &self.base(),
            &["clone", "--quiet", &self.docs_url, &path.to_string_lossy()],
        );
        assert!(out.status.success(), "{}", stderr(&out));
        path
    }

    /// Another source repo, initialised against the same docs repo.
    pub fn other_repo(&self, name: &str) -> PathBuf {
        let path = new_source(&self.base(), name);
        let out = self.run_in(&path, &["init", "--url", &self.docs_url]);
        assert!(out.status.success(), "{}", stderr(&out));
        path
    }

    /// The docs repo's tree at `main`, as a sorted list of paths.
    pub fn remote_files(&self) -> Vec<String> {
        let clone = self.docs_clone(&format!("peek-{}", rand_suffix()));
        let out = self.git(&clone, &["ls-tree", "-r", "--name-only", "main"]);
        let mut files: Vec<String> = stdout(&out).lines().map(str::to_string).collect();
        files.sort();
        files
    }

    /// One file's contents in the docs repo at `main`.
    pub fn remote_file(&self, path: &str) -> String {
        let clone = self.docs_clone(&format!("peek-{}", rand_suffix()));
        stdout(&self.git(&clone, &["show", &format!("main:{path}")]))
    }

    /// Writes one file under any source repo, creating parent directories.
    pub fn write_file(&self, repo: &Path, path: &str, body: &str) {
        let full = repo.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("file dir");
        }
        std::fs::write(full, body).expect("write file");
    }

    /// Writes files at arbitrary repo-relative paths in any source repo.
    pub fn write_files_in(&self, repo: &Path, files: &[(&str, &str)]) {
        for (path, body) in files {
            self.write_file(repo, path, body);
        }
    }

    /// One repo folder's stamp in the docs repo at `main`.
    pub fn remote_stamp(&self, repo: &str) -> String {
        self.remote_file(&format!("{repo}/.quarry-stamp"))
    }

    /// Commits one file at the docs repo root through a second clone and pushes
    /// it, the way a traffic exporter would; call `sync` afterwards.
    pub fn push_docs_root_file(&self, name: &str, body: &str) {
        let clone = self.docs_clone(&format!("writer-{}", rand_suffix()));
        std::fs::write(clone.join(name), body).expect("write root file");
        self.git(&clone, &["add", "-A"]);
        self.git(
            &clone,
            &["commit", "--quiet", "-m", &format!("export {name}")],
        );
        let pushed = self.git(&clone, &["push", "--quiet", "origin", "HEAD:main"]);
        assert!(pushed.status.success(), "{}", stderr(&pushed));
    }
}

/// An observed-edges.json body; each row is (from, to, kind, name, last_seen).
/// An empty `generated_at` or `last_seen` leaves the key out.
pub fn observed_file(generated_at: &str, rows: &[(&str, &str, &str, &str, &str)]) -> String {
    let edges: Vec<serde_json::Value> = rows
        .iter()
        .map(|(from, to, kind, name, seen)| {
            let mut row = serde_json::json!({"from": from, "to": to, "kind": kind, "name": name});
            if !seen.is_empty() {
                row["last_seen"] = serde_json::json!(seen);
            }
            row
        })
        .collect();
    let mut file = serde_json::json!({ "edges": edges });
    if !generated_at.is_empty() {
        file["generated_at"] = serde_json::json!(generated_at);
    }
    file.to_string()
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{nanos}")
}

fn env(base: &Path) -> Vec<(String, String)> {
    let config = base.join("gitconfig").to_string_lossy().to_string();
    vec![
        ("GIT_AUTHOR_NAME".into(), "quarry test".into()),
        ("GIT_AUTHOR_EMAIL".into(), "test@example.invalid".into()),
        ("GIT_COMMITTER_NAME".into(), "quarry test".into()),
        ("GIT_COMMITTER_EMAIL".into(), "test@example.invalid".into()),
        ("GIT_AUTHOR_DATE".into(), "2026-09-05T00:00:00+00:00".into()),
        (
            "GIT_COMMITTER_DATE".into(),
            "2026-09-05T00:00:00+00:00".into(),
        ),
        ("GIT_CONFIG_GLOBAL".into(), config.clone()),
        ("GIT_CONFIG_SYSTEM".into(), config),
        ("GIT_TERMINAL_PROMPT".into(), "0".into()),
    ]
}

fn git(base: &Path, dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .envs(env(base))
        .output()
        .expect("run git")
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

pub fn code(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

/// A minimal Capstone-shaped index page.
pub fn index_page(date: &str) -> String {
    format!(
        "---\ngenerated_date: {date}\ncapstone_version: 5.2.1\n---\n\n# Overview\n\nWhat this repo is.\n"
    )
}

/// A page declaring one produced edge.
pub fn produces_page(date: &str, to: &str, kind: &str, name: &str) -> String {
    format!(
        "---\ngenerated_date: {date}\nproduces:\n  - kind: {kind}\n    name: {name}\n    to: {to}\n---\n\n## Produces\n\n| Kind | Name |\n|---|---|\n| {kind} | {name} |\n"
    )
}

/// A produces page whose frontmatter entry carries a site.
pub fn produces_page_with_site(date: &str, to: &str, kind: &str, name: &str, site: &str) -> String {
    format!(
        "---\ngenerated_date: {date}\nproduces:\n  - kind: {kind}\n    name: {name}\n    to: {to}\n    site: {site}\n---\n\n## Produces\n\n| Kind | Name |\n|---|---|\n| {kind} | {name} |\n"
    )
}

/// A table-only interfaces page with one produces row and a Site cell.
pub fn produces_table_page(date: &str, to: &str, kind: &str, name: &str, site: &str) -> String {
    format!(
        "---\ngenerated_date: {date}\n---\n\n## Produces\n\n| Kind | Name | To | Site |\n|---|---|---|---|\n| {kind} | {name} | {to} | `{site}` |\n"
    )
}

/// A consumes page whose frontmatter entry carries a client site.
pub fn consumes_page_with_site(
    date: &str,
    from: &str,
    kind: &str,
    name: &str,
    site: &str,
) -> String {
    format!(
        "---\ngenerated_date: {date}\nconsumes:\n  - kind: {kind}\n    name: {name}\n    from: {from}\n    site: {site}\n---\n\n## Consumes\n\n### {name} (v2)\n\n| Field | Type |\n|---|---|\n| file_id | string |\n"
    )
}

/// A page declaring one consumed edge, with the contract inline.
pub fn consumes_page(date: &str, from: &str, kind: &str, name: &str) -> String {
    format!(
        "---\ngenerated_date: {date}\nconsumes:\n  - kind: {kind}\n    name: {name}\n    from: {from}\n---\n\n## Consumes\n\n### {name} (v2)\n\n| Field | Type |\n|---|---|\n| file_id | string |\n"
    )
}

/// Pattern-shaped, never valid: the checksum bytes are all 'a'.
pub const FAKE_GITHUB_TOKEN: &str = "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// The header line alone; no key body follows it anywhere in the tests.
pub const FAKE_PRIVATE_KEY_HEADER: &str = "-----BEGIN RSA PRIVATE KEY-----";
/// Pattern-shaped, never valid: sixteen 'a' where the key bytes go.
pub const FAKE_STRIPE_KEY: &str = "sk_test_aaaaaaaaaaaaaaaa";

/// An operations chapter quoting one configuration value verbatim.
pub fn operations_page(date: &str, value: &str) -> String {
    format!(
        "---\ngenerated_date: {date}\n---\n\n# Operations\n\n## Configuration\n\n| Name | Default |\n|---|---|\n| GITHUB_TOKEN | {value} |\n"
    )
}

/// A page declaring one produced contract whose consumers are unknown.
pub fn produces_unknown_page(date: &str, kind: &str, name: &str) -> String {
    produces_page(date, "unknown", kind, name)
}

/// A consumes page that also lists the names this repo is known by.
pub fn aliased_consumes_page(
    date: &str,
    aliases: &[&str],
    from: &str,
    kind: &str,
    name: &str,
) -> String {
    let list = alias_list(aliases);
    format!(
        "---\ngenerated_date: {date}\nknown_as:\n{list}consumes:\n  - kind: {kind}\n    name: {name}\n    from: {from}\n---\n\n## Consumes\n\n### {name} (v2)\n\n| Field | Type |\n|---|---|\n| file_id | string |\n"
    )
}

/// A page whose frontmatter lists the names this repo answers to and declares
/// no edges.
pub fn known_as_page(date: &str, aliases: &[&str]) -> String {
    let list = alias_list(aliases);
    format!(
        "---\ngenerated_date: {date}\nknown_as:\n{list}---\n\n# Overview\n\nWhat this repo is.\n"
    )
}

fn alias_list(aliases: &[&str]) -> String {
    aliases.iter().map(|a| format!("  - {a}\n")).collect()
}

/// An interfaces page that declares one produced edge and separately mentions
/// another repo's contract by name.
pub fn produces_and_mentions_page(
    date: &str,
    to: &str,
    kind: &str,
    name: &str,
    mentions: &str,
) -> String {
    format!(
        "---\ngenerated_date: {date}\nproduces:\n  - kind: {kind}\n    name: {name}\n    to: {to}\n---\n\n## Produces\n\n| Kind | Name |\n|---|---|\n| {kind} | {name} |\n\n## Consumes\n\n### {mentions} (v2)\n\n| Field | Type |\n|---|---|\n| id | string |\n"
    )
}

/// The three fields report-builder reads from GET /records.
pub const RECORD_FIELDS: &[(&str, &str, &str)] = &[
    ("id", "string", "yes"),
    ("created_at", "string", "yes"),
    ("content_type", "enum", "yes"),
];

/// A `| Field | Type | Required |` table.
pub fn payload_table(fields: &[(&str, &str, &str)]) -> String {
    let mut text = "| Field | Type | Required |\n|---|---|---|\n".to_string();
    for (name, ty, required) in fields {
        text.push_str(&format!("| {name} | {ty} | {required} |\n"));
    }
    text
}

/// A producer page with its payload section; no section when `fields` is empty.
pub fn producer_page(
    date: &str,
    to: &str,
    kind: &str,
    name: &str,
    fields: &[(&str, &str, &str)],
) -> String {
    let mut text = format!(
        "---\ngenerated_date: {date}\n---\n\n## Produces\n\n| Kind | Name | To |\n|---|---|---|\n| {kind} | {name} | [{to}](../{to}/09-interfaces.md) |\n"
    );
    if !fields.is_empty() {
        text.push_str(&format!("\n### {name}\n\n{}", payload_table(fields)));
    }
    text
}

/// A consumer page. `heading` None means no contract section at all; a heading
/// with no fields gets a section holding prose, which is the "lists no fields"
/// case.
pub fn consumer_page_with(
    date: &str,
    from: &str,
    kind: &str,
    name: &str,
    heading: Option<&str>,
    fields: &[(&str, &str, &str)],
) -> String {
    let mut text = format!(
        "---\ngenerated_date: {date}\n---\n\n## Consumes\n\n| Kind | Name | From |\n|---|---|---|\n| {kind} | {name} | [{from}](../{from}/09-interfaces.md) |\n"
    );
    if let Some(heading) = heading {
        let body = if fields.is_empty() {
            "Still being written.\n".to_string()
        } else {
            payload_table(fields)
        };
        text.push_str(&format!("\n### {heading}\n\n{body}"));
    }
    text
}

pub struct Contracts {
    pub w: World,
    pub producer: PathBuf,
    pub consumer: PathBuf,
}

/// record-store produces http GET /records for report-builder, both registered
/// and synced, with matching payload tables; the default ingest-api source is
/// unused.
pub fn contract_world() -> Contracts {
    let w = world();
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
                    RECORD_FIELDS,
                ),
            ),
        ],
    );
    w.commit_push_in(&producer, "docs");
    assert!(w.run_in(&producer, &["add"]).status.success());
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
    assert!(w.run_in(&producer, &["sync"]).status.success());
    Contracts {
        w,
        producer,
        consumer,
    }
}

/// A root index-of-indexes linking each workspace's docs index, as Capstone's
/// `map` writes it; each link is given as (workspace, target).
pub fn umbrella_index(date: &str, links: &[(&str, &str)]) -> String {
    let mut text = format!(
        "---\ngenerated_date: {date}\n---\n\n# Workspaces\n\n| Workspace | Index |\n|---|---|\n"
    );
    for (name, target) in links {
        text.push_str(&format!("| {name} | [{name}]({target}) |\n"));
    }
    text
}

pub struct Mono {
    pub w: World,
    pub head: String,
}

/// The two workspaces and the root index-of-indexes `monorepo()` builds, for a
/// test that wants to register the targets in its own order.
pub fn write_monorepo_files(w: &World) {
    w.write_files_in(
        &w.source,
        &[
            (
                "services/billing/docs/capstone/00-index.md",
                &index_page("2026-09-04"),
            ),
            (
                "services/billing/docs/capstone/09-interfaces.md",
                &produces_page("2026-09-04", "orders", "http", "GET /invoices"),
            ),
            (
                "services/orders/docs/capstone/00-index.md",
                &index_page("2026-09-03"),
            ),
            (
                "services/orders/docs/capstone/09-interfaces.md",
                &consumes_page("2026-09-03", "billing", "http", "GET /invoices"),
            ),
            (
                "docs/capstone/00-index.md",
                &umbrella_index(
                    "2026-09-04",
                    &[
                        (
                            "billing",
                            "../../services/billing/docs/capstone/00-index.md",
                        ),
                        (
                            "orders",
                            "../../services/orders/docs/capstone/00-index.md#overview",
                        ),
                    ],
                ),
            ),
        ],
    );
}

/// One source repo (`ingest-api`) with two workspaces registered as targets and
/// a root index-of-indexes. Registered but not imported.
pub fn monorepo() -> Mono {
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
    let second = w.run(&[
        "init",
        "--name",
        "orders",
        "--docs-dir",
        "services/orders/docs/capstone",
    ]);
    assert_eq!(code(&second), 0, "{}", stderr(&second));
    let head = w.commit_push("docs");
    Mono { w, head }
}

pub struct Wired {
    pub w: World,
    pub data: PathBuf,
    pub report_builder: PathBuf,
}

/// Three repos in the docs repo: ingest-api -> record-store -> report-builder.
pub fn wired() -> Wired {
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
                "---\ngenerated_date: 2026-09-03\nconsumes:\n  - kind: sqs\n    name: file-ingest\n    from: ingest-api\nproduces:\n  - kind: http\n    name: GET /records\n    to: report-builder\n---\n\n## Consumes\n\n### file-ingest (v2)\n\n| Field | Type | Required |\n|---|---|---|\n| file_id | string | yes |\n| content_type | enum | yes |\n",
            ),
        ],
    );
    w.commit_push_in(&data, "docs");
    assert!(w.run_in(&data, &["add"]).status.success());

    let report_builder = w.other_repo("report-builder");
    w.write_docs_in(
        &report_builder,
        &[
            ("00-index.md", &index_page("2026-09-01")),
            (
                "09-interfaces.md",
                &consumes_page("2026-09-01", "record-store", "http", "GET /records"),
            ),
        ],
    );
    w.commit_push_in(&report_builder, "docs");
    assert!(w.run_in(&report_builder, &["add"]).status.success());
    assert!(w.run(&["sync"]).status.success());
    Wired {
        w,
        data,
        report_builder,
    }
}
