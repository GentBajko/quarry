---
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
paths_covered:
  - ":(top)src/**"
  - ":(top)Cargo.toml"
---
> Prescriptive design intent; code does not exist yet.

# Conventions

The binding rules are in `standards.md`; this chapter records the structural conventions the architecture assumes.

## Paradigm

Procedural Rust: modules expose functions; data travels as plain structs and enums; newtypes for domain strings (`RepoName`, `Sha`, `NormalizedOrigin`). No traits of quarry's own except `Display`/`Error` on `QuarryError`; no generics beyond what closures need (`commit_and_push`'s `redo`). `Index` is the one struct with methods, wrapping a `rusqlite::Connection`.

## Typing

`#![deny(unsafe_code)]`, `#![warn(missing_docs)]` on the crate; clippy with `-D warnings`, `clippy::unwrap_used` and `clippy::expect_used` denied in `src/` (allowed in tests). No `as` casts on user data; `TryFrom` for sizes. Enums: `ExitCode`, `DeclaredBy`, `Direction` (`Downstream | Upstream`), `UpdateResult` (`Imported | Current | Skipped`). `serde_json::Value` is the one dynamically-typed value, confined to frontmatter storage.

## Error handling

`Result<T, QuarryError>` everywhere; `QuarryError` is a `thiserror` enum with `Refusal(String)` (exit 1) and `External(String)` (exit 2), plus `From` impls for `std::io::Error`, `rusqlite::Error`, `serde_json::Error` (all → `External`). Git failures are wrapped by `Git::run` carrying stderr verbatim. Only `main` inspects the variant, to choose the exit code and stream (S13). Warnings (skipped edge entries, unparsable pages) are values (`Vec<Warning>`) returned from `Index::rebuild`, never errors. No panics on any input path; no `anyhow`. No retries except S1's three push attempts in `docsrepo::commit_and_push`.

## Dependency injection

Explicit parameters. `Context` (built once in `main`) carries the repo root, `RepoIdentity`, `Config`, clone and index paths, output flags, and `now: fn() -> jiff::Timestamp` for tests. No container, no statics with interior mutability; `importer::HOST_TEMPLATES` is the one module-level `const`.
