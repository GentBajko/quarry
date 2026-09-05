---
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
paths_covered:
  - ":(top)src/config.rs"
  - ":(top)src/identity.rs"
  - ":(top)src/frontmatter.rs"
  - ":(top)src/index.rs"
  - ":(top)src/docsrepo.rs"
---
> Prescriptive design intent; code does not exist yet.

# Models

All types are plain structs and enums deriving `Debug, Clone, PartialEq, Eq` (and `Serialize`/`Deserialize` where persisted); persistence is JSON files (per-repo state) and one SQLite file (derived index).

## Entities

| Entity | Definition site (planned) | Storage | Purpose |
| --- | --- | --- | --- |
| `Config` | `src/config.rs` | `.quarry/.config` (JSON) | the source repo's link to its docs repo (S12) |
| `RepoIdentity` | `src/identity.rs` | derived per run | name and normalized origin (S9) |
| `Stamp` | `src/docsrepo.rs` | `<docs repo>/<repo>/.quarry-stamp` (JSON, committed) | imported commit and origin (S1, S9) |
| `Page` | `src/index.rs` | `pages` table | one markdown file's frontmatter and date |
| `Edge` | `src/index.rs` | `edges` table | one producer→consumer relation (S6) |
| `Section` | `src/frontmatter.rs` | `sections` FTS5 table | one heading and its body (S8) |
| `RepoRow` | `src/index.rs` | `repos` table | per-repo summary for `list` and the root index (S3) |
| `IndexMeta` | `src/index.rs` | `meta` table | `built_at_commit`, `schema_version`, `synced_at` (S5) |

Newtypes: `RepoName(String)`, `Sha(String)` (40 hex, validated), `NormalizedOrigin(String)`.

## Fields and types

`Config { url: String, docs_dir: String /* repo-relative, default "docs/capstone" */, default_branch: String, permalink_template: Option<String> }`.

`RepoIdentity { name: RepoName, host: String, owner: String, repo: String, origin: NormalizedOrigin /* host/owner/repo, lowercase */ }`.

`Stamp { commit: Sha, origin: Option<NormalizedOrigin> }`; `None` only when loading a stamp written before the field existed; rewritten on the next import.

`Page { repo: RepoName, path: String /* repo-relative, '/' separators */, frontmatter: serde_json::Value, generated_date: Option<String> /* YYYY-MM-DD verbatim */ }`.

`Edge { from_repo: RepoName, to_repo: RepoName, kind: String /* lowercased */, name: String, declared_by: DeclaredBy, via: Vec<String> /* sorted page paths */, missing: bool }` with `enum DeclaredBy { Producer, Consumer, Both }`.

`Section { repo: RepoName, path: String, heading: String, normalized: String, level: u8 /* 1–6 */, body: String }`.

`RepoRow { name: RepoName, commit: Option<Sha>, origin: Option<NormalizedOrigin>, newest_generated_date: Option<String>, pages: u32, produces: u32, consumes: u32 }`.

`IndexMeta { built_at_commit: Sha, schema_version: u32, synced_at: Option<jiff::Timestamp> }`.

## Relationships

- `Page.repo` and `Section.repo` reference `RepoRow.name`; `Section.(repo, path)` references `Page`.
- `Edge.from_repo` / `Edge.to_repo` reference `RepoRow.name`; a reference with no row is `missing = true` (S6), kept.
- `Stamp` lives inside the folder it describes; `RepoRow.commit` is read from it at rebuild.
- `Config` is per source repo and references nothing in the index.

## Boundaries

| Representation | Where converted |
| --- | --- |
| YAML frontmatter → `Frontmatter` (a `serde_json::Value` object) | `frontmatter::parse` via `serde_saphyr::from_str::<serde_json::Value>`; the only YAML call site |
| `Frontmatter` → `EdgeDecl` | `frontmatter::edges_of` (S6 required fields; invalid entries dropped with a warning) |
| structs ↔ SQLite rows | `index` accessors, one per table; no ORM |
| structs → JSON envelope | `output::envelope` (`serde_json::to_string`) |
| JSON file ↔ `Config`, `Stamp` | `config::{load, write}`, `docsrepo::{read_stamp, write_stamp}` via `serde_json` |

No API layer and no DTOs exist beyond the envelope.

## Validation

- CLI arguments: `clap` value parsers (`--depth` `u32`, `--downstream`/`--upstream` in an `ArgGroup(required = true)`).
- `Config`: `docs_dir` relative, no `..` component; URL must parse per S9.
- Frontmatter: S6 rules in `frontmatter::edges_of`; unparsable YAML → empty frontmatter plus a warning collected by `Index::rebuild`.
- Page size cap and the FTS5 guard: `logic/08-index-freshness.md` step 3 (the guard is retired: `rusqlite` `bundled` compiles FTS5 in; a unit test asserts `pragma compile_options` contains `ENABLE_FTS5`).
- Paths inside a copied docs folder: `git archive` output is unpacked by `importer` with every entry name checked for `..` and absolute components (S4, Q4); offending entries are refused.
- Database constraints below enforce edge identity and `declared_by`.

## Schema

`schema_version = 1`. Created whole on every rebuild into a temp file, then atomically renamed over `.quarry/.docs-index.sqlite` (S5). No migrations: a version mismatch rebuilds.

```sql
CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);                                     -- built_at_commit, schema_version, synced_at

CREATE TABLE repos (
  name                  TEXT PRIMARY KEY,
  commit_sha            TEXT,
  origin                TEXT,
  newest_generated_date TEXT,
  pages                 INTEGER NOT NULL,
  produces              INTEGER NOT NULL DEFAULT 0,
  consumes              INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE pages (
  repo           TEXT NOT NULL,
  path           TEXT NOT NULL,
  frontmatter    TEXT NOT NULL,        -- JSON
  generated_date TEXT,
  PRIMARY KEY (repo, path)
);

CREATE TABLE edges (
  from_repo   TEXT NOT NULL,
  to_repo     TEXT NOT NULL,
  kind        TEXT NOT NULL,
  name        TEXT NOT NULL,
  declared_by TEXT NOT NULL CHECK (declared_by IN ('producer','consumer','both')),
  via         TEXT NOT NULL,           -- JSON array of page paths
  missing     INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (from_repo, to_repo, kind, name)
);
CREATE INDEX edges_to ON edges (to_repo);

CREATE VIRTUAL TABLE sections USING fts5 (
  repo UNINDEXED, path UNINDEXED, heading, normalized UNINDEXED, level UNINDEXED, body,
  tokenize = 'unicode61'
);
```

Files outside SQLite:

- `.quarry/.config`: `{"default_branch":"main","docs_dir":"docs/capstone","permalink_template":null,"url":"..."}` (keys sorted, pretty-printed, trailing newline).
- `<docs repo>/<repo>/.quarry-stamp`: `{"commit":"<40 hex>","origin":"github.com/acme/ingest-api"}` plus trailing newline; two keys, sorted, so two importers write identical bytes.
