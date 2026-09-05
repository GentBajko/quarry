---
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
paths_covered:
  - ":(top)src/**"
  - ":(top)Cargo.toml"
---
> Prescriptive design intent; code does not exist yet.

# Architecture

One Rust binary crate, one process, no services. The docs repo (git) is the system of record; everything local is derived. Users run a precompiled binary; no runtime, no `cargo`.

## Layers

| Layer | Files | Uses | Purpose |
| --- | --- | --- | --- |
| entry | `src/main.rs` | `cli` | `fn main() -> ExitCode`: parse, dispatch, map `QuarryError` to exit code |
| cli | `src/cli.rs` | `commands`, `context`, `output`, `errors` | `clap` derive tree (`Cli`, `Command`, `DocsCommand`), `run(cli) -> Result<Rendered>` |
| commands | `src/commands.rs` | every core module | one `pub fn` per CLI verb (`init`, `add`, `update`, `sync`, `remove`, `docs_list`, `docs_show`, `docs_section`, `docs_search`, `docs_deps`, `docs_path`, `docs_index`) |
| core | `src/{context,config,identity,gitcmd,docsrepo,importer,frontmatter,index,query,output,errors}.rs` | std, the crates in `05-dependencies.md`, core modules per the table below | the behaviour |

Direction is downward only: nothing but `main.rs` uses `cli`; core never uses `commands`. Enforced by `tests/layering.rs`, which reads `src/*.rs` and fails on a `use crate::cli` or `use crate::commands` outside the allowed files, plus Rust's module privacy (`pub(crate)` on core, `pub` only on what `main.rs` needs).

Core dependency table (an unlisted pair is forbidden):

| Module | May use |
| --- | --- |
| `errors` | std, `thiserror` |
| `context` | `config`, `identity`, `errors` |
| `config` | `errors`, `serde_json` |
| `identity` | `errors` |
| `gitcmd` | `errors` |
| `docsrepo` | `gitcmd`, `frontmatter`, `errors`, `serde_json` |
| `importer` | `gitcmd`, `identity`, `errors`, `regex`, `tempfile` |
| `frontmatter` | `errors`, `serde-saphyr` |
| `index` | `frontmatter`, `gitcmd`, `errors`, `rusqlite`, `jiff` |
| `query` | `index`, `errors` |
| `output` | `errors`, `serde_json`, `jiff` |

Rationale for the flat core: every module has one implementation and one reason to change; traits over git or SQLite would be single-adapter seams (`code-craft.md`, Interfaces: earned).

## Module boundaries

| Module | Public surface (`pub(crate)`) | Owns |
| --- | --- | --- |
| `config` | `Config`, `load(repo_root) -> Result<Option<Config>>`, `resolve(flags, env, existing) -> Result<Config>`, `write(repo_root, &Config)`, `write_gitignore(repo_root, clone_name)` | `.quarry/.config`, `.quarry/.gitignore` (S12) |
| `identity` | `RepoIdentity`, `RepoIdentity::from_origin_url(&str) -> Result<RepoIdentity>` | S9 |
| `gitcmd` | `Git { cwd, verbose }` with `run(&[&str]) -> Result<Output>`, `rev_parse`, `fetch`, `is_ancestor`, `rev_exists`, `reset_hard`, `clone_shallow`, `commit`, `push` | every `std::process::Command` invocation; argument vectors, never a shell |
| `docsrepo` | `ensure_clone(&Context)`, `refresh(&Context) -> Result<Discarded>`, `read_stamp(&Context, &RepoName) -> Result<Option<Stamp>>`, `write_folder(&Context, &RepoName, built: TempDir)`, `regenerate_root_index(&Context)`, `commit_and_push(&Context, msg, redo: impl FnMut() -> Result<Redo>) -> Result<PushResult>` | the clone's tree and history (S1, S3, S11) |
| `importer` | `build(&Context, &Sha) -> Result<TempDir>` (archive at sha, permalink rewrite S4, stamp) | the temp-dir build |
| `frontmatter` | `parse(&str) -> Parsed { fields: Frontmatter, body: &str, warning: Option<Warning> }`, `edges_of(repo, path, &Frontmatter) -> Vec<EdgeDecl>`, `sections_of(body) -> Vec<Section>` | YAML boundary, S6 field rules, heading split (S8) |
| `index` | `Index::open_current(&Context) -> Result<Index>` (rebuild if stale, S5), `Index::rebuild(&Context, force) -> Result<RebuildReport>`, `meta()`, typed row accessors | `.quarry/.docs-index.sqlite` |
| `query` | `repos`, `files`, `show`, `section`, `search`, `deps`, `path` over `&Index` | S7, S8 read logic |
| `output` | `Rendered`, `render(&Context, Result) -> Rendered`, `envelope`, `error_envelope` | S13 |
| `errors` | `QuarryError { Refusal(String), External(String) }`, `ExitCode`, `Result<T>` | the exit-code contract |
| `context` | `Context { repo_root, identity: Option<RepoIdentity>, config, clone_path, index_path, json, verbose, now: fn() -> Timestamp }`, `Context::build(&Cli)` | wiring inputs |

## Entry points

| Entry | Planned site | Invoked by |
| --- | --- | --- |
| `quarry` binary | `src/main.rs` (`[[bin]] name = "quarry"`) | engineers, CI, agents |

No other processes, workers, or scheduled entries exist.

## Communication

| Edge | Mechanism | Registered / dispatched at |
| --- | --- | --- |
| quarry → git | `std::process::Command::new("git").args([...])` through `gitcmd::Git::run` | `src/gitcmd.rs` |
| quarry → docs repo remote | git fetch/push with the user's ssh or https credentials | `docsrepo::refresh`, `docsrepo::commit_and_push` |
| quarry → index | `rusqlite::Connection`, one per command | `index::Index::open_current` |
| quarry → caller | stdout (human or one JSON document), stderr (human errors), exit code | `output`, `main.rs` |
| subcommand dispatch | `match cli.command { Command::Init(a) => commands::init(&ctx, a), … }` | `src/cli.rs` |

No network protocol of quarry's own, no HTTP, no messaging.

## Composition

`main`: `Cli::parse()` → `Context::build(&cli)` (reads `.quarry/.config`, env vars, `origin`; `Refusal` when a command needs what is missing) → `cli::run` matches the verb to one `commands::*` function → `output::render` → `std::process::ExitCode`. `QuarryError` is matched only in `main`; every other function returns `Result`. There is no container, registry, or global state beyond `importer::HOST_TEMPLATES` (a `const` slice).

## Frontend

None. `mockup/README.md` records `surfaces: [cli]`; the `uiux` stage was skipped on that basis.
