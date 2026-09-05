---
generated_date: 2026-09-05
capstone_version: 5.2.1
---

# Changelog

## 2026-09-05 - logic: 09-edge-contract
key: logic/09-edge-contract@Q19

- `logic/09-edge-contract.md`: a second accepted form added. Tables under `## Produces` / `## Consumes` on a page named `09-interfaces.md` declare edges when the page carries no frontmatter keys; frontmatter wins when both are present.
- Decision: columns matched by name, cells stripped of backticks and markdown link syntax, `\|` a literal pipe.
- Decision: tables are read on that filename only and never inside a fenced block, so a chapter documenting the format declares nothing.
- Reason: the mirrored form let the table and the frontmatter disagree silently, with quarry believing one and the reader the other.
- Rejected: replacing frontmatter with tables. Positional columns rename silently, optional fields need new columns everywhere, and a non-Capstone generator would have to emit escaped markdown.
- Source: `src/frontmatter.rs` table reader, 6 unit tests and 2 integration tests, including the documented-example case that caught the prose hazard.

## 2026-09-05 - build: code
key: build/code@Q1

- Source: `src/` (14 modules), `tests/` (13 files), `Cargo.toml`, `README.md`, `.github/workflows/ci.yml`, `dist-workspace.toml`.
- All 15 build-order steps complete; 119 tests pass (27 unit, 92 integration) plus the 5,000-page rebuild budget at 10.9 s in release.
- Gates green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- Spikes: serde-saphyr parses Capstone frontmatter unchanged; a `--depth 1` clone pushes (local paths need `file://` for depth to apply, recorded in the test harness).
- Divergences from the plan recorded in `implementation.md` § Divergences: ls-tree/show instead of a tar reader, `update --force`, reset to `FETCH_HEAD`, deepen before counting discards.
- Bugs the tests caught: empty env vars read as values; `refresh` failing on an empty docs repo; a false "discarded local commits" line on a shallow clone.
- Repository initialised with four commits on `main`; no history carries the struck doc comments.

## 2026-09-05 - standards: readback
key: standards/readback@doc-comments

- `standards.md` § Organization: the mandatory scenario-citing doc comment is struck; `code-craft.md`'s Comments section governs unchanged.
- Reason: the user flagged the generated comments as useless prose the craft file already bans. The rule was self-answered under delegation and manufactured them.
- Kept: clap `///` strings, which are `--help` output rather than commentary.
- Applied to `src/` before the first commit, so no history carries them.

## 2026-09-05 - readback: all
key: readback/all@a463e52ce97d

- Re-run after the Rust decision: read `mockup/`, `logic/`, chapters `01`–`08`, `standards.md`, `05-dependencies.md`.
- No misplacement: language, crates, distribution sit in architecture and stack; no logic scenario names a language.
- Contradiction resolved: `logic/08-index-freshness.md` step 3's FTS5 startup guard vs `rusqlite` `bundled` (FTS5 compiled in): the guard is retired in the chapters and the plan; the logic step keeps the precondition as "asserted at build by a unit test". See `logic/readback@a463e52ce97d`.
- `mockup/05-ci-workflow.md`: installer line added (a precompiled binary replaces `uv tool install`).

## 2026-09-05 - logic: readback
key: logic/readback@a463e52ce97d

- `logic/08-index-freshness.md` step 3: FTS5 precondition reworded from a runtime refusal to a build-time assertion; 5 MB page cap unchanged.

## 2026-09-05 - mockup: readback
key: mockup/readback@a463e52ce97d

- `mockup/05-ci-workflow.md`: installer element and layout line added.

## 2026-09-05 - stack: all
key: stack/all@Q11

- `05-dependencies.md`: capability matrix rewritten for Rust; nine runtime crates recorded with floors and licences.
- Picks: clap 4 (derive, env), rusqlite `bundled`, serde + serde_json, serde-saphyr, thiserror 2, tempfile 3, regex 1, jiff 0.2; cargo-dist 0.32 to GitHub Releases; six targets.
- Rejected: gix, git2, sqlx, chrono, anyhow, assert_cmd, insta, crates.io-only distribution; Homebrew tap deferred.
- Superseded: PyYAML, argparse, stdlib sqlite3, uv_build, pytest/pyright/ruff/uv, PyPI (`stack/all@Q10`).

## 2026-09-05 - standards: all
key: standards/all@Q10

- `standards.md`: rewritten for Rust; typing (`forbid(unsafe_code)`, `missing_docs`, clippy `unwrap_used`/`expect_used`, newtypes), crate budget = the recorded list, `Result` + `thiserror`, no panics on input, inline unit tests + binary-driven integration tests, `cargo fmt/clippy/test` on three OSes.
- Process and agent rules carried over; added: never `unsafe`, never add a crate on its own.
- Supersedes `standards/all@Q9`.

## 2026-09-05 - architecture: all
key: architecture/all@Q23

- User re-decision (Q22): Rust, precompiled releases, no cargo or runtime for users. Supersedes `architecture/all@Q21` Python picks.
- `01-architecture.md`, `02-models.md`, `03-conventions.md`, `04-data-flow.md`, `06-testing.md`, `07-operations.md`, `08-glossary.md`: rewritten for the Rust crate; `05-dependencies.md` by the stack re-run; schema and lifecycles unchanged.
- Rejected: Python (distribution, startup), Go (maintainer preference, cgo).
- Derived: clap, rusqlite bundled, serde-saphyr, thiserror, tempfile, regex, jiff; git via `std::process::Command`; cargo-dist with six targets and shell/PowerShell installers.
- Retired risk: FTS5 availability (compiled in). New spike: serde-saphyr on Capstone frontmatter. Deferred additions: Homebrew tap, `gix`.

## 2026-09-05 - build: plan
key: build/plan@Q1

- `implementation.md`: rewritten for Rust; layout, `Cargo.toml`, load-bearing sketches (error mapping, `Git::run`, S1 retry loop, `git archive` + hand-rolled tar reader, atomic index rebuild, deps BFS), 15 build steps with verification, coverage 13 scenarios / 15 screens / 13 components.
- Awaiting the user's approval; no code written.

## 2026-09-05 - readback: all
key: readback/all@f483d8b597a7

- Read: `mockup/` (README + 15 screens), `logic/` (13), `uiux-interview.md` (skipped), `01`–`08` chapters, `standards.md`, `05-dependencies.md`.
- Moved: edge frontmatter shape, `mockup/README.md` → `logic/09-edge-contract.md` (README keeps a citation).
- Moved: 5 MB page cap and FTS5 startup guard, `02-models.md` / architecture Q19–Q21 → `logic/08-index-freshness.md` step 3 (chapter keeps a citation).
- Resolved: `standards.md` agent rule "never add a runtime dependency" vs § Libraries budget-with-ledger-entry: agent never adds on its own; the user's recorded decision may.
- Refreshed: `mockup/README.md` Assumed table now names the owning file for each item settled downstream.
- No contradictions between numbers or rules found across the six outputs.

## 2026-09-05 - standards: readback
key: standards/readback@f483d8b597a7

- `standards.md`: agent rule on runtime dependencies reworded to match § Libraries.

## 2026-09-05 - logic: readback
key: logic/readback@f483d8b597a7

- `logic/08-index-freshness.md`: step 3 added (FTS5 guard, 5 MB page cap), relocated from the architecture stage.

## 2026-09-05 - mockup: readback
key: mockup/readback@f483d8b597a7

- `mockup/README.md`: § Edge contract reduced to a citation of `logic/09-edge-contract.md`; § Assumed rewritten with the settling file per item.

## 2026-09-05 - stack: all
key: stack/all@Q10

- `05-dependencies.md`: capability matrix added; `mode: prescriptive` kept.
- Pick: PyYAML ≥ 6.0.3 (MIT), the one runtime dependency; rejected ruamel.yaml 0.19.1 and python-frontmatter.
- Pick: `uv_build` ≥ 0.11.19 build backend; rejected hatchling, setuptools.
- No dependency: argparse, sqlite3/FTS5, system git (ladder rungs 3–4); rejected GitPython, dulwich, pygit2.
- Derived from standards: pytest ≥ 8, pyright ≥ 1.1.380, ruff ≥ 0.5, uv.
- CI GitHub Actions; PyPI trusted publishing; no paid services; no docs site (deferred).
- Research sources: web search 2026-09-05 (PyPI, Snyk, Astral docs), Context7 `/yaml/pyyaml`.

## 2026-09-05 - standards: all
key: standards/all@Q9

- `standards.md`: written; nine domains, imperative rules.
- Decisions self-answered under the user's delegation from the architecture chapters and the user's standing agent instructions.
- Decision: pyright strict, `Any` only at the YAML boundary, `StrEnum` for closed sets.
- Decision: runtime dependency budget of one; adding one needs a ledger entry naming the ladder rung.
- Decision: frozen slotted dataclasses, procedural modules, no inheritance for behaviour.
- Decision: exceptions caught only in `cli.main`; warnings as values; no logging.
- Decision: docstrings mandatory on public core functions, citing the logic step; none elsewhere.
- Decision: TDD per code-craft; real git and SQLite in tests; mocks forbidden; one test per logic rule.
- Decision: ruff (E, F, I, UP, B, SIM, RUF; 100 cols), uv, pytest `--strict-markers`, slow marker.
- Decision: code-craft git rules plus a standing ban on attribution trailers.
- No domain ruled out; no code-craft rule overridden.
- Suggested, not done: seed `CLAUDE.md`/`AGENTS.md` from § Agent rules.

## 2026-09-05 - architecture: all
key: architecture/all@Q21

- `01-architecture.md` … `08-glossary.md`: written, `mode: prescriptive`, planned-layout `paths_covered`.
- Decisions Q1–Q21 self-answered under the user's delegation; alternatives recorded per entry.
- One-way door: one Python ≥ 3.12 package (modular monolith, `uv tool install`); rejected Go single binary (team fit) and library + indexer service (nothing long-running).
- One-way door: stdlib `argparse`, stdlib `sqlite3` with FTS5, one YAML library; no framework; rejected `typer`/`click` for now (deferred with trigger).
- Decision: docs repo clone is shallow (`--depth 1`), reset on every write; rejected `--filter=blob:none` and a shared per-machine clone (deferred).
- Decision: `.quarry/.config` and `.quarry-stamp` are JSON; stamp holds `commit` and `origin` only.
- Decision: import reads `git archive HEAD:<docs_dir>`, never the working tree.
- Decision: errors are one exception hierarchy mapped to exit codes in `cli.main`; no result types; warnings are values.
- Decision: pyright strict; `Any` confined to the YAML boundary.
- Decision: tests use real git repos in `tmp_path`; no git or SQLite mocks; one test per logic rule.
- Walking skeleton: `init` → `add` → `docs list` against a local bare docs repo, plus CI publishing a dev build to TestPyPI.
- Deferred with triggers: incremental index (rebuild > 30 s); `typer` (> 15 subcommands or completion requested); shared per-machine clone (disk complaints); `.sqlite` as CI artifact; native Windows (first Windows user); `import-linter` (a violation slips the layering test).
- Pre-build spikes: FTS5 presence across Python builds; push from a `--depth 1` clone to GitHub and GitLab.
- Not applicable: frontend, multi-tenant, AI/ML, compliance, legacy, real-time, observability (local CLI).
- Quality measures: `docs *` p95 ≤ 200 ms at 5k pages; rebuild ≤ 10 s at 5k, ≤ 60 s at 50k; `update` ≤ 30 s excluding network; page cap 5 MB.

## 2026-09-05 - uiux: skipped
key: uiux/skipped@2026-09-05

- No output: `mockup/README.md` records `surfaces: [cli]` only; nothing to design.
- Consequence: the mockup screens' command copy and help text stand as final wording.

## 2026-09-05 - logic: wrap
key: logic/all@Q18

- `00-index.md`: 13 `logic` topic rows added.
- Decisions Q7–Q18 self-answered under the user's delegation; each records the alternatives rejected.
- Coverage: 13 scenarios; no dimension left open; inapplicable dimensions listed per file.
- Cross-scenario check: none contradict; `--force` now exists on `init` (relink) and `update` (diverged stamp) with different meanings, and never overrides the S9 origin check.

## 2026-09-05 - logic: 13-output-and-exit-codes
key: logic/13-output-and-exit-codes@Q18

- `logic/13-output-and-exit-codes.md`: written; exit 0/1/2 table; JSON envelope with `built_at_commit`, `synced_at`; per-item `file`, `generated_date`; write-command result shapes.
- Rejected: exit 0 on unknown repo.

## 2026-09-05 - logic: 12-removal-dangling-edges
key: logic/12-removal-dangling-edges@Q17

- `logic/12-removal-dangling-edges.md`: written; warn and proceed.
- Rejected: refuse; silent.

## 2026-09-05 - logic: 11-section-and-search
key: logic/11-section-and-search@Q16

- `logic/11-section-and-search.md`: written; exact-then-prefix heading match, ambiguity exit 1 with candidates; FTS5 per section, bm25, limit 20.
- Rejected: fuzzy match; whole-file return.

## 2026-09-05 - logic: 10-graph-traversal
key: logic/10-graph-traversal@Q15

- `logic/10-graph-traversal.md`: written; `deps` BFS, default depth 1, `0` unlimited, cycle marker; `path` directed BFS with byte-order tie-break, reverse fallback.
- Rejected: unlimited default depth; undirected default.

## 2026-09-05 - logic: 09-edge-contract
key: logic/09-edge-contract@Q14

- `logic/09-edge-contract.md`: written; `produces`/`consumes` fields, required set, skip-with-warning, `declared_by`, `missing`, `via` list.
- Rejected: closed `kind` enum; rejecting one-sided edges.

## 2026-09-05 - logic: 08-index-freshness
key: logic/08-index-freshness@Q13

- `logic/08-index-freshness.md`: written; rebuild on missing/corrupt/schema/HEAD mismatch, always full, atomic; queries never pull; 24h staleness line; `synced_at` in JSON.
- Rejected: auto-pull per query; incremental rebuild.

## 2026-09-05 - logic: 07-network-failure
key: logic/07-network-failure@Q12

- `logic/07-network-failure.md`: written; no quarry-level retries; exit 2; diverged clone reset hard; credentials are git's.
- Rejected: backoff retries on network errors.

## 2026-09-05 - logic: 06-permalinks
key: logic/06-permalinks@Q11

- `logic/06-permalinks.md`: written; backticked `path:NN[-MM]` in bodies → pinned links; GitHub and GitLab templates; `permalink_template` override; frontmatter untouched.
- Rejected: rewriting `site:`; branch-based links.

## 2026-09-05 - logic: 05-root-index
key: logic/05-root-index@Q10

- `logic/05-root-index.md`: written; regenerated whole on every write from folders holding `00-index.md`; six columns; byte-order sort; no timestamps.
- Rejected: generated-at line (breaks byte-identical regeneration).

## 2026-09-05 - logic: 04-add-remove-lifecycle
key: logic/04-add-remove-lifecycle@Q9

- `logic/04-add-remove-lifecycle.md`: written; `add` idempotent (falls through to `update`); `update` refuses when not registered; `add` refuses without `00-index.md`; `remove` idempotent.
- Rejected: `update` auto-add.

## 2026-09-05 - logic: 03-repo-identity
key: logic/03-repo-identity@Q8

- `logic/03-repo-identity.md`: written; name from `origin` last segment, case preserved; normalized origin stored in `.quarry-stamp`; mismatch refuses, `--force` does not override.
- Rejected: `owner/repo` names; case folding.

## 2026-09-05 - logic: 02-init-and-config
key: logic/02-init-and-config@Q7

- `logic/02-init-and-config.md`: written; precedence flag > env > config, idempotent re-run, `--force` relink, `.gitignore` rewrite, `default_branch` from `origin/HEAD`.
- Decision: differing `--url` refuses without `--force`; no TTY and nothing set refuses naming the flags.
- Rejected: silent config overwrite; per-machine `~/.quarry`.
- Mockup follow-up: `02-init.md` gains `--force`.

## 2026-09-05 - logic: 01-update-ordering
key: logic/01-update-ordering@Q6

- `logic/01-update-ordering.md`: written; trigger, 11 steps, branches, unhappy paths, stamp transitions, invariants.
- Decision: import only commits reachable from the source repo's default branch (from `origin/HEAD` at `init`); no `--branch` escape.
- Decision: shallow checkout → fetch the stamp commit on demand; unresolvable or diverged stamp → refuse; `update --force` overrides.
- Decision: three push attempts, redo the copy on the fresh tree, never rebase; then `docs repo busy`.
- Decision: stampless folder imports unconditionally; killed run's temp dir discarded next run.
- Rejected: treating unknown ancestry as forward; requiring `fetch-depth: 0` in every workflow; branch previews in the docs repo.
- Mockup follow-up: `04-update.md` gains `--force`; regenerate at the readback or next mockup run.

## 2026-09-05 - mockup: all
key: mockup/all@Q19

- `mockup/README.md`: product brief (purpose, audience, positioning, success, surfaces `[cli]`, commercial model, constraints, non-goals, repo identity, edge contract, rationale, open questions), Screens/Journeys/Scenarios tables, assumed list.
- `mockup/01-help.md` … `15-quarry-layout.md`: one screen per CLI command plus the CI workflow and the docs-repo layout; journeys J1 first run, J2 docs stay current, J3 agent checks before it guesses, J4 engineer browses, J5 leaving.
- Decision: name `quarry`; the shared docs repo is "the quarry"; repo name from `origin` URL, not configurable.
- Decision: commands `init [--url] [--docs-dir]`, `add`, `update`, `sync`, `remove`, `docs list|show|section|search|deps|path|index`; `--json` everywhere.
- Decision: `.quarry/` in the source repo holds `.config`, `.gitignore`, and the docs repo clone; clone never committed to the source repo.
- Decision: writers are engineers by hand and the source repo's CI on merge to main; the docs repo has no CI.
- Decision: `sync` (pull + reindex) and `docs index` (reindex only, `--force`) stay separate; one touches the network, the other does not.
- Decision: Apache-2.0 plus Patreon / Buy Me a Coffee donations; no commercial scenario for `logic`.
- Decision: success = at least one org other than the company using quarry within a year.
- Rejected: `update --all`, manifest, cron, docs-repo CI; `--map` and its lock ref; Obsidian, Quartz, `.base` files, wikilinks, Sync.
- Out of scope: all Capstone-side changes (`09-interfaces.md` topic, `non_interactive`/`extract`, `groom`/`plan` integration, content hash, `changelog/` dir, feature numbering) → separate project; that integration must live in skill text, not harness hooks (Copilot has none).
- Out of scope: merge resolution of doc files inside a source repo (Capstone's).
- Handed to `logic`: 13 scenarios S1–S13 (README § Scenarios), every `rule: logic` marker owned by one row.
- Left open: first-wave repo count; whether ad-hoc agent sessions beyond `groom`/`plan` are a target caller (assumed yes).
- Assumed, for review: 11 items in README § Assumed (env var names, stamp file name, index file name, committable `.config`, output columns).
