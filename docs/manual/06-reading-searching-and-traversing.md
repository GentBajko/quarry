# Reading, searching, and traversing dependencies

All queries operate from the managed shared-repository clone and its local index. By default they do not fetch. Add `--sync` to attempt a refresh before a query, or set `sync_on_read` in config. When that refresh fails, Quarry can still answer from the local clone and adds a note.

Every repo argument must match a canonical imported folder name exactly, including case. `known_as` helps resolve **declared endpoints**, not command arguments. Start with `quarry docs list` when you do not know the name.

## List repositories or files

```sh
quarry docs list
quarry docs list record-store
quarry --json docs list
```

Without an argument, the human table shows `repo`, `newest`, `pages`, `produces`, and `consumes`, ordered by name. JSON includes each repo's source `commit`, normalized `origin`, `newest_generated_date`, and `known_as` list as well.

With a repo argument, the command prints that repo's source stamp and its recursively indexed Markdown paths with page dates, ordered by path. It does not list copied images or other non-Markdown assets. No registered repositories is a successful empty answer. An unknown repo is a refusal, exit 1.

## Show an overview

```sh
quarry docs show record-store
quarry --json docs show record-store
```

`show` returns the repo stamp/date/aliases, its produced and consumed edges, publications with unknown consumers, and the **first indexed section** from `00-index.md`. It does not retrieve the entire index page or dynamically follow chapter links. If the first section is only a title with a short introductory paragraph, that is the overview shown.

The JSON result has `repo`, `produces`, `consumes`, `publications`, `overview`, and `observed_file`. `repo` is a metadata object, not just the name string. Edge metadata preserves declaration sources in `via`; observed-only rows may have an empty `via` list.

## Retrieve one section

```sh
quarry docs section record-store "GET /records"
quarry docs section record-store "Storage"
quarry --json docs section record-store "Recovery"
```

Matching normalizes the request and heading by trimming, collapsing whitespace, lowercasing, and removing a trailing `{#anchor}`. It first looks for exact normalized matches across **all pages in that repo**. Only if none exist does it try a prefix. Exactly one candidate returns a section; zero candidates refuse with up to five nearest-prefix suggestions; multiple candidates refuse and list their file/heading pairs.

There is no `--file` option to break a tie. A longer unique heading can help with prefix ambiguity; duplicate exact headings in different pages require opening the named files or changing headings in source docs.

A returned section body stops at the next heading of any level, so `## Produces` returns its own table/prose, not every `###` contract below it. The query returns the stored body and metadata; it does not expand `Model: Record` into a model table. Fetch the `Record` heading separately when needed.

**Recorded output** from the four-repo fixture:

```text
$ quarry docs section record-store "GET /records"
record-store/09-interfaces.md § GET /records   2026-09-07


Model: Record
```

Leading blank lines are preserved in section bodies. The apparent extra blank line above is present in the actual rendering.

## Search section text

```sh
quarry docs search "file-ingest"
quarry docs search "GET /records" --repo record-store
quarry docs search "content type" --limit 5
quarry --json docs search "file-ingest" --repo record-store --limit 1
```

Search covers indexed heading and body text, not frontmatter values, repo names, or path columns. `--repo` applies an exact canonical folder filter. `--limit` is an unsigned 32-bit value, default 20; 0 returns no rows. There is no offset/pagination flag or total-match count. The human closing `N hits` means returned rows after the limit.

The supplied term is escaped and wrapped as **one literal FTS5 phrase**. Quarry does not expose FTS Boolean syntax, wildcard syntax, raw SQL, semantic embeddings, or a regex search language. For example, `file-ingest OR (broken` is tokenized as a phrase rather than executed as a Boolean query.

“Literal phrase” still means SQLite's `unicode61` tokenization, not an exact byte substring: punctuation contributes token boundaries, and FTS case/diacritic handling applies. Search can match a heading even if the returned body snippet does not contain the phrase.

Rows are ordered by `bm25(sections)`, then repo and path. JSON includes a body `snippet` produced by FTS5 with a 12-token target window and ` … ` between excerpts; embedded newlines become spaces. Human output only lists repo, file, heading, and date. It does not print that snippet.

**Recorded JSON result item**, extracted without changing values from the fixture's `--limit 1` search:

```json
{
  "file": "09-interfaces.md",
  "generated_date": "2026-09-07",
  "heading": "file-ingest",
  "repo": "record-store",
  "snippet": " Model: IngestedFile"
}
```

Zero hits is a successful result with exit 0 and JSON `result: []`.

## Walk downstream or upstream

```sh
quarry docs deps ingest-api --downstream
quarry docs deps ingest-api --downstream --depth 2
quarry docs deps ingest-api --downstream --depth 0
quarry docs deps report-builder --upstream --depth 0
quarry --json docs deps record-store --downstream
```

Exactly one of `--downstream` or `--upstream` is required. Downstream follows producer → consumer; upstream follows consumer → producer. The default maximum depth is 1. `--depth 0` is unlimited.

Traversal is breadth-first. Each repo is expanded at most once; every encountered edge can still print, so parallel contracts or converging routes do not disappear. Edge rows are ordered within each expansion by counterpart repo, kind, and name. A loop back along the recorded traversal ancestry, or a self-edge, is marked `(cycle)`; a second contract to the same already-discovered repo is not automatically a cycle.

Missing endpoints are printed as `(not in quarry)` and are not expanded. A limited walk is `truncated` when a reached boundary node still has edges in the requested direction, or downstream publications. That indicates unexpanded information, not necessarily a new distinct reachable repo beyond the limit.

The closing `repos` count is the number of distinct counterpart names in non-by-name output rows. It can include a missing endpoint and can include the starting repo if a cycle returns to it. It is not a count of all unique registered repos visited excluding the start. `max_depth` is the greatest depth of a printed row; it may be lower than the requested limit.

## Read the markers

| Human marker | Meaning |
|---|---|
| `(declared by producer only)` | Only the producer supplied this exact endpoint/kind/name declaration. |
| `(declared by consumer only)` | Only the consumer supplied it. |
| No one-sided marker for an ordinary edge | Both declared it, or it is a joined/observed edge identified by other metadata. |
| `(resolved by name)` | A unique open producer matched an open consumer by contract key. |
| `(by name only)` | A publication phrase appears in another interfaces page; a lead, not a traversable edge. |
| `(declared as alias)` | Endpoint spelling was resolved to a different canonical folder. |
| `(site unverified)` | A source stamp marks this contract's site path untracked at import. |
| `(observed, undeclared)` | The relationship comes only from observed traffic input. |
| `(declared, never observed)` | Observed file exists, but no exact traffic row matched this declared nonmissing edge. |
| `(not in quarry)` | At least one endpoint lacks a registered folder. |
| `(cycle)` | This row closes a loop along the traversal ancestry. |

`deps` carries these details in JSON fields rather than asking scripts to parse labels. `show` shares alias/site/observation markers but does not reproduce every one-sided or missing marker from the `deps` renderer; use its JSON fields when that distinction matters.

No edges prints `no edges declared for <repo>; try quarry docs search`. That phrase is a generic empty-result message, not proof that a service has no runtime dependencies.

## Find a shortest directed path

```sh
quarry docs path ingest-api report-builder
quarry docs path report-builder record-store
quarry docs path record-store record-store
quarry --json docs path identity-api report-builder
```

Quarry first searches forward producer-to-consumer edges using breadth-first search. If any forward path exists, the shortest forward path wins. Only when no forward path exists does it search the reverse direction. It does not compare forward and reverse lengths or mix directions into an undirected path.

The JSON result has `from`, `to`, `direction`, and `path`:

| `direction` | `path` |
|---|---|
| `forward` | Ordered hops from requested `from` to requested `to`. |
| `reverse` | Ordered producer-to-consumer hops from requested `to` to requested `from`. |
| `same` | Empty array, when both arguments are the same registered repo. |
| `none` | `null`, when neither direction connects them. |

Each hop contains `from`, `to`, `kind`, and `name`. Path output does not include all evidence/observation/site annotations from `deps`; inspect the corresponding edges when provenance matters. Observed-only and joined edges are eligible; by-name leads are not. A nonexistent endpoint argument is a refusal, while no path between known repos is success with exit 0.

## Inspect index diagnostics

```sh
quarry docs index
quarry docs index --force
quarry --json docs index --force
```

This is always a local operation, even when `--sync` or `sync_on_read` is set. To refresh and get a complete new diagnostic report:

```sh
quarry sync
quarry docs index --force
```

Use the forced form for a reliable current list of YAML/alias/observed warnings and explicitly unresolved targets. A cached `index current` response preserves ambiguity and observed metadata but does not persist ordinary rebuild warnings or the unresolved list. See [The SQLite index](07-sqlite-index-and-freshness.md).

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/query.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/query.rs) [src/cli.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/cli.rs) [src/output.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/output.rs) [src/frontmatter.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/frontmatter.rs) [tests/s05_index.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s05_index.rs) [tests/s07_graph.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s07_graph.rs) [tests/s08_section_search.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s08_section_search.rs) [tests/s13_output.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s13_output.rs)
