---
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
paths_covered:
  - ":(top)src/commands.rs"
  - ":(top)src/docsrepo.rs"
  - ":(top)src/importer.rs"
  - ":(top)src/index.rs"
  - ":(top)src/query.rs"
---
> Prescriptive design intent; code does not exist yet.

# Data flow

Three lifecycles carry the design; the rules at each hop are `logic/`'s, cited by scenario.

## Lifecycles

**`quarry update`** (`logic/01-update-ordering.md`)

1. `main` → `Context::build`: repo root from `git rev-parse --show-toplevel`; `Config` from `.quarry/.config` or env; `RepoIdentity` from `origin` (S9). Missing config → `Refusal`.
2. `commands::update`: `git.rev_parse("HEAD")`; `git.fetch(origin, default_branch)`; `git.is_ancestor(HEAD, origin/<default>)` else `Refusal` (S1 step 2).
3. `docsrepo::refresh`: `git fetch --depth 1 origin main` + `reset --hard origin/main` in the clone (S11 diverged rule).
4. `docsrepo::read_stamp` → S9 origin check → S1 steps 4–5 (`current` / skip / `is_ancestor` with on-demand `fetch <stamp>` / refuse / `--force`).
5. `importer::build(&ctx, &head)`: `git archive <sha>:<docs_dir>` piped into a tar reader, entries unpacked into a `tempfile::TempDir` under `.quarry/` (never the working tree, so uncommitted edits are excluded), permalink rewrite (S4), stamp written.
6. `docsrepo::write_folder`: `fs::remove_dir_all(<clone>/<repo>)`, `fs::rename(tmp, <clone>/<repo>)` (same filesystem).
7. `docsrepo::regenerate_root_index` (S3).
8. `docsrepo::commit_and_push`: `git add -A`, `commit -m "update <repo> @<sha>"`, `push origin HEAD:main`; on rejection reset to `origin/main`, refetch, redo from step 4 through the `redo` closure, three attempts (S1 step 10).
9. `Index::rebuild(&ctx)`; `output::render`.

**`quarry docs deps`** (`logic/08-index-freshness.md`, `logic/10-graph-traversal.md`)

1. `Context::build` (no `origin` needed for `docs *`; only the clone).
2. `Index::open_current`: read `meta`; compare `schema_version` and `built_at_commit` with `git rev-parse HEAD` in the clone; mismatch, missing, or corrupt → `Index::rebuild` (parse every page under the clone via `frontmatter::parse`, `edges_of`, `sections_of`; write a temp SQLite file; rename over the old one).
3. `query::deps`: BFS over `edges` (S7), producing `DepsResult`.
4. `output::render`: human tree or envelope; staleness line from `meta.synced_at` (S5 step 7).

**`quarry init`** (`logic/02-init-and-config.md`)

1. `config::resolve(flags, env, existing)`; prompts only when `std::io::stdin().is_terminal()`.
2. `RepoIdentity::from_origin_url`; `default_branch` from `git symbolic-ref refs/remotes/origin/HEAD`.
3. `config::write`, `config::write_gitignore`.
4. `docsrepo::ensure_clone`: `git clone --depth 1 <url> .quarry/<name>/` or `refresh` when present.

## State

| State | Location | Written by | Read by |
| --- | --- | --- | --- |
| link config | `.quarry/.config` | `config::write` (init only) | `Context::build` |
| docs clone | `.quarry/<name>/` | `docsrepo` (refresh, write_folder, regenerate_root_index, commit_and_push) | `Index::rebuild`; `query` only via the index |
| stamp | `<clone>/<repo>/.quarry-stamp` | `importer::build` (inside the temp dir) | `docsrepo::read_stamp`, `Index::rebuild` |
| index | `.quarry/.docs-index.sqlite` | `Index::rebuild` | `query` |
| temp build | `tempfile::TempDir` under `.quarry/` | `importer::build` | `docsrepo::write_folder` |

No process state survives a run; no client state exists.

## Side-effect boundaries

| IO | Confined to |
| --- | --- |
| subprocess (`git`) | `gitcmd::Git::run`, the only `std::process::Command` use in the crate (asserted by `tests/layering.rs`) |
| filesystem writes | `config` (`.quarry/.config`, `.gitignore`), `importer` (temp dir), `docsrepo` (clone tree), `index` (SQLite temp file and rename) |
| network | only through `gitcmd` (clone, fetch, push); `query` and `output` never touch it |
| stdout/stderr | `output`, and `main` for the final write |
| clock | `Context.now` |

`identity`, `frontmatter`, `query`, `output` are pure and unit-tested with data only.

## Failure paths

| Boundary | Behaviour | Scenario |
| --- | --- | --- |
| git clone/fetch/pull fails | `External`, exit 2, nothing written | S11 |
| push rejected (remote moved) | redo from stamp check, three attempts, then `External` `docs repo busy` | S1 |
| push fails for network/auth | `External` at once; clone reset | S11 |
| process killed between steps 6 and 8 of `update` | clone has uncommitted changes; next run's `refresh` resets them; `TempDir` removed on drop or at next start | S1 |
| index rebuild fails mid-way (disk full, unparsable page) | temp file removed; old index kept; unparsable pages are warnings, not failures | S5 |
| two processes rebuild the index at once | both write temp files; last rename wins; contents identical for the same HEAD | S5 |
| stdout closed | `External`, exit 2 | S13 |

There are no non-atomic multi-step writes across boundaries: the docs repo changes in one commit, the index in one rename.
