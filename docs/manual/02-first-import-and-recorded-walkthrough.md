# First import and recorded walkthrough

Use this sequence to contribute an existing source repository. Commands containing `acme`, example paths, or placeholder values are **illustrative commands** to adapt. Blocks explicitly marked **Recorded output** were executed against an isolated local Git fixture built from the revision cited at the end.

## Prepare the shared repository

Create a Git repository that every participating project can read and the publishing identities can push to. Quarry can clone and initialize an empty remote. Give it a conventional default branch such as `main`. It will contain one folder per source repo or configured target, plus a generated `00-index.md`.

The source repository needs its own `origin`, a commit on the configured default branch, and documentation in Git. The shared repository URL belongs in `.quarry/.config`; it does not replace the source repository's origin.

## Create the smallest useful source document

The default folder is `docs/capstone`. Its `00-index.md` makes the imported folder discoverable.

Illustrative `docs/capstone/00-index.md`:

```markdown
---
generated_date: "2026-09-09"
---

# record-store

Stores ingested files and serves record metadata.

## Chapters

| Topic | File |
|---|---|
| Interfaces | [Interfaces](09-interfaces.md) |
```

Use ISO-formatted date strings. Quarry displays `generated_date` but does not generate it or check whether it is truthful. You can start with only the index page, then add `09-interfaces.md` for graph and contract features and `02-models.md` for reusable field tables.

## Link and commit the configuration

```sh
quarry init --url git@github.com:acme/docs-quarry.git
```

For another docs directory or default branch:

```sh
quarry init --url git@github.com:acme/docs-quarry.git \
  --docs-dir docs/reference --default-branch trunk
```

`init` writes `.quarry/.config` and `.quarry/.gitignore`, clones or refreshes the shared repository, and builds the local index. It does not register this source repository in the shared repository. A note about missing source docs does not prevent linking for read-only use.

Review and commit the two generated configuration files along with your docs:

```sh
git add .quarry/.config .quarry/.gitignore docs/capstone
git commit -m "Document service and link Quarry"
git push origin main
```

Follow your repository's normal review/merge process if direct pushes are not used. The requirement for the next step is that the source `HEAD` is reachable from `origin/main`, or the branch set by `default_branch`.

## Register and import

```sh
quarry add --strict
```

The strict option refuses a newly built import if an interface `Site` path is absent from the source commit or a copied file matches a supported secret-shaped pattern. It is optional in the CLI; the reusable CI template enables it by default.

A successful import copies all tracked files below the configured docs directory, rewrites supported source pointers in Markdown, adds a `.quarry-stamp`, replaces this repository's folder, regenerates the shared root catalog, commits, and pushes.

After subsequent source changes have merged and your local `HEAD` is the desired reachable commit:

```sh
quarry update --strict
```

Running `add` again on the same imported commit is harmless: it notes that the repository already exists and returns `current`. `add` can also import a newer commit for an existing folder. `update` requires the folder to exist and otherwise instructs you to run `add`.

## Inspect another repository

```sh
quarry sync
quarry docs list
quarry docs show record-store
quarry docs list record-store
quarry docs section record-store "GET /records"
quarry docs deps record-store --downstream
quarry docs deps report-builder --upstream
```

`sync` refreshes your local shared clone. Ordinary queries thereafter need no network by default. Add `--sync` to a read command to attempt a refresh before answering.

## Recorded four-repository example

The following output was produced by a locally built Quarry 0.2.0 using the committed `tests/fixtures/estate/` documents. Four disposable source repositories had local bare origins and one local bare docs remote. Every fixture's cited `src/*.rs` path was tracked; each repository was registered with `add --strict`, and every local clone was synchronized. This run used no production repository or network service.

**Recorded output:**

```text
$ quarry docs index --force
index rebuilt: 4 repos, 12 pages, 4 edges
```

**Recorded output:**

```text
$ quarry docs show record-store
record-store @ 8e38765 (newest doc 2026-09-07)
produces: http GET /records -> report-builder (resolved by name)
consumes: sqs file-ingest <- ingest-api (resolved by name)

# record-store

Stores ingested files, serves records.
```

**Recorded output:**

```text
$ quarry docs deps ingest-api --downstream --depth 0
ingest-api
  sqs file-ingest -> record-store (resolved by name)
  sqs file-ingest -> report-builder (resolved by name)
    http GET /records -> report-builder (resolved by name)
2 repos, depth 2
```

There are three printed relationships and two distinct target repo names. `report-builder` receives the queue directly and also reads `record-store`'s HTTP response. The direct queue relationship is why a shortest-path query may be shorter than a visually plausible multi-hop chain.

**Recorded output:**

```text
$ quarry docs path report-builder record-store
reverse path (record-store produces for report-builder)
record-store -[http GET /records]-> report-builder
1 hops
```

The final line preserves the CLI's current wording. The requested direction has no forward path, so Quarry finds the reverse directed path and prints its actual producer-to-consumer order.

## Check before merging a producer change

```sh
quarry --sync check
```

This compares your working-tree producer docs against the imported consumers. It does not require the producer change to be committed or merged.

**Recorded output**, after removing `content_type` from `ingest-api`'s working-tree `FileIngestMessage` table:

```text
$ quarry check
ingest-api produces sqs file-ingest (fields from 02-models.md § FileIngestMessage)
  record-store reads file_id, content_type   (2026-09-07)
  break: content_type no longer produced
  report-builder reads file_id   (2026-09-07)
1 break
```

The process exited **1**. Restoring the field produced `no breaks` and exit 0. These are documentation comparisons; they do not validate runtime payloads or discover undocumented consumers.

## Leave the shared repository

From the contributing source repository:

```sh
quarry remove
```

This removes its imported folder, or all configured target folders plus an existing umbrella folder, commits and pushes the removal, and reports remaining declared relationships involving those folders. Configuration, local source docs, and the local `.quarry/` folder remain in place. Review [Import lifecycle](04-imports-ownership-and-concurrency.md) before changing target ownership or retiring only one target.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [tests/s21_estate.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s21_estate.rs) [tests/fixtures/estate/ingest-api/docs/capstone/09-interfaces.md](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/fixtures/estate/ingest-api/docs/capstone/09-interfaces.md) [tests/fixtures/estate/record-store/docs/capstone/09-interfaces.md](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/fixtures/estate/record-store/docs/capstone/09-interfaces.md) [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [src/docsrepo.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/docsrepo.rs)
