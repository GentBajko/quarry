<p align="center">
  <img src="assets/logo.svg" alt="quarry" width="300">
</p>

<p align="center">
  <strong>Your repos already document themselves.</strong><br>
  quarry puts those docs in one place, indexes them,<br>
  and answers the question no single repo can.
</p>

<p align="center">
  <a href="#install"><img
    src="https://img.shields.io/badge/install-one%20binary%2C%20no%20runtime-7FA7E6?style=flat-square"
    alt="Installs as a single precompiled binary"></a>
  <img
    src="https://img.shields.io/badge/needs-git%202.30%2B-444C56?style=flat-square"
    alt="Requires git 2.30 or newer and nothing else">
  <a href="LICENSE"><img
    src="https://img.shields.io/badge/license-Apache--2.0-1F2328?style=flat-square"
    alt="Apache-2.0 licensed"></a>
</p>

<p align="center">
  <a href="https://www.patreon.com/cw/GentBajko"><img
    src="https://img.shields.io/badge/Patreon-support-F96854?style=for-the-badge&logo=patreon&logoColor=white"
    alt="Support quarry on Patreon"></a>
  <a href="https://buymeacoffee.com/gentbajko"><img
    src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-support-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=000000"
    alt="Buy Gent a coffee"></a>
</p>

<p align="center">
  Every repo generates its own architecture reference. quarry copies each
  one into a single git repository, indexes the frontmatter and the prose
  into SQLite, and serves cross-repo answers to agents and to people:
  who consumes what this repo produces, what breaks if this changes, one
  section of one page. It never generates documentation and never runs a
  model.
</p>

<p align="center">
  <img src="assets/flow.svg" width="720"
    alt="Four repos document themselves; init clones the shared docs repo; add and update import at one commit, subject to a stamp comparison; sync rebuilds a local index; eight docs commands answer from it.">
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#use-it">Use it</a> ·
  <a href="#in-ci">In CI</a> ·
  <a href="#cross-repo-edges">Edges</a> ·
  <a href="#commands">Commands</a> ·
  <a href="#rules-worth-knowing">Rules</a> ·
  <a href="#for-agents">For agents</a>
</p>

---

## Install

A single static binary. No runtime, no `cargo`, nothing to keep running.

```sh
curl -LsSf https://github.com/GentBajko/quarry/releases/latest/download/quarry-installer.sh | sh
```

Windows:

```powershell
irm https://github.com/GentBajko/quarry/releases/latest/download/quarry-installer.ps1 | iex
```

Rust users can take `cargo binstall quarry`; anyone else can grab the
archive for their platform off the releases page. Building it yourself is
`cargo build --release`.

The only requirement is `git` 2.30 or newer on `PATH` — quarry drives the
git binary so your credentials, proxies and SSH config are the ones it
already uses. SQLite is compiled in.

## Use it

Four commands run in a source repo, once each in the places you'd expect:

```sh
quarry init --url git@github.com:acme/docs-quarry.git   # once per repo
quarry add                                             # register and import
quarry update                                          # after every merge to main
quarry sync                                            # pull what other repos pushed
```

Eight answer questions, from the local index, offline, with `--json` on
every one:

```sh
quarry docs list                                    # every repo, with page and edge counts
quarry docs list record-store                       # one repo's files and their dates
quarry docs show record-store                       # stamps, both edge directions, overview
quarry docs section record-store "file-ingest (v2)" # one section, never a whole file
quarry docs search "file-ingest" --repo record-store
quarry docs deps ingest-api --downstream --depth 2  # who breaks if this changes
quarry docs deps report-builder --upstream          # what this repo relies on
quarry docs path ingest-api report-builder          # the chain between two repos
```

What that last group is for:

```text
$ quarry docs deps ingest-api --downstream --depth 0
ingest-api
  sqs file-ingest -> record-store
    http GET /records -> report-builder
2 repos, depth 2

$ quarry docs section record-store "file-ingest (v2)"
record-store/09-interfaces.md § file-ingest (v2)   2026-09-03

| Field        | Type          | Required | Notes                                     |
|--------------|---------------|----------|-------------------------------------------|
| file_id      | string (uuid) | yes      |                                           |
| content_type | enum          | yes      | accepted: application/json, application/xml |
```

Two calls, and the CSV parser nobody has written yet already has a
constraint attached to it: `content_type` is an enum without CSV, and it
lives in a repo you were not going to open.

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

Pin the installer to a tag rather than `latest`, so two hundred workflows
do not all move the day a release ships. `quarry init` is a no-op when
`.quarry/.config` is committed, which is the normal case.

## Cross-repo edges

Edges come from each repo's own `09-interfaces.md`, written as tables:

```markdown
## Produces

| Kind | Name | To | Site |
|---|---|---|---|
| sqs | file-ingest | [record-store](../record-store/09-interfaces.md) | `src/publish.rs:57` |

## Consumes

| Kind | Name | From |
|---|---|---|
| http | GET /users/{id} | [identity-api](../identity-api/09-interfaces.md) |
```

Columns are matched by name, not position, so a renamed column is a
warning rather than a silent zero. Backticks and link syntax are stripped
from cells, which means the relative link that gives you a graph in any
markdown viewer is the same cell the repo name comes from. Tables are read
only on that page and never inside a fenced block, so a document
describing this format declares nothing.

Frontmatter does the same job on any page, for generators that would
rather emit data than markdown, and wins when a page carries both:

```yaml
produces:
  - kind: sqs
    name: file-ingest
    to: record-store
    site: src/publish.rs:57
```

`kind` is free-form and lowercased; `name` and `to`/`from` are required.
Either side may declare an edge, and when both do quarry reports
`declared_by: both` — a disagreement between the two stays visible instead
of being merged away. An edge pointing at a repo that is not in the docs
repo is kept and marked `(not in quarry)`, so a broken link is something
you can see. Repos declaring no edges at all are still fully searchable.

## Commands

**In a source repo**

| Command | What it does |
| --- | --- |
| `quarry init [--url] [--docs-dir] [--force]` | Link this repo to a docs repo, write `.quarry/`, clone it shallowly. `--force` relinks to a different docs repo |
| `quarry add` | Register this repo in the docs repo and import its docs. Running it twice is a no-op |
| `quarry update [--force]` | Copy the docs at `HEAD` into the docs repo, commit, push. `--force` imports over a diverged or unreachable stamp |
| `quarry sync` | Pull the docs repo clone, then rebuild the index |
| `quarry remove` | Drop this repo's folder, reporting who still declares edges to it |

**Queries**

| Command | What it answers |
| --- | --- |
| `quarry docs list [<repo>]` | Every repo with its page and edge counts, or one repo's files |
| `quarry docs show <repo>` | Stamps, both edge directions, and the repo's overview section |
| `quarry docs section <repo> "<heading>"` | One section by heading. Exact match first, then prefix; an ambiguous heading lists the candidates and exits 1 |
| `quarry docs search "<term>" [--repo] [--limit]` | Full-text hits by repo, file and heading |
| `quarry docs deps <repo> --downstream\|--upstream [--depth N]` | The edge walk. `--depth 0` is unlimited; cycles are marked once and not expanded |
| `quarry docs path <a> <b>` | The shortest chain of edges, falling back to the reverse direction |
| `quarry docs index [--force]` | Rebuild the local index without touching the network |

Every command takes `--json` and prints exactly one JSON document,
including on failure. Add `--verbose` to see each git command on stderr.

## Rules worth knowing

**Only commits reachable from the default branch are imported.** The docs
repo stays a function of what actually shipped, and branch previews live
in the source repo where you already have them.

**The imported commit is stamped.** Re-importing the same commit does
nothing; an older commit is skipped; a diverged history is refused until
`--force`. On a shallow CI checkout quarry fetches the stamp commit rather
than guessing.

**Two writers never corrupt the docs repo.** A person and a CI job racing
on the same repo end with the newer commit either way: a rejected push
discards the local commit, resets to the remote and redoes the copy, up to
three times, because an import is a pure function of `(repo, commit)` and
two machines produce identical bytes.

**Queries never touch the network.** The index rebuilds whenever the clone
moves; `sync` is the only read-side command that pulls. A clone unsynced
for a day says so before it answers.

Exit codes: `0` answered or nothing to do, `1` refused, `2` something
external failed.

## For agents

Point your agent at `quarry docs deps` and `quarry docs section` before it
writes code that crosses a repo boundary. With
[Capstone](https://github.com/GentBajko/capstone), that already happens:
`groom` and `plan` consult quarry when a feature touches paths covered by
`09-interfaces.md`, so another repo's contract is cited in the plan before
the first task. Set `cross_repo: "off"` in `capstone.json` to stop it.

<p align="center">
  <img src="assets/capstone.svg" width="680"
    alt="Capstone generates each repo's docs with a model; quarry copies, indexes and answers, and the answer returns as a citation in the next plan.">
</p>

---

<details>
<summary>What lands where</summary>

```text
<source repo>/.quarry/
  .config              docs repo URL, docs folder, default branch
  .gitignore           ignores the clone and the index
  <docs-repo>/         shallow clone; the commits inside it are quarry's
  .docs-index.sqlite   derived, never committed, rebuilt when the clone moves
```

Commit `.quarry/.config` and `.quarry/.gitignore`; everything else is
local and disposable. `init` writes nothing outside `.quarry/`.

The docs repo itself is one folder per repo, plus a root `00-index.md`
regenerated on every write:

```text
docs-quarry/
  00-index.md
  ingest-api/
    .quarry-stamp        the imported commit and origin
    00-index.md, 01-architecture.md, …, 09-interfaces.md, logic/
  record-store/
  …
```

`file:line` pointers in the copied pages are rewritten into permalinks
pinned to the imported commit, so a link stays true after the code moves.
GitHub and GitLab shapes are built in; anything else uses the GitHub shape
unless `.quarry/.config` carries a `permalink_template` with `{owner}`,
`{repo}`, `{sha}`, `{path}` and `{line}`.

</details>

<details>
<summary>Configuration</summary>

`.quarry/.config`, written by `init`:

```json
{
  "default_branch": "main",
  "docs_dir": "docs/capstone",
  "permalink_template": null,
  "url": "git@github.com:acme/docs-quarry.git"
}
```

`QUARRY_DOCS_REPO` and `QUARRY_DOCS_DIR` stand in for `--url` and
`--docs-dir` when there is no config yet, which is what makes a fresh CI
runner work. Precedence is flag, then environment, then the file.

The repo's name in the docs repo is the last path segment of its `origin`
URL and is not configurable. Two repos from different owners claiming one
name is refused, on the strength of the origin recorded in the stamp.

</details>

<details>
<summary>Rough edges</summary>

Retrieval is SQLite FTS5 over the copied markdown. That is quick at a few
thousand pages and untested at fifty thousand; the rebuild budget is one
marked test, at ten seconds for five thousand pages.

The docs repo has no CI of its own by design, so nothing garbage-collects
a repo that stops pushing. Its folder simply keeps its last stamp, and
`quarry docs list` shows the date going stale.

Windows is built and tested but thin in the field. macOS and Linux are the
ones in daily use.

quarry reads whatever `docs/capstone/` holds. It does not care which tool
wrote it, but the edge commands need somebody to declare edges — with
Capstone that is `09-interfaces.md`, and with anything else it is the two
frontmatter keys above.

</details>

---

## License

[Apache-2.0](LICENSE). Free to use, fork and build on, commercially or
otherwise. A file you modify carries a notice saying you changed it
(§4(b)), and a derivative you distribute reproduces the attribution in
[NOTICE](NOTICE) (§4(d)).

Issues and pull requests welcome; by opening one you license your
contribution under the same terms (§5).
