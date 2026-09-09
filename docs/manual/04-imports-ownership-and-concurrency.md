# Imports, generated files, ownership, and concurrency

`quarry add` and `quarry update` import the tree at source `HEAD`. They do not import uncommitted edits and do not automatically check out the newest source commit. A detached `HEAD` is acceptable if that commit is reachable from the configured default branch.

## Import command reference

```sh
quarry add
quarry add --strict
quarry update
quarry update --strict
quarry update --force --strict
quarry remove
```

`add` registers missing folders and also updates existing ones. `update` requires every selected folder to exist. `--strict` controls secret/site gates. `update --force` only permits a diverged or unreachable previous source stamp. It does not mean overwrite every constraint.

## Actual import sequence

1. Load configuration and source identity; form the target list and optional umbrella.
2. Require the managed docs clone and resolve source `HEAD`.
3. Fetch the source default branch from origin, then require `HEAD` to be an ancestor of `origin/<default_branch>`.
4. Refresh the docs clone to its remote branch, discarding local clone changes; record a sync timestamp.
5. Check folder presence, registration requirements, ownership stamps, and commit ancestry for every unit.
6. Build all pending copies in separate `.quarry/.build-*` temporary directories before replacing any target folder.
7. Run strict gates across those builds. Secret findings are reported before site findings.
8. Replace pending folders, regenerate the shared root `00-index.md`, stage all clone changes, commit, and push.
9. Rebuild the local index and run advisory contract comparisons against the just-imported docs.

A strict refusal prevents writing the pending imported folders, but it is not a promise of zero local side effects: the preceding docs-clone refresh may already have discarded local edits and updated freshness metadata. Target builds are prepared together, but filesystem replacements are sequential and are not a multi-folder filesystem transaction.

## What is copied

Every Git-tracked file beneath the configured docs directory is read with `git show` at the selected source SHA. Markdown files ending in lowercase `.md` are rendered with source-pointer rewriting. Other files are copied as bytes. The imported `files` count includes all copied files, not just indexed Markdown; it excludes the generated `.quarry-stamp`.

Quarry replaces the entire imported folder. Files removed from the source docs disappear on the next pending import. Files manually added to that shared folder are also removed when it is replaced. A newer source commit imports even if only code changed, since the stamp must identify the new commit.

The initial `add` index-page existence check looks at the working tree; the actual contents come from the commit. Ensure `00-index.md` is committed, not merely present as an untracked file. An import whose copied folder lacks `00-index.md` will not be discoverable by the index.

## Files created in the source worktree

For docs URL `git@github.com:acme/docs-quarry.git`:

```text
.quarry/
├── .config
├── .gitignore
├── docs-quarry/
│   ├── .git/
│   ├── 00-index.md
│   ├── observed-edges.json       optional external input
│   └── record-store/
│       ├── .quarry-stamp
│       ├── 00-index.md
│       ├── 02-models.md
│       ├── 09-interfaces.md
│       └── ...                  all imported files
├── .docs-index.sqlite
├── .docs-index.sqlite.tmp       during a rebuild
└── .build-<random>/             during an import
```

Commit `.config` and `.gitignore` in the source repository. Keep the clone, SQLite cache, and temporary builds local. The generated `.quarry/.gitignore` has exactly this shape for the example URL:

```gitignore
docs-quarry/
.docs-index.sqlite
.docs-index.sqlite.tmp
.build-*/
```

`init` overwrites that ignore file; manual additions are not merged.

## Import stamp schema

Each imported folder contains `.quarry-stamp`, compact JSON with a trailing newline:

```json
{"commit":"8e7a6fd71fc1c4c652160f7ab5008e3900656212","origin":"localhost/remotes/ingest-api","docs_dir":"docs/capstone"}
```

The block above is **recorded file content** from the local fixture run. A stamp with site warnings looks like this **illustrative shape**:

```json
{
  "commit": "FULL_SOURCE_COMMIT_SHA",
  "origin": "github.com/acme/record-store",
  "docs_dir": "docs/capstone",
  "unverified": ["http GET /records", "sqs file-ingest"]
}
```

| Field | Meaning |
|---|---|
| `commit` | Full source commit imported into this folder. |
| `origin` | Normalized source host/owner/repo ownership identity. |
| `docs_dir` | Source docs directory that owns the imported folder. |
| `unverified` | Optional sorted, deduplicated `kind + space + name` keys for interface declarations whose site paths were not tracked. |

Unverified entries store contract keys, not paths or secret findings. If one declared site for a contract is missing, the indexed edge can be marked `site_unverified` from either endpoint. A later import with tracked sites clears the mark by replacing the stamp.

Legacy stamps may omit `origin` and `docs_dir`. A missing `docs_dir` is interpreted as the currently configured root docs directory when checking a stamp that has an origin. Missing or invalid stamp JSON is treated as no stamp and imported without ancestry/ownership comparison. Stamps are coordination metadata, not authenticated access controls.

## Commit ordering

After ownership validation:

| Relationship between imported stamp and current source HEAD | Result |
|---|---|
| No usable stamp | Import. |
| Same commit | `current`; no new copy. |
| Current HEAD is older than the stamp | `skipped`; preserve the newer import. |
| Stamp is an ancestor of HEAD | Import the newer source snapshot. |
| Stamp cannot be found even after an attempted fetch by SHA | Refuse unless `update --force`. |
| Stamp and HEAD diverged | Refuse unless `update --force`. |
| Stamp claims another origin or docs directory | Refuse, including with `--force`. |
| HEAD is not reachable from configured source default branch | Refuse, including with `--force`. |

`--force` also does not force an unchanged stamp to rebuild, and does not roll a newer import back to an older ancestor. Changing only config or a permalink template is not generally enough to re-import at an unchanged SHA; the specific umbrella target-set rule is an exception.

## Pinned source pointers

In Markdown body prose outside backtick/tilde fences, a tracked pointer such as:

```text
See `src/routes.rs:10-20`.
```

is rewritten to an inline Markdown link pinned to the full imported source SHA. GitHub uses:

```text
https://github.com/<owner>/<repo>/blob/<sha>/src/routes.rs#L10-L20
```

GitLab.com uses `/-/blob/` in place of `/blob/`. Other hosts default to the GitHub-shaped template. Existing ordinary links, YAML frontmatter, fenced code, and untracked pointer paths are left untouched by pointer rewriting.

A custom `.config` template can use `{host}`, `{owner}`, `{repo}`, `{sha}`, `{path}`, `{line}`, and `{line_end}`. For a range, `{line}` is the literal string `10-L20`; `{line_end}` is `20`. For one line, `{line_end}` is empty. The template is a raw string replacement, with no URL-encoding or provider validation.

For a target at `services/billing/docs/capstone`, when root docs_dir is `docs/capstone`, workspace-relative `src/routes.rs` can resolve to `services/billing/src/routes.rs`. An exact source-root path wins if both exist. This workspace base is inferred only when the target docs path ends in `/<root_docs_dir>`.

## Site and secret gates

Site verification reads `site`/`Site` on parsed interface declarations and checks whether its path is tracked at the imported commit. It strips a numeric `:line` or `:start-end` suffix. It does **not** check that line numbers exist, the function implements the contract, or the endpoint is live. A page with frontmatter `mode: prescriptive` skips site verification, because planned paths may not exist yet.

Secret scanning applies to **every copied file**, including non-Markdown and frontmatter, and is not skipped for prescriptive pages. The six pattern names are `aws-access-key`, `github-token`, `slack-token`, `stripe-key`, `google-api-key`, and `private-key`. Findings contain the file and pattern name, never the matching value. See the exact patterns in [Implementation details](12-parser-details-and-implementation-limits.md).

Without `--strict`, these findings become notes and files are imported unchanged. With `--strict`, any match refuses the new import. This is a small pattern detector, not a complete credential scanner. A current/skipped import has no pending build and therefore does not re-scan old content, even with `--strict`.

## Shared root catalog

Quarry regenerates root `00-index.md` from immediate child directories containing their own `00-index.md`. Rows are alphabetically ordered and contain:

| Column | Meaning |
|---|---|
| Repo | Link to `<folder>/00-index.md`. |
| Imported | First seven characters of the source stamp commit, or `-`. |
| Newest doc | Lexically greatest string `generated_date` among that folder's Markdown pages, or `-`. |
| Pages | Recursively counted lowercase `.md` files. |
| Produces / Consumes | Parsed declaration counts, not deduplicated graph-edge counts. |

There is no age-based pruning. Repos that stop importing retain their last folder and date. Root `observed-edges.json` is not generated by Quarry.

## Push retries and parallel writers

Each source worktree has its own clone. When independent clones push competing changes, Git rejects a non-fast-forward push. Quarry detects stderr containing `rejected` or `fetch first`, refreshes/reset-cleans its clone, repeats ownership/ancestry planning against the new remote snapshot, rebuilds pending imports, regenerates the root index, and retries.

There are **at most three push attempts in total**, not three retries after the first attempt. A competing newer source stamp may turn the retry into a successful `current` or `skipped` result. Exhaustion returns exit 2 with `docs repo busy, retry`. Other push failures trigger a refresh attempt then an external error; if cleanup fetch also fails, that fetch error can be the reported failure.

There is **no Quarry lock for concurrent commands sharing one `.quarry/` directory**. The SQLite rebuild also uses a fixed `.docs-index.sqlite.tmp` filename. Do not run import/sync/index commands concurrently in the same worktree; serialize them or use separate worktrees/clones. The retry logic handles independent Git push races, not every local filesystem race.

The clone is managed scratch space: refresh uses fetch, `reset --hard FETCH_HEAD`, and `clean -qfd`. Use a separate clone to maintain shared root files or reviewed manual changes. The source code worktree itself is not reset by these operations.

## Removal scope

`remove` refreshes the docs clone, identifies currently configured folders, deletes those that exist, regenerates the root catalog, commits `remove <names>`, pushes with the same retry mechanism, and rebuilds locally. It does not check source-HEAD default-branch ancestry or run import ownership-stamp planning. Treat it as an explicitly authorized repository maintenance operation, not an ownership enforcement boundary.

Its `dangling` result lists other endpoint repos of indexed declared edges touching a removed folder, excluding sibling folders removed in the same run and excluding observed-only edges. Other repos' docs are not edited to remove those declarations. Running it again when folders are absent returns a successful no-op.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [src/importer.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/importer.rs) [src/docsrepo.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/docsrepo.rs) [src/config.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/config.rs) [src/secrets.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/secrets.rs) [tests/s01_update_ordering.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s01_update_ordering.rs) [tests/s18_site_verification.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s18_site_verification.rs) [tests/s19_secrets.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s19_secrets.rs)
