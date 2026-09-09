# Writing pages, interfaces, aliases, and observed edges

Quarry accepts ordinary documentation as long as the configured docs folder contains `00-index.md`. It recursively indexes lowercase `.md` files. Capstone uses numbered chapters and `generated_date` frontmatter, but Quarry does not require a particular number of chapters or require every page to have frontmatter.

## Page format

Illustrative page:

```markdown
---
generated_date: "2026-09-09"
known_as:
  - records.internal
  - records-api
---

# record-store

Stores record metadata.

## Storage

Records are written to the primary database.

## Recovery {#recovery}

Replay the file-ingest queue after repairing the failure.
```

`generated_date` is page metadata. `known_as` declares alternate endpoint names for this imported repository/target. Other frontmatter is retained as JSON in the `pages` table, but Quarry does not interpret arbitrary Capstone coverage fields as freshness checks.

A top-level `---` opening fence is required for YAML; LF and CRLF are supported. `---` or `...` closes it. Missing frontmatter is valid. Invalid YAML or a nonmapping frontmatter value produces an index warning and empty metadata while the body can still be indexed. An unclosed frontmatter block is treated as body text rather than parsed YAML.

The section parser recognizes `#` through `######` headings followed by a literal space. Text before the first heading is not put into a searchable section. A section ends at the next recognized heading at **any** level: a parent section does not include its child sections. Headings inside backtick/tilde fences are ignored. Fenced text under a real heading remains part of that section's body and is searchable.

## Declare interfaces in YAML

The preferred explicit schema is an `edges` mapping. It can live in the frontmatter of any indexed Markdown page, though `09-interfaces.md` is the normal location and the one used for contract payloads.

```yaml
---
generated_date: "2026-09-09"
edges:
  produces:
    - kind: http
      name: "GET /records"
      to: report-builder
      site: src/routes.rs:10-25
      schema: Record
    - kind: sqs
      name: file-ingest
      to: [record-store, report-builder]
      site: src/publish.rs:50
  consumes:
    - kind: http
      name: "GET /users/{id}"
      from: identity.internal
      site: src/auth.rs:12
      schema: UserView
---
```

| Field | Applies to | Meaning |
|---|---|---|
| `kind` | Every row; required | Protocol/resource label. Trimmed and ASCII-lowercased. Values are not restricted to an enum. |
| `name` | Every row; required, with `endpoint` fallback | Contract/resource name; trimmed. |
| `to` | Produces | Consumer repo/alias, an array of them, `unknown`, or omitted for name joining. |
| `from` | Consumes | Producer repo/alias, an array of them, or omitted for name joining. |
| `site` | Optional | Source path or path with numeric line suffix, verified at import. |
| `schema` | Optional | Local entity heading in `02-models.md` used by `check`. |

A scalar endpoint creates one declaration; an endpoint array creates one declaration per unique trimmed endpoint. An empty array leaves the endpoint open for name joining. Blank/nonstring items in arrays warn and discard that row. Fields such as `kind`, `name`, `site`, and scalar endpoints use a helper that accepts strings or numbers (numbers become strings); prefer strings for predictable documents. Unsupported scalar types can become absent, and missing endpoints then mean name joining.

For a page using the older form, `produces` and `consumes` may instead be top-level frontmatter keys:

```yaml
---
produces:
  - {kind: sqs, name: file-ingest, to: record-store}
consumes:
  - {kind: http, endpoint: "GET /users/{id}", from: identity-api}
---
```

Do not duplicate the same facts across forms expecting them to merge.

## Declare interfaces in Markdown tables

When frontmatter has neither a valid `edges` mapping nor top-level `produces`/`consumes` keys, files whose basename is `09-interfaces.md` are scanned for tables under headings normalized to `Produces` and `Consumes`.

```markdown
# Interfaces

## Produces

| Kind | Name | To | Site | Schema |
|---|---|---|---|---|
| http | GET /records | [report-builder](../report-builder/09-interfaces.md) | `src/routes.rs:10` | `Record` |
| sqs | file-ingest | - | `src/publish.rs` | `FileIngestMessage` |

## Consumes

| Kind | Name | From | Site | Schema |
|---|---|---|---|---|
| http | GET /users/{id} | identity.internal | `src/auth.rs` | `UserView` |
```

Headers are matched by name, case-insensitively. `Endpoint` can replace `Name`, and `Repo` can replace `To`/`From`. Surrounding backticks are stripped, and a Markdown link contributes its **label**, not the linked path, as the repo name. The parser supports escaped `\|` within a cell.

A table needs a `To`/`From` column or a `Repo` column even if its cells are empty. Empty and lone `-` cells leave the endpoint open. `unknown` is not the same as an empty endpoint. Markdown tables do not provide the YAML endpoint-array expansion; put separate rows in the table to name multiple endpoints.

Only the first unfenced pipe table in a matched section is used. Keep the interface table directly below `## Produces` or `## Consumes`, before a child heading. The parser does not search every table or parse a full CommonMark AST.

## Precedence rules

1. A mapping at frontmatter `edges` wins, even if empty.
2. Otherwise, the presence of either top-level `produces` or `consumes` selects the legacy frontmatter form for the entire page.
3. Otherwise, an `09-interfaces.md` basename permits Markdown table parsing.

An invalid nonmapping `edges` value warns and falls through. A valid empty `edges: {}` intentionally suppresses tables. A malformed legacy list warns but still suppresses table fallback. This is often why a visible table fails to produce edges.

Edge counts on repo listings count parsed declarations, including duplicates, endpoint-array expansion, and open rows. They are not the number of merged graph edges.

## Open endpoints and name joining

Omit `to`/`from`, use an empty array, or leave the table endpoint cell empty/`-` when the source documentation knows a contract but not the other repository:

Producer:

```yaml
edges:
  produces:
    - {kind: http, name: "GET /records"}
```

Consumer:

```yaml
edges:
  consumes:
    - {kind: http, name: "GET /records"}
```

During a full index build, each open consumer row searches open producer rows in other repos by `(kind, normalized name)`:

| Candidate producer repos | Result |
|---|---|
| Exactly one | A real graph edge with `declared_by: "joined"`, `resolved_by: "name"`. |
| More than one | No joined edge; an ambiguity report names the consumer and all candidate producers. |
| None | No edge; the unmatched open consumer is not added to the explicit unresolved-target report. |

One producer may join to many consumers. A producer row no consumer claims becomes a publication with unknown consumers. Self-joins are excluded.

For `http`, `ws`, `wss`, and `grpc`, route joining lowercases the first whitespace-separated token, collapses token whitespace, and replaces complete parameter segments such as `{id}`, `:id`, or `<id>` with `{}`. For example, `GET /users/{id}`, `get /users/:userId`, and `GET /users/<id>` share a join key. Other path text remains case-sensitive. Other kinds compare the trimmed name as written; `file-ingest` and `File-Ingest` are distinct queue names.

An explicit `to`/`from` does not participate in the open-endpoint join. Its identity is trusted as written after repo/alias resolution. In particular, explicit counterpart routes with different parameter spellings are not automatically merged into the same edge. Keep explicit declarations' `kind` and `name` consistent across both ends.

## `unknown` and by-name leads

A **produces** row explicitly naming `to: unknown` is stored as a publication, not an edge to a repo called `unknown`. Matching is case-insensitive for this sentinel. It does not participate in name joining, even if another page has an open consumer row with the same name.

For downstream traversal, Quarry searches a publication's literal FTS phrase in other repos' root `09-interfaces.md` sections. Matches are reported as possible consumers with `(by name only)` and `by_name: true`. The mention can be prose or a produced interface; Quarry has not established that it is a consumer. These leads are never expanded, never used by `docs path` or `check`, and never included in the dependency `repos` count.

A consumes row explicitly naming `from: unknown` is different: it is retained as a missing endpoint edge, and the special name is suppressed from the ordinary unresolved warning list. Prefer an omitted endpoint if you want the name join to find the producer.

## Aliases

Declare names such as deployment identifiers and internal hostnames in any imported Markdown page:

```yaml
known_as:
  - records.internal
  - records-api
```

Aliases are combined across the folder's pages. Repo listings in JSON and `docs show` expose the declared names. Endpoint resolution uses this order:

1. Exact folder name.
2. Case-folded folder name, if exactly one folder has that folded name.
3. Case-insensitive accepted alias.

A folder name wins over another repo's alias claim. An alias claimed by several repos is ignored and reported as a warning. Case-only folder collisions retain exact-name lookup but disable their ambiguous folded lookup. An alias identical to its own folder name is redundant and ignored by the registry.

A resolved endpoint that differs from its spelling in the declaration carries `as_declared`; human output shows `(declared as ...)`. Aliases do **not** rename folders and are not accepted as CLI repo arguments: use `quarry docs show record-store`, not `quarry docs show records.internal`.

## Observed traffic file

Quarry can merge externally measured relationships from **`observed-edges.json` at the shared docs repository root**:

```json
{
  "generated_at": "2026-09-09T08:00:00Z",
  "edges": [
    {
      "from": "record-store",
      "to": "report-builder",
      "kind": "http",
      "name": "GET /records",
      "last_seen": "2026-09-09T07:58:00Z"
    }
  ]
}
```

`from` means producer and `to` means consumer, matching Quarry's graph direction. For HTTP request traces this is typically server-to-client, so transform caller/callee data accordingly before exporting it.

The top level must be an object with an `edges` array. Each row needs `from`, `to`, `kind`, and `name`; `last_seen` is optional. `generated_at` is optional and only accepts a nonempty string. Unknown fields are ignored. Invalid JSON, an invalid top level, or invalid rows become index warnings; valid rows can still be indexed.

Endpoints resolve through the same folder/alias registry. A row with either endpoint unregistered is skipped with a warning. Kind is lowercased. The merged key is the exact `(from, to, kind, name)` after endpoint resolution; observed names do not receive route normalization. Keep the spelling aligned with declared edges to avoid creating a separate observed-only edge.

Repeated matching observations merge into one graph edge and keep the lexically greatest `last_seen`. Use consistently formatted ISO dates/timestamps. The reported observed row count counts accepted input rows, including duplicates, rather than unique edges.

| State | Result |
|---|---|
| Declared edge matched by an observation | `observed: true`; optional `last_seen`. |
| Observation with no matching declaration | `declared_by: "observed"`; traversable edge marked `(observed, undeclared)`. |
| Declared, nonmissing edge not observed while file exists | Human marker `(declared, never observed)`. |
| No observed file | No “never observed” claim; `observed_file: false`. |

The file's existence does not establish complete or recent traffic coverage. Even an invalid existing file can set `observed_file: true` with zero accepted rows. Read the forced index warnings and generation time before interpreting “never observed.”

Quarry never writes this file. Commit updates from your exporter through a separate docs-repository checkout, then synchronize readers. An untracked file placed in Quarry's managed clone may be removed by refresh; changing it without moving docs `HEAD` also does not trigger automatic index invalidation.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/frontmatter.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/frontmatter.rs) [src/index.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/index.rs) [src/observed.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/observed.rs) [src/query.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/query.rs) [tests/s06_edges.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s06_edges.rs) [tests/s15_aliases.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s15_aliases.rs) [tests/s16_observed.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s16_observed.rs) [tests/s20_name_join.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s20_name_join.rs)
