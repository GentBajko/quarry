# quarry

One docs repo for many code repos, queryable by agents and people.

Each repo generates its own architecture reference (Capstone's `docs/capstone/`, or
anything shaped like it). `quarry` copies that folder into one shared git repository,
indexes it, and answers cross-repo questions: who consumes what this repo produces,
what breaks if this changes, one section of one page. It never generates docs and never
runs an LLM.

## Install

No runtime and no cargo needed; the release is a static binary.

```sh
curl -LsSf https://github.com/GentBajko/quarry/releases/latest/download/quarry-installer.sh | sh
```

Windows:

```powershell
irm https://github.com/GentBajko/quarry/releases/latest/download/quarry-installer.ps1 | iex
```

`git` 2.30+ must be on PATH. Nothing else is required.

## Use it

```sh
quarry init --url git@github.com:acme/docs-quarry.git   # once per repo
quarry add                                              # register and import
quarry update                                           # after every merge to main
quarry sync                                             # pull someone else's imports
```

Query it, as a person or an agent (`--json` on every command):

```sh
quarry docs list                                     # every repo, with page and edge counts
quarry docs list ingest-api                         # one repo's files
quarry docs show record-store                     # stamps, edges, overview
quarry docs section record-store "file-ingest (v2)"
quarry docs search "file-ingest" --repo record-store
quarry docs deps ingest-api --downstream --depth 2  # who breaks if this changes
quarry docs path ingest-api report-builder
```

## In CI

```yaml
on:
  push:
    branches: [main]
jobs:
  quarry:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: curl -LsSf https://github.com/GentBajko/quarry/releases/download/v0.1.0/quarry-installer.sh | sh
      - run: quarry init --url ${{ vars.QUARRY_DOCS_REPO }}
      - run: quarry update
```

Pin the installer to a release rather than `latest` so 200 workflows do not all move at
once. `quarry init` is a no-op when `.quarry/.config` is committed.

## Cross-repo edges

Edges come from two frontmatter keys on any page in a repo's folder:

```yaml
produces:
  - kind: sqs
    name: file-ingest
    to: record-store
    site: src/publish/sqs.py:57
consumes:
  - kind: http
    name: GET /customers/{id}
    from: identity-api
```

`kind` is free-form and lowercased; `name` and `to`/`from` are required. Either side may
declare an edge; when both do, quarry reports `declared_by: both`. A target that is not
in the docs repo is kept and marked `(not in quarry)` rather than dropped, so a broken
link stays visible. Repos without these keys are still fully searchable.

## What lands where

```
<source repo>/.quarry/
  .config              docs repo URL, docs folder, default branch
  .gitignore           ignores the clone and the index
  <docs-repo>/         shallow clone; commits inside it are quarry's
  .docs-index.sqlite   derived, never committed, rebuilt when the clone moves
```

Commit `.quarry/.config` and `.quarry/.gitignore`; everything else is local and
disposable.

## Rules worth knowing

Only commits reachable from the source repo's default branch are imported, and the
imported commit is recorded in `<docs repo>/<repo>/.quarry-stamp`. Re-importing the same
commit does nothing; an older commit is skipped; a diverged history is refused until
`--force`. Two writers (a person and CI) never corrupt the docs repo: quarry rebuilds on
top of the remote and pushes again, up to three times.

Exit codes: `0` answered or nothing to do, `1` refused, `2` something external failed.

## Licence

Apache-2.0. If it saves you time: [Patreon](https://patreon.com/quarry) ·
[Buy Me a Coffee](https://buymeacoffee.com/quarry).
