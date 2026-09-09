# Quarry: what it does and how to install it

Quarry copies documentation from many Git repositories into one shared documentation repository. Each source repository contributes a folder. Every contributor gets its own local clone and SQLite index for searching pages, following dependency edges, and checking documented contract fields.

Quarry does not generate prose or inspect source code to infer dependencies. It imports the files already present in a configured documentation folder. Capstone can produce that folder; hand-maintained documentation works when it follows the formats described in [Writing pages and interfaces](05-writing-pages-and-interfaces.md).

## Four places, four responsibilities

| Place | What it owns | Example |
|---|---|---|
| Source Git repository | Code and original documentation | `record-store/docs/capstone/` |
| Source repository's Quarry configuration | Link to the shared repository, source docs path, default branch, optional targets | `record-store/.quarry/.config` |
| Shared docs Git repository | Imported folders, import stamps, generated root catalog, optional observed edges | `docs-quarry/record-store/` |
| Local clone and index | Disposable retrieval data inside each source repository | `record-store/.quarry/docs-quarry/` and `.quarry/.docs-index.sqlite` |

An import pins a **source commit**. The index records a separate **docs-repository commit**. Those hashes answer different questions: which source code a folder documents, and which combined snapshot a query used.

The usual lifecycle is:

```text
source docs at a committed HEAD
  -> quarry add / quarry update
  -> shared docs repository
  -> quarry sync in another source repository
  -> local SQLite index
  -> quarry docs ... / quarry check
```

`quarry check` has an extra input: the producer's current working-tree documentation. This makes it useful before merging a change, while ordinary queries answer from imported docs.

## Install from the verified source revision

**As of 9 September 2026, the GitHub repository has no published releases or `v0.*` tags, and the README's `latest/download/quarry-installer.sh` route returns HTTP 404.** Use the source build below. The manifest's version `0.2.0` identifies the code; it does not mean a `v0.2.0` binary release has been published.

With Rust/Cargo and Git installed:

```sh
git clone https://github.com/GentBajko/quarry.git
cd quarry
git checkout 78c71fab5f5818662dc02d5867246cec963567a9
cargo install --path . --locked
quarry --version
```

Cargo normally puts the installed executable in its configured bin directory, commonly `~/.cargo/bin`; ensure that directory is on your `PATH`. To build without installing:

```sh
cargo build --release --locked
./target/release/quarry --version
```

The manifest declares Rust edition 2024 and `rust-version = "1.85"`. This guide's local source build used Rust/Cargo 1.91.1 and Git 2.43.0; it does not establish compatibility with every older compiler named by the manifest. The lockfile makes the dependency selection reproducible. The guide's recorded build used `cargo build --offline --locked` with dependencies already cached.

**Recorded output**, from that binary:

```text
$ quarry --version
quarry 0.2.0
```

The README specifies Git 2.30 or newer on `PATH`. Quarry invokes the installed `git` executable, using its credentials, transport, proxy configuration, and commit identity. It does not implement its own authentication prompt or token store.

## Release routes documented by the repository

The README lists shell/PowerShell installers, platform archives, and `cargo binstall quarry`. These are **repository-documented release routes, not verified currently working installation paths**. No published GitHub assets existed at the verification date, and a package-registry/binstall route was not established by this guide. Do not use the following installer commands as a working quickstart today.

Shell installer URL and its intended invocation, once a release exists:

```sh
curl -LsSf https://github.com/GentBajko/quarry/releases/latest/download/quarry-installer.sh | sh
```

PowerShell equivalent, once the corresponding asset exists:

```powershell
irm https://github.com/GentBajko/quarry/releases/latest/download/quarry-installer.ps1 | iex
```

The reusable CI template names a tag-specific URL ending in `/releases/download/v0.2.0/quarry-installer.sh`, which likewise requires a release that has not been published at this verification date. See [CI and automation](09-ci-and-automation.md) for the required source-build adaptation.

The release configuration names these intended targets:

| Platform | Target |
|---|---|
| Linux, x86-64, GNU | `x86_64-unknown-linux-gnu` |
| Linux, ARM64, GNU | `aarch64-unknown-linux-gnu` |
| Linux, x86-64, musl | `x86_64-unknown-linux-musl` |
| macOS, Apple silicon | `aarch64-apple-darwin` |
| macOS, Intel | `x86_64-apple-darwin` |
| Windows, x86-64, MSVC | `x86_64-pc-windows-msvc` |

This is a configured build matrix, not a list of currently downloadable artifacts. It includes GNU targets, so “a single binary” should not be read as “every build is fully statically linked.” SQLite is bundled into Quarry through `rusqlite`; normal use needs neither an external SQLite server nor its command-line program. There is no background daemon or model service. The release configuration disables an automatic updater.

## Start in the right directory

Run commands anywhere inside the source Git worktree. Quarry finds the root with `git rev-parse --show-toplevel` and uses the root's `.quarry/` folder. The error says to run in a repo's root, but nested working directories resolve correctly.

Bare `quarry`, bare `quarry docs`, `--help`, and `--version` can display help without initialization. Other commands need a Git repository. Contribution commands also need a usable `origin` URL and a committed source `HEAD`.

For a new project that only needs to read other repos' docs, initialize Git first, then link to the docs repository. An origin, source commit, source docs folder, and `quarry add` are unnecessary for read-only use:

```sh
mkdir new-project
cd new-project
git init
quarry init --url git@github.com:acme/docs-quarry.git
quarry docs list
```

Continue with [First import](02-first-import-and-recorded-walkthrough.md) for contribution, or [Reading and graph queries](06-reading-searching-and-traversing.md) for retrieval.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [Cargo.toml](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/Cargo.toml) [dist-workspace.toml](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/dist-workspace.toml) [src/context.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/context.rs) [src/gitcmd.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/gitcmd.rs) [src/cli.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/cli.rs) [README.md](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/README.md)
