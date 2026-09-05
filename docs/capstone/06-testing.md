---
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
paths_covered:
  - ":(top)tests/**"
  - ":(top)src/**"
  - ":(top)Cargo.toml"
---
> Prescriptive design intent; code does not exist yet.

# Testing

## Layout

Unit tests inline under `#[cfg(test)] mod tests` in each `src/*.rs` (pure modules: `identity`, `frontmatter`, permalink rewrite, `query`, `output`). Integration tests in `tests/`, one file per logic scenario (`tests/s01_update_ordering.rs` … `tests/s13_output.rs`) plus `tests/layering.rs`, sharing `tests/common/mod.rs` (fixtures: `source_repo()`, `docs_remote()` bare repo, `quarry()` command builder with `GIT_*` env). Run: `cargo test`. Lint: `cargo clippy --all-targets -- -D warnings`. Format: `cargo fmt --check`.

## Doubles

None for git or SQLite: integration tests create real repositories in `tempfile::tempdir()` (`git init` a source repo with commits on `main`, `git init --bare` a docs remote, `quarry init --url <bare path>`) and drive the built binary via `std::process::Command::new(env!("CARGO_BIN_EXE_quarry"))`, which is code-craft's category 2, local-substitutable. Concurrency cases (S1 push races) are reproduced by advancing the bare remote from a second clone between steps. Determinism seams: `GIT_AUTHOR_NAME/EMAIL`, `GIT_COMMITTER_NAME/EMAIL`, `GIT_AUTHOR_DATE/COMMITTER_DATE` set by the fixture; `Context.now` is a `fn` pointer replaced in unit tests, and integration tests set `QUARRY_TEST_NOW` (read only in test builds via `cfg!(test)`-gated code, never in release binaries).

## Coverage shape

Every rule in a `logic/` scenario has at least one test named after it (`s1_never_go_backwards`, `s4_pointer_outside_tree_untouched`). No numeric coverage threshold. No load, chaos, or security suites; the Q19 performance measures are checked by one `#[ignore]`d test that builds a 5,000-page clone and asserts rebuild ≤ 10 s, run in CI on `main` with `cargo test -- --ignored`.
