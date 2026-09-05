---
scenario: index-freshness
id: S5
surfaces_on: [mockup/06-sync.md, mockup/07-docs-list.md, mockup/08-docs-show.md, mockup/13-docs-index.md]
depends_on: [S6, S11, S13]
generated_date: 2026-09-05
readback: a463e52ce97d
capstone_version: 5.2.1
mode: prescriptive
---
# S5 - index freshness

The local SQLite index is derived from the clone and rebuilt whenever it lags; queries never reach the network.

## Trigger & preconditions

- Trigger: every `quarry docs *` command opens the index before answering; `quarry sync` after pulling; `add`/`update`/`remove` after pushing; `docs index [--force]` explicitly.
- `.quarry/` initialised with a clone present (S12).

## Steps

1. Index metadata rows: `built_at_commit` (clone HEAD at build time), `schema_version` (fixed per quarry release), `synced_at` (UTC instant of the last `sync`, or of the last fetch by `init`/`add`/`update`/`remove`).
2. On open: rebuild in full when the file is missing, cannot be opened, `schema_version` differs from the running quarry, or `built_at_commit` ≠ the clone's current HEAD. Otherwise use as-is.
3. Precondition: the SQLite quarry links has FTS5. Asserted at build time by a unit test on `pragma compile_options`; no runtime check, since the binary bundles its own SQLite. A page larger than 5 MB is indexed by frontmatter only, body skipped, warning recorded.
4. A rebuild: parse every `.md` under the clone (root `00-index.md` excluded), write `pages` (path, repo, frontmatter JSON, `generated_date`), `edges` (S6), and the FTS table of sections (S8) into a new file, then atomically replace the old one. Never incremental.
5. `docs index --force`: rebuild regardless of stamps.
6. `sync`: `git pull --ff-only` on the clone (reset first per S11), set `synced_at`, then step 2.
7. Human output of any `docs *` command: when `synced_at` is older than 24 hours, one leading line `docs clone last synced <age> ago; run quarry sync`. JSON: `synced_at` in the envelope, no warning text.

## Branches

| Point | Rule |
| --- | --- |
| index current | answer |
| stale or absent | rebuild, then answer |
| corrupt (open or integrity check fails) | delete, rebuild, then answer; no `--force` needed |
| `synced_at` absent (never synced) | warning line says `never synced` |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| rebuild fails on one unparsable page | page recorded with empty frontmatter and no edges; `docs index` lists it as a warning; the build completes |
| two processes rebuild at once | each builds its own temp file; the last atomic replace wins; both are identical for the same HEAD |
| disk full during rebuild | temp file removed, old index kept, exit 2 |

## State transitions

Index: `absent → built(HEAD)`; `built(H1) → built(H2)` on any HEAD change. No terminal state.

## Invariants

- A query never answers from an index whose `built_at_commit` differs from the clone HEAD.
- A query never modifies the clone.
- Rebuild is atomic: the index file is always either the previous complete build or the new one.

## Outcomes & side effects

- `sync` and `index` print counts (`mockup/06-sync.md`, `13-docs-index.md`); queries print the staleness line when due.
- Nobody notified.

## Dimensions not in play

- D1 (local file, cwd's user), D3 (no input beyond `--force`), D4 (counts only), D5, D9 (index has no lifecycle beyond rebuild), D13, D14, D15 (the index is disposable; the clone's git log is the record): none.
- D6: the 24-hour staleness line is a warning, not a limit; the 5 MB page cap is the one hard limit and degrades to frontmatter-only indexing rather than refusing.
- D12: `synced_at` and `built_at_commit` are always exposed in JSON.
