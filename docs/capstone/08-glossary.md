---
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
paths_covered:
  - ":(top)src/**"
---
> Prescriptive design intent; code does not exist yet.

# Glossary

## Concepts

| Term | Meaning in this system | Where |
| --- | --- | --- |
| quarry | the CLI (a single precompiled binary), and by extension the whole scheme | `src/` |
| docs repo | the shared git repository holding every repo's copied docs; the system of record; "the main quarry" in conversation | `docsrepo` |
| source repo | a code repository whose `docs/capstone/` is imported | `Context.repo_root` |
| clone | the shallow (`--depth 1`) checkout of the docs repo inside a source repo's `.quarry/<name>/`; disposable, reset on every write | `docsrepo.refresh` |
| repo name | last path segment of the source repo's `origin` URL; the folder name in the docs repo | `identity` (S9) |
| origin (normalized) | `host/owner/repo`, lowercase; stored in the stamp to detect two repos claiming one name | `identity` |
| default branch | the ref `origin/HEAD` pointed at when `init` ran; the only branch whose commits may be imported | `Config.default_branch` (S1) |
| stamp | `.quarry-stamp` in a repo's docs-repo folder: the imported commit and origin; the idempotence key | `docsrepo` (S1) |
| forward / backward / diverged | the ancestry relation between the stamp and HEAD deciding import, skip, or refuse | S1 step 5 |
| import | copying the docs folder at one commit into the docs repo, with permalinks | `importer` |
| permalink | a `path:line` pointer rewritten as a link pinned to the imported commit | `importer` (S4) |
| root index | the docs repo's `00-index.md`, regenerated whole on every write | `docsrepo.regenerate_root_index` (S3) |
| page | one markdown file under a repo's folder | `pages` table |
| section | the text under one heading up to the next heading of equal or higher level | `sections` table (S8) |
| edge | a producer→consumer relation declared in `produces`/`consumes` frontmatter | `edges` table (S6) |
| declared_by | which side(s) declared an edge: `producer`, `consumer`, `both` | `Edge.declared_by` |
| dangling edge | an edge whose other repo has no folder in the docs repo (`missing`) | S6, S10 |
| index | `.quarry/.docs-index.sqlite`, derived from the clone, rebuilt whole when stale | `index` (S5) |
| sync | `git pull` of the clone followed by a rebuild; the only network touch on the read path | `commands.sync` |
| envelope | the JSON document every command prints under `--json` | `output` (S13) |
| refusal / external failure | the two `QuarryError` variants, exit 1 and exit 2 | `errors` (S13) |
