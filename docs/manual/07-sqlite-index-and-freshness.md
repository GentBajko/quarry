# The SQLite index, SQL, and freshness

Quarry uses embedded **SQLite with FTS5**, provided by `rusqlite` with its `bundled` feature. The index is `.quarry/.docs-index.sqlite` in each source worktree. There is no remote SQL service, shared SQLite file, vector database, or separate indexing process.

The current application schema version is **3**. This number describes Quarry's tables and metadata; it is unrelated to the SQLite engine version or source-doc format version.

## When the index rebuilds

Every indexed read compares the shared clone's current Git `HEAD` with `meta.built_at_commit`, and checks `meta.schema_version`. It fully rebuilds when:

- The cache file is missing or unreadable as a usable cache.
- The schema version does not match 3.
- The docs clone's `HEAD` differs from `built_at_commit`.
- `quarry docs index --force` requests it.

`init` and completed writes explicitly rebuild. `sync` fetches/reset-cleans the clone, then opens the index and rebuilds only when needed. An empty docs repository has the sentinel head string `empty`.

This is a **whole-index replacement**, not a per-page incremental update or SQL migration. The builder removes a stale `.docs-index.sqlite.tmp`, creates a fresh database there, inserts the complete scan in a transaction, commits and closes it, then renames it to `.docs-index.sqlite`. An existing `synced_at` value is carried across a rebuild. The final cache is not edited page by page.

The fixed temporary path means concurrent rebuilds in the same worktree are unsafe. Serialize commands in one `.quarry/`; independent source worktrees have independent caches.

## What the scanner sees

At the shared clone root, only **immediate child directories containing `00-index.md`** are registered repo folders. Root-level prose is not indexed as a repo. Within each registered folder the walker recursively gathers lowercase `.md` files, skipping directories whose basename is `.git`, and orders paths lexically.

For each Markdown page it stores the frontmatter object and optional string date, splits the body into heading sections, collects aliases, and parses edges. After scanning all repos it resolves endpoint names/aliases, joins eligible open ends, merges explicit/observed edges, and writes publications and diagnostics metadata.

A page larger than **5 × 1024 × 1024 bytes** keeps its page metadata, aliases, and parsed edge declarations but contributes no searchable sections. The rebuild warning says `over 5 MB, body not indexed`. The implementation still reads/parses the file before suppressing sections; this is not a streaming memory limit. Read failures or invalid UTF-8 can result in an empty page body in the scan because page text uses a default-on-error read.

## Exact schema

The following SQL is the schema embedded in the documented revision:

```sql
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE repos (
  name TEXT PRIMARY KEY,
  commit_sha TEXT,
  origin TEXT,
  newest_generated_date TEXT,
  pages INTEGER NOT NULL,
  produces INTEGER NOT NULL DEFAULT 0,
  consumes INTEGER NOT NULL DEFAULT 0,
  known_as TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE pages (
  repo TEXT NOT NULL,
  path TEXT NOT NULL,
  frontmatter TEXT NOT NULL,
  generated_date TEXT,
  PRIMARY KEY (repo, path)
);
CREATE TABLE edges (
  from_repo TEXT NOT NULL,
  to_repo TEXT NOT NULL,
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  declared_by TEXT NOT NULL CHECK (
    declared_by IN ('producer','consumer','both','observed','joined')
  ),
  via TEXT NOT NULL,
  missing INTEGER NOT NULL DEFAULT 0,
  site_unverified INTEGER NOT NULL DEFAULT 0,
  as_declared TEXT,
  resolved_by TEXT,
  observed INTEGER NOT NULL DEFAULT 0,
  last_seen TEXT,
  PRIMARY KEY (from_repo, to_repo, kind, name)
);
CREATE INDEX edges_to ON edges (to_repo);
CREATE TABLE aliases (alias TEXT PRIMARY KEY, repo TEXT NOT NULL);
CREATE TABLE publications (
  repo TEXT NOT NULL,
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  via TEXT NOT NULL,
  PRIMARY KEY (repo, kind, name)
);
CREATE VIRTUAL TABLE sections USING fts5 (
  repo UNINDEXED, path UNINDEXED, heading, normalized UNINDEXED,
  level UNINDEXED, body,
  tokenize = 'unicode61'
);
```

`known_as`, page `frontmatter`, and each `via` are JSON encoded into TEXT columns. Boolean edge fields use 0/1 integers. `as_declared` is a comma-joined string of changed endpoint spellings. There are no SQL foreign keys; unresolved explicit endpoints are intentionally representable. `by-name` is a query-time lead and is not a permitted `declared_by` value in stored edges.

FTS5 creates its own shadow tables in addition to the schema above. `heading` and `body` are full-text indexed; `repo`, `path`, `normalized`, and `level` are stored without full-text indexing. `normalized` supports heading lookup. The split section body excludes its heading and later subheadings' bodies.

## Metadata keys

| Key | Stored value |
|---|---|
| `schema_version` | String `3`. |
| `built_at_commit` | Full docs-repo HEAD used as the rebuild identity, or `empty`. |
| `synced_at` | Timestamp recorded by successful Quarry refresh/write initialization logic; may be absent. |
| `ambiguous` | JSON array of nonunique producer matches, stored only when nonempty. |
| `observed_edges` | Accepted observation row count, present when observed file exists. |
| `observed_generated_at` | Optional generation string from observed file. |

Ordinary warnings and explicitly unresolved endpoint records are **not** stored in `meta`. They are returned by that rebuild's `RebuildReport`. A current-cache `docs index` recovers repo count, ambiguities, and observed metadata, while `pages` and `edges` remain default report counters (zero in JSON), and `warnings`/`unresolved` are empty. This does not mean the docs have zero pages/edges or no unresolved endpoints. Run `docs index --force` to obtain the complete report again.

## Search SQL and parameter handling

Quarry escapes every `"` in the user term by doubling it, then encloses the complete input in `"..."`. That phrase is bound as parameter 1; repository and limit are also bound parameters. The query is:

```sql
SELECT s.repo, s.path, s.heading,
       snippet(sections, 5, '', '', ' … ', 12),
       p.generated_date
FROM sections s
LEFT JOIN pages p ON p.repo = s.repo AND p.path = s.path
WHERE sections MATCH ?1 AND (?2 IS NULL OR s.repo = ?2)
ORDER BY bm25(sections), s.repo, s.path
LIMIT ?3;
```

The app does not pass caller input through as raw SQL or raw FTS query syntax. Ranking uses default `bm25` weights; lower values sort first. The snippet is from FTS column 5 (`body`), without highlight markup. Newlines are replaced with spaces in the JSON result.

Exact section matching joins section rows to page dates and uses `s.repo = ?1 AND s.normalized = ?2`. Only when exact matching returns nothing does prefix matching use:

```sql
s.repo = ?1 AND s.normalized LIKE ?2 || '%'
```

The fallback does not escape SQL `LIKE` wildcards: `%` and `_` in a heading query can act as wildcards. A quote is still a bound value and cannot escape into SQL syntax. The nearest-heading fallback is a shared-prefix scorer, not edit distance or semantic similarity.

Graph traversal runs in Rust over SQL edge reads ordered by counterpart repo, kind, and name. It is not a recursive SQL CTE. `docs path` uses breadth-first search; downstream by-name leads use a separate FTS query restricted to exact page path `09-interfaces.md`.

## Inspect with the optional sqlite3 CLI

The following commands are **illustrative read-only inspection recipes**, run from a source repo's root. They require a separate `sqlite3` CLI installation; Quarry itself does not require it.

```sh
sqlite3 -readonly .quarry/.docs-index.sqlite '.schema'
sqlite3 -readonly .quarry/.docs-index.sqlite \
  'SELECT key, value FROM meta ORDER BY key;'
sqlite3 -readonly .quarry/.docs-index.sqlite \
  'SELECT name, pages, produces, consumes FROM repos ORDER BY name;'
```

```sql
-- Trace a single imported page and its metadata.
SELECT repo, path, generated_date, frontmatter
FROM pages
WHERE repo = 'record-store'
ORDER BY path;

-- Inspect unresolved explicit endpoints retained as edges.
SELECT from_repo, to_repo, kind, name, declared_by, via
FROM edges
WHERE missing = 1
ORDER BY from_repo, to_repo, kind, name;

-- Compare declared and measured relationships.
SELECT from_repo, to_repo, kind, name, declared_by,
       observed, last_seen, site_unverified
FROM edges
ORDER BY from_repo, to_repo, kind, name;

-- Read accepted alias keys, which are stored in lowercase.
SELECT alias, repo FROM aliases ORDER BY alias;

-- Count searchable section rows, separate from page count.
SELECT repo, count(*) AS sections FROM sections GROUP BY repo;
```

Do not modify the derived database to correct docs. Your changes are disposable and the next rebuild overwrites them. Edit the source pages or committed external input, import/sync, then rebuild.

## Four freshness signals

| Signal | Answers | Does not establish |
|---|---|---|
| Folder `.quarry-stamp.commit` / repo `commit` | Which source commit was imported? | Whether docs accurately cover that code. |
| `built_at_commit` | Which combined docs-clone commit is the cache keyed to? | Whether the remote has newer commits. |
| `synced_at` | When did this local reader record a successful refresh/init/write? | When source docs were generated or reviewed. |
| Page `generated_date` / repo `newest_generated_date` | What date did a page author put in metadata? | Freshness of every other page or verified coverage. |

Observed traffic adds its own `generated_at`/`last_seen` strings. None of these checks poll code repos or compare coverage fingerprints. Capstone's `map check` is a different check with different inputs.

Human indexed reads emit a stale-clone note when `synced_at` is absent or at least 24 whole hours old. A parseable timestamp is required; an invalid timestamp yields no age note. JSON suppresses the human age sentence and instead exposes the timestamps. A stale note does not change exit status.

## Manual clone changes and recovery

Automatic freshness keys only on docs Git `HEAD` plus schema, not mtimes, content hashes, or dirty state. Editing a copied page or `observed-edges.json` without moving HEAD does not automatically rebuild. `docs index --force` scans the current clone filesystem, including such changes, and labels it with the same HEAD. Therefore `built_at_commit` is a reliable committed snapshot identifier only while the managed clone stays clean.

`quarry sync` restores the managed clone to the remote and normally makes your cache reflect it. If you previously forced an index of dirty clone content and the remote HEAD did not change, follow sync with `docs index --force` to replace that cache too. Rebuild does not imply a network fetch; sync does not imply source docs were regenerated.

A missing, corrupt, or older-schema cache is normally rebuilt automatically. If diagnostics remain suspicious:

```sh
quarry sync
quarry docs index --force
```

Restore remote access separately if sync fails. Ordinary `--sync` reads may answer locally after a fetch failure; explicit `quarry sync` reports an external failure instead.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [Cargo.toml](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/Cargo.toml) [src/index.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/index.rs) [src/query.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/query.rs) [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [src/output.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/output.rs) [tests/s05_index.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s05_index.rs) [tests/s20_name_join.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s20_name_join.rs) [tests/perf.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/perf.rs)
