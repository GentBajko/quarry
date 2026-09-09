# Quarry documentation

Quarry brings the documentation of many Git repositories into one shared Git repository and makes it queryable through a local CLI. This manual covers Quarry **0.2.0** at source revision **78c71fab5f5818662dc02d5867246cec963567a9**.

Start with [First import and recorded walkthrough](02-first-import-and-recorded-walkthrough.md) to connect a source repo and publish its docs. If you only need to read an existing shared repository, see the [installation and read-only setup](01-overview-and-installation.md), then [query commands](06-reading-searching-and-traversing.md).

## Read by task

| Task | Chapter |
|---|---|
| Understand the components and install/build the CLI | [Overview and installation](01-overview-and-installation.md) |
| Publish a repo and see real query/check output | [First import and recorded walkthrough](02-first-import-and-recorded-walkthrough.md) |
| Configure URLs, paths, default branch, and monorepos | [Configuration and targets](03-configuration-and-targets.md) |
| Understand exactly what imports overwrite/generate | [Imports, files, ownership, and concurrency](04-imports-ownership-and-concurrency.md) |
| Author the Markdown/YAML and traffic input | [Pages, interfaces, aliases, and observed edges](05-writing-pages-and-interfaces.md) |
| List, retrieve, search, traverse, and find a path | [Reading and graph queries](06-reading-searching-and-traversing.md) |
| Inspect SQLite schema/SQL and diagnose freshness | [SQLite index and freshness](07-sqlite-index-and-freshness.md) |
| Check a proposed producer change against consumers | [Contract checking](08-contract-checking.md) |
| Publish after merge and audit the shared repository | [CI and automation](09-ci-and-automation.md) |
| Parse JSON, handle exit status, and look up flags | [Command/output reference](10-command-output-and-exit-reference.md) |
| Resolve a specific failure or confusing result | [Troubleshooting](11-troubleshooting.md) |
| Find parser details and less obvious limits | [Implementation details](12-parser-details-and-implementation-limits.md) |

## Evidence and examples

Behavioral descriptions were checked against implementation and integration tests, including the CLI, config, importer, Git retry logic, frontmatter parser, SQLite schema, queries, contract comparison, and workflow templates. Each chapter ends with links pinned to the source revision used.

Blocks labeled **Recorded output** came from an executed local Git fixture using the committed four-repository estate documents and a binary built from this revision. They are not screenshots or invented demonstrations. The fixture used local bare remotes and did not publish to a production repository. The documentation verification retained a local evidence log of commands, working directories, stdout, stderr, exit codes, generated config/ignore/stamp, and root catalog. The readable recorded blocks in this manual are drawn from that run.

Other sample commands, page contents, URLs, and workflow fragments are **illustrative** and need your repository names, paths, permissions, and release choices. Source tests demonstrate intended scenarios; the guide labels test definitions separately from tests actually executed during documentation verification.

## The distinctions to keep visible

An import copies **committed source HEAD**, after checking that it is reachable from the configured default branch. A contract check reads **producer working-tree docs** and **imported consumer docs**. A plain query reads the **local combined snapshot**. These boundaries explain most freshness and no-op questions.

Repository names come from source origins or explicit monorepo targets. Alias matching resolves graph endpoints; CLI repo arguments still use canonical folder names. Name-joined edges, traffic-observed edges, and by-name search leads carry different evidence and should not be treated as interchangeable.

The SQLite database and managed docs clone are disposable local state. Authored work belongs in source docs or a separate shared-repository checkout. Serialize commands sharing one `.quarry/`; the independent-clone Git push retry mechanism is not a shared-worktree lock.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [Cargo.toml](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/Cargo.toml) [src/cli.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/cli.rs) [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [tests/s21_estate.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s21_estate.rs)
