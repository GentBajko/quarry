# Command, output, and exit-code reference

Use this reference for scripts and agents. Human output is designed for reading; JSON payloads carry provenance and flags without requiring text parsing.

## Complete command synopsis

```text
quarry [--json] [--verbose] [--sync | --offline] <COMMAND>

quarry init [--url URL] [--docs-dir DIR] [--name NAME]
            [--default-branch BRANCH] [--force]
quarry add [--strict]
quarry update [--force] [--strict]
quarry sync
quarry remove
quarry check

quarry docs list [REPO]
quarry docs show REPO
quarry docs section REPO HEADING
quarry docs search TERM [--repo REPO] [--limit N]
quarry docs deps REPO (--downstream | --upstream) [--depth N]
quarry docs path FROM TO
quarry docs index [--force]
```

Global flags can appear before or after subcommands. `--json` chooses the structured runtime result; `--verbose` echoes Git commands to stderr; `--sync`/`--offline` choose read refresh policy and conflict with one another. `--limit` defaults to 20 and `--depth` to 1; both accept unsigned 32-bit integers, with 0 meaning no rows for search and unlimited traversal for deps.

Use `quarry --help`, `quarry docs --help`, and `quarry <command> --help` for the installed version's CLI help. Bare `quarry` and `quarry docs` also show their help. `-h` is the help alias and `-V` is the root version alias. There are no user-defined command aliases, SQL subcommand, per-file retrieval flag, or per-target operation selector in this revision.

## Exit codes

| Code | Meaning | Examples |
|---|---|---|
| `0` | Answered, successful write, or successful no-op/empty result | Current import; older import skipped; zero search hits; no path; check with warnings but no breaks. |
| `1` | Refusal, CLI usage error, or completed contract check with breaks | Unknown repo; ambiguous section; unmerged source commit; strict refusal; invalid flags; field removal. |
| `2` | External operation failed | Git transport/push error; filesystem/SQLite failure; stdout write failure. |

A check with breaks is a completed computation, not the same JSON shape as a refusal. Warnings and notes do not themselves make a successful command fail.

## Runtime success envelope

Every successfully computed `--json` runtime response has these top-level keys:

```json
{
  "built_at_commit": "DOCS_REPOSITORY_COMMIT_OR_NULL",
  "notes": [],
  "ok": true,
  "result": {},
  "synced_at": "LOCAL_SYNC_TIMESTAMP_OR_NULL"
}
```

That is an **illustrative schema**, not literal valid stamp values to copy. Indexed query/check responses have the docs-clone commit and available sync timestamp. `init`, `add`, `update`, `remove`, and explicit `sync` use a bare response and normally have `null` for both top-level stamp fields, even though their internal work can rebuild/update the cache.

Top-level `notes` always exists. Command-specific result notes can additionally occur inside `result.notes` or each write array element. Empty notes in write/init/sync result objects are omitted. Check's `contracts`, `breaks`, `warnings`, and `notes` arrays always exist, even when empty.

The program emits one compact JSON document followed by a newline. Key order is not a parsing contract. Use a JSON parser and named fields.

## Runtime error envelope

**Recorded output**, exit **1**, from a linked fixture repository:

```text
$ quarry --json docs show ghost
{"code":1,"error":"unknown repo ghost","ok":false,"result":null}
```

External runtime errors use the same shape with `code: 2`. Runtime errors go to stdout in JSON mode, and to stderr in human mode. Verbose Git traces remain on stderr.

## Parser/help exceptions to JSON

The `--json` envelope is implemented after clap parsing. An unknown command, unknown flag, missing argument, conflicting flags, or invalid number is rejected by clap **before** runtime output rendering. These errors print human usage/error text to stderr, with exit 1, even when `--json` is present. `--help` and `--version` also print ordinary text, exit 0.

Bare `quarry --json` and `quarry --json docs` take the runtime help path and return the normal success envelope with `result.help`. This differs from `quarry --json --help`.

For automation, supply a configured URL or `--url` to `init`; stdin being a terminal can trigger the URL prompt even in JSON mode. The integration test called “json never prompts” drives a nonterminal subprocess, so it does not establish a separate JSON-mode prompt guard.

## Result payloads

| Command | `result` shape | Principal fields |
|---|---|---|
| `init` | Object | `repo` (nullable), `url`, `docs_dir`, `default_branch`, absolute `clone`, `cloned`; optional `targets`, `notes`. |
| `add`, `update` without targets | Object | `repo`, `from` (nullable short source stamp), `to` (short source HEAD), `files`, inner `result`, optional `notes`. |
| `add`, `update` with targets | Array of write objects | Same fields per selected unit, including current/skipped units; optional umbrella is a further block. |
| `sync` | Object | `pulled`, `index` (`rebuilt`/`current`), `repos`, `pages`, `edges`, optional `notes`. |
| `remove` without targets | Object | `repo`, `removed`, `dangling`. |
| `remove` with targets | Array of remove objects | Same fields per configured target/existing umbrella. |
| `docs list` | Array | Repo metadata rows described below. |
| `docs list REPO` | Object | `repo`, full source `commit` or null, `files` array of `{path, generated_date}`. |
| `docs show` | Object | `repo` metadata object, `produces`, `consumes`, `publications`, `overview`, `observed_file`. |
| `docs section` | Object | `repo`, `file`, `heading`, `generated_date`, `body`. |
| `docs search` | Array | `{repo, file, heading, generated_date, snippet}` per returned section. |
| `docs deps` | Object | `repo`, `direction`, `edges`, `repos`, `max_depth`, `truncated`, `observed_file`. |
| `docs path` | Object | `from`, `to`, `direction`, `path` (hop array or null). |
| `docs index` | Object | `rebuilt`, `repos`, `pages`, `edges`, `warnings`, `unresolved`, `ambiguous`, optional `observed`. |
| `check` | Object | Source `repo`, `contracts`, `breaks`, `warnings`, `notes`. |

Write result status is `imported`, `current`, or `skipped`. `files: 0` on a current import means no files were copied in this run, not an empty registered repository. For explicit sync/index on a current cache, `pages`/`edges` can be zero because they are rebuild-report counters; inspect repo metadata or force an index rebuild for totals.

Repo metadata rows have `name`, `commit`, `origin`, `newest_generated_date`, `pages`, `produces`, `consumes`, and `known_as`. Nullable dates remain `null`; human views usually render `-` or an empty date column instead.

Stored edge objects exposed by `show` have `from_repo`, `to_repo`, `kind`, `name`, `declared_by`, `via`, `missing`, `site_unverified`, `observed`, and optional `as_declared`, `resolved_by`, `last_seen`. Dependency walk rows instead carry the counterpart `repo`, plus `depth`, `cycle`, and `by_name`; they do not repeat the complete `from_repo`/`to_repo` pair. `via` is a list of `<repo>/<page>` declaration paths.

`unresolved` objects have `declared`, `via`, and `nearest`. `ambiguous` objects have `direction`, `kind`, `name`, `repo`, and `candidates`; currently ambiguity is reported on an open consumer. `observed` has accepted `rows` and optional `generated_at`.

Check contracts have `kind`, `name`, optional `model`, optional producing `target`, and `consumers`. Each consumer has `repo`, `fields`, `generated_date`, `breaks` strings, and `warnings` strings. Top-level check break objects have `consumer`, `kind`, `name`, `field`, `reason`, and optional `target`.

## Recorded no-op result

**Recorded output**, exit **0**, after the local fixture's source commit was already imported:

```text
$ quarry --json update
{"built_at_commit":null,"notes":[],"ok":true,"result":{"files":0,"from":"8e7a6fd","repo":"ingest-api","result":"current","to":"8e7a6fd"},"synced_at":null}
```

`result.result` is intentional nesting: the outer key contains the command payload; the inner key is the write status.

## Useful JSON recipes

Illustrative commands requiring `jq`:

```sh
# Canonical names and their latest documented date.
quarry --json docs list | jq -r '.result[] | [.name, .newest_generated_date] | @tsv'

# Full retrieved body without the human heading/date wrapper.
quarry --json docs section record-store "Storage" | jq -r '.result.body'

# Evidence for each downstream relationship.
quarry --json docs deps record-store --downstream --depth 0 \
  | jq '.result.edges[] | {repo, kind, name, via, declared_by, observed}'

# Complete diagnostics rather than a cached abbreviated report.
quarry --json docs index --force \
  | jq '.result | {warnings, unresolved, ambiguous, observed}'
```

A pipeline's status may otherwise be the status of `jq`, not Quarry. In Bash, enable `set -o pipefail`, or capture Quarry's exit code before processing its output when refusals or contract breaks must fail the caller.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/cli.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/cli.rs) [src/output.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/output.rs) [src/main.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/main.rs) [src/errors.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/errors.rs) [src/query.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/query.rs) [src/index.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/index.rs) [src/check.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/check.rs) [tests/s13_output.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s13_output.rs)
