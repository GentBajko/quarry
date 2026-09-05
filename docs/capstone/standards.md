---
generated_date: 2026-09-05
capstone_version: 5.2.1
readback: a463e52ce97d
revised: 2026-09-05
---

> Standards the user set: binding, not a description of current code.

# Standards

These rules outrank generic best practice and `code-craft.md` wherever they speak; `code-craft.md` governs what they leave open. `03-conventions.md` describes what the code does; where the two disagree, the code is wrong.

## Typing

- Compile with `#![deny(unsafe_code)]` and `#![warn(missing_docs)]` at the crate root; CI treats warnings as errors.
- Deny `clippy::unwrap_used` and `clippy::expect_used` in `src/`; allow them in tests only.
- Wrap domain strings in newtypes: `RepoName`, `Sha`, `NormalizedOrigin`. Never pass a bare `String` where one of them fits.
- Use enums for every closed set (`ExitCode`, `DeclaredBy`, `Direction`, `UpdateResult`). Never compare against string literals for a closed set.
- No `as` casts on user-derived numbers; use `TryFrom`.
- `serde_json::Value` is the one dynamically typed value, confined to frontmatter storage in `frontmatter` and `index`.

## Libraries

- The ladder in `code-craft.md` applies: std before a crate; never a new crate for what a few lines do.
- The crate list in `05-dependencies.md` is the budget. Adding a crate requires a `changelog.md` entry naming the ladder rung it climbs.
- Vetting bar for any crate: permissive licence (MIT, Apache-2.0, BSD, Unlicense), a release within the last 12 months, no `unsafe` in its public API path that quarry exercises unless the crate is `rusqlite`/`libsqlite3-sys`.
- Hand-roll frontmatter splitting, heading parsing, permalink rewriting, the JSON envelope shape, tar entry reading from `git archive`. Never hand-roll YAML parsing, SQLite access, git, or date formatting.

## Paradigm

- Write procedural modules: free functions over plain structs and enums deriving `Debug, Clone, PartialEq, Eq`.
- Define no traits of quarry's own; `Index` is the only struct with methods beyond constructors.
- No generics except closure parameters (`impl FnMut`).
- No statics with interior mutability. `importer::HOST_TEMPLATES` is the one module-level `const`.
- Pass dependencies as explicit parameters; `Context` is the one struct passed down.

## Error handling

- Return `Result<T, QuarryError>` from every fallible function; `QuarryError` is a `thiserror` enum with `Refusal(String)` and `External(String)`.
- Match on the variant only in `main`, to choose the exit code and stream.
- Never panic on input: no `unwrap`, `expect`, `panic!`, `unreachable!` on any path a user or a repo can reach.
- Carry git's stderr verbatim in the `External` message.
- Return warnings as values (`Vec<Warning>`); never print from a core module.
- Do not log. `--verbose` echoes git commands to stderr from `gitcmd::Git::run` only.
- Never discard a `Result` (`clippy::let_underscore_must_use` denied); never swallow an IO, SQLite, or subprocess error.

## Organization

- Lay the crate out as `01-architecture.md` names it: one flat `src/` with one file per module. Split a module into a directory only when it passes 500 lines, and then by reason for change.
- Name modules and functions `snake_case`, types `PascalCase`, consts `UPPER_SNAKE`. Name each command function exactly as its CLI verb with underscores (`docs_deps`).
- Comments follow `code-craft.md`'s Comments section without change.
- Do not write doc comments that restate a signature or cite a scenario id; the function name and the `logic/` file already carry that. A comment earns its line only by saying what the code cannot: a constraint, a workaround, or a rejected alternative. `///` on a clap struct or field is user-facing help text, not a comment, and stays.

## Testing

- Follow the TDD cycle in `code-craft.md`: failing test, minimum code to green, verify, commit, per task.
- Unit tests live inline under `#[cfg(test)]` in pure modules; integration tests in `tests/` drive the built binary via `env!("CARGO_BIN_EXE_quarry")` against real git repositories and a bare docs remote in `tempfile::tempdir()`.
- Never add a mocking crate; never fake git or SQLite.
- Write one test per rule in a `logic/` scenario, named `s<N>_<rule>`.
- No coverage threshold. A new branch in a core module without a test is a review finding.
- Mark the 5,000-page performance test `#[ignore]`; CI on `main` runs `cargo test -- --ignored`.

## Tooling

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass before any commit.
- Stable toolchain; `rust-version` in `Cargo.toml` is the stable release current at each quarry release; edition 2024.
- CI runs all three on Linux, macOS, and Windows for every PR; release builds come from `cargo-dist`'s generated workflow, never hand-edited.

## Process

- Commits: `<type>(<scope>): <subject>`, imperative, ≤ 72 characters, body says why; one reviewable idea per commit; red and green of a TDD cycle land together. Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`.
- Branches: `<type>/<slug>`. Never commit to `main` without the user's explicit consent.
- Never add attribution, co-author, or "generated by" trailers to commits or PR descriptions.
- PR title = the branch's main commit subject; PR body = why, plus the `logic/` scenario ids touched.
- Releases: tag `vX.Y.Z` on `main`; `Cargo.toml` version bumped in the same commit as the changelog line.

## Agent rules

An AI assistant working in this repository must:

- Read the `logic/` scenario before changing the module that implements it.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` before reporting a task done, and report their output faithfully.
- Keep `docs/capstone/` current through Capstone (`map` after code changes); never hand-edit generated chapters.
- Use Serena for symbol navigation and Context7 for crate documentation before writing against an external API.

It must never:

- Add a crate on its own; only the user's recorded decision (§ Libraries) does.
- Write `unsafe`.
- Mock git or SQLite.
- Pass a user-supplied string through a shell (`sh -c`, `cmd /C`, string-built commands).
- Write outside `.quarry/` and the docs repo folder quarry owns, in code or in tests.
- Rewrite, amend, or force-push the docs repo's history.
- Add attribution trailers to commits.
