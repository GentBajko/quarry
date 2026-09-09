# Checking documented contracts before a merge

`quarry check` compares the fields documented by a producer against the fields its imported consumers say they read. Run it in the producer's source worktree before merging a contract change:

```sh
quarry check
quarry --sync check
quarry --json check
```

The producer input is the **working-tree** `09-interfaces.md` and optional `02-models.md` in each configured docs directory. The consumer input is each registered consumer's **imported** root `09-interfaces.md` and optional `02-models.md` in the shared clone. The indexed graph determines which consumers/contracts are considered.

No code is executed, no endpoint is called, and no source types are parsed by this command. A clean result means no documented break was detected among the usable recorded inputs, not that runtime compatibility or consumer coverage was proved.

## Inline field tables

Illustrative producer `09-interfaces.md`:

```markdown
---
generated_date: "2026-09-09"
---

# Interfaces

## Produces

| Kind | Name | To |
|---|---|---|
| http | GET /records | report-builder |

### GET /records

| Field | Type | Required |
|---|---|---|
| id | string | yes |
| content_type | enum | yes |
| created_at | string | yes |
```

Illustrative consumer page:

```markdown
---
generated_date: "2026-09-08"
---

# Interfaces

## Consumes

| Kind | Name | From |
|---|---|---|
| http | GET /records | record-store |

### GET /records

| Field | Type | Required |
|---|---|---|
| id | string | yes |
| content_type | enum | yes |
```

The consumer intentionally records only the fields it uses. Removing producer `created_at` does not break this consumer's recorded contract; removing `content_type` does.

Payload sections must be level 3 or deeper beneath an exact normalized level-2 `## Produces` or `## Consumes`. A new level-1 or level-2 heading ends that parent region. The first matching section is used: exact normalized contract name first, then a heading beginning `<name> (` such as `### file-ingest (v2)`. Surrounding backticks on contract headings/names are stripped for this lookup.

The parser reads the first unfenced table in that section body. Header order does not matter, but it needs a `Field` column. `Type` and `Required` are optional. Put the payload directly in the contract section; a child heading creates another section and can detach the table from the named contract.

## Use a model table

A `schema` on the interface row can point to a type in the same repository's `02-models.md`:

```yaml
edges:
  produces:
    - {kind: sqs, name: file-ingest, schema: FileIngestMessage}
```

Equivalent table form:

```markdown
## Produces

| Kind | Name | To | Schema |
|---|---|---|---|
| sqs | file-ingest | - | `FileIngestMessage` |
```

Illustrative `02-models.md`:

```markdown
# Models

## Messages

### FileIngestMessage

| Field | Type | Required |
|---|---|---|
| file_id | string | yes |
| content_type | enum | yes |
```

Alternatively, put a `Model:` line in the contract section:

```markdown
### file-ingest

Model: FileIngestMessage
```

A row's `schema` takes precedence over a section's `Model:` line. `FileIngestMessage[]` resolves to the same entity heading as `FileIngestMessage`. Model headings must be level 3 or deeper; matching uses the same exact-then-parenthesized-suffix rule. Only one direct table is read; this is not a recursive schema resolver.

Consumers may name their own local schema, such as `IngestedFile` or `RecordView`. Quarry reads the consumer's model file in its imported folder. Producer and consumer do not need to agree on a type name, only on the fields and compared values.

When an inline field table and a named model both exist, the inline table wins and a warning says so. A producer model that cannot be resolved is a warning and leaves nothing to compare. A missing consumer model produces the “lists no fields” note, not a synthetic break. With `schema`, the model can work even when there is no separate contract subsection.

## Verdict rules

The comparison loops over the consumer's fields in table order and looks up exact producer field names.

| Situation | Verdict | Exit impact |
|---|---|---|
| Consumer field has no exact producer field name | Break: `no longer produced`. | Exit 1. |
| Both sides specify a type and normalized strings differ | Break: `type changed: <producer> (consumer reads <consumer>)`. | Exit 1. |
| Both sides specify Required and Boolean values differ | Warning: `required flipped: ...`. | None. |
| Producer has additional fields the consumer does not list | No finding. | None. |
| Type missing/blank on either side | No type comparison for that field. | None. |
| Required column/cell missing on either side | No required comparison for that field. | None. |

Field names are case-sensitive and are not interpreted as nested object paths. Types are trimmed, lowercased, and internal whitespace collapsed; they are compared as strings, not understood as a type system. `string`, `string (uuid)`, and `uuid` remain different normalized strings. Union ordering, enum members in Notes, numeric bounds, array element schemas, and backward-compatible widening are not semantically evaluated.

In a present `Required` cell, only case-insensitive `yes` or `true` means true; every other string means false, including a blank cell, `no`, `optional`, or a typo. Prefer consistent `yes`/`no`. A missing Required column is distinct from a present empty cell.

## Which contracts are checked

The check starts from indexed outgoing edges for this producer. It excludes missing endpoints, self-edges, and observed-only edges. Joined edges are eligible when the working-tree producer contract still exists. By-name leads are never eligible.

The producer's current declaration is matched by exact `kind` and `name` to that indexed contract. A newly added producer row with no indexed consumer edge has nothing to compare. Consumers are not discovered by searching the working tree or by live traffic during the check.

This produces several important missing-data behaviors:

- An unregistered producer returns a note to run `quarry add`, with exit 0.
- A producer with no eligible consumer edges returns “nothing to compare,” with exit 0.
- A current produced row without a payload table/model yields a warning and skips field comparison for it.
- A consumer without the matching contract subsection/model yields a note and no field comparison.
- A consumer with an empty Field table records an empty read set and produces no field findings.
- A produced contract absent from current docs is treated as removed only when the indexed group includes `declared_by: producer` or `both`. Its former consumers' recorded fields then compare against an empty produced set.
- If that absent contract is known only through `consumer` or `joined` edges, it produces notes that consumers declare it but it is not in Produces, rather than a removed-contract break. Deleting an entire name-joined contract can therefore evade a breaking verdict in this revision.

A missing producer `09-interfaces.md` can still lead to removal checks for previously producer-declared edges; the command also reports why the page is missing. Configured targets with no working-tree interfaces page are skipped with a target-specific note. The umbrella never participates.

For a monorepo, producer docs are read from each target's working tree, but sibling consumers are still read from their **imported** snapshots. Editing both sides locally is not a simultaneous whole-worktree compatibility check.

## Recorded clean and breaking results

**Recorded output**, from the local four-repo estate, exit **0**:

```text
$ quarry check
ingest-api produces sqs file-ingest (fields from 02-models.md § FileIngestMessage)
  record-store reads file_id, content_type   (2026-09-07)
  report-builder reads file_id   (2026-09-07)
no breaks
```

After deleting only the producer model row `content_type`, **recorded output**, exit **1**:

```text
$ quarry check
ingest-api produces sqs file-ingest (fields from 02-models.md § FileIngestMessage)
  record-store reads file_id, content_type   (2026-09-07)
  break: content_type no longer produced
  report-builder reads file_id   (2026-09-07)
1 break
```

The corresponding **recorded break object** from `--json check`, reformatted only:

```json
{
  "consumer": "record-store",
  "field": "content_type",
  "kind": "sqs",
  "name": "file-ingest",
  "reason": "no longer produced"
}
```

A completed check with breaks still has top-level JSON `ok: true`. Inspect both the process exit status and `result.breaks`. In a targeted monorepo, each contract/break also has its producing `target` field. Warning and note vectors remain present even when empty.

## Recorded whole-contract deletion limitation

In the same fixture, replacing the producer interfaces declarations with `edges: {}` removed the entire queue declaration. Its cached edges were name-joined, so the current implementation emitted notes rather than field-removal breaks.

**Recorded output**, exit **0**:

```text
$ quarry check
note: record-store declares sqs file-ingest from ingest-api; not in ingest-api's Produces table
note: report-builder declares sqs file-ingest from ingest-api; not in ingest-api's Produces table
no breaks
```

This is why a CI policy that reads only the exit code cannot treat every missing contract as covered. It also shows why deleting one field and deleting the entire declaration can have different verdicts in this revision.

## Import-time advice is not the CI gate

After an actual `add`/`update` import, Quarry runs a comparison against the imported producer pages and emits breaks as result notes such as `contract check: ...`. It has already pushed the docs. These notes do not alter a successful import's exit code. Same-commit no-op imports do not run that post-import comparison.

Use `quarry check` in a pull-request job to turn recorded field breaks into a merge signal. For a network-required gate, run explicit `quarry sync` first and allow its failure to fail the job; `--sync check` intentionally falls back to local docs if its refresh fails. See [CI and automation](09-ci-and-automation.md).

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/check.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/check.rs) [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [src/frontmatter.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/frontmatter.rs) [src/output.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/output.rs) [tests/s14_check.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s14_check.rs) [tests/s17_targets.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s17_targets.rs) [tests/s21_estate.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s21_estate.rs)
