---
screen: 07-docs-list
serves_journeys: [J1, J4]
scenarios: [S5, S13]
assumed:
  - list <repo> prints the topic and companion rows of that repo's 00-index.md
  - columns of the repo table (name, newest stamp, pages, edges)
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry docs list [<repo>]` - what is in the docs repo

First read-side screen a new user reaches (J1's closing line points at
`show`; `list` is what a browsing engineer types).

## Layout

```
$ quarry docs list
repo                 newest stamp   pages  produces  consumes
ingest-api          2026-09-04        23         1         2
identity-api    2026-09-01        19         3         0
record-store      2026-09-03        27         0         4
… 209 more
```

```
$ quarry docs list ingest-api
ingest-api @ 7be2d10 (2026-09-04)
  00-index.md
  01-architecture.md        generated 2026-09-04
  02-models.md              generated 2026-08-30
  …
  logic/01-file-ingest.md
  changelog.md
```

Element tree: table (one row per repo folder) / file list (one row per
file in the repo's `00-index.md`, assumed).

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| repo row | name (`README.md` § Repo identity), newest `generated_date` across its pages, page count, edge counts | `08-docs-show.md` |
| file row | path and its own `generated_date` | `09-docs-section.md` |
| `--json` | array of the same rows | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| populated | tables above | index has rows |
| empty docs repo | "no repos yet; run `quarry add` in a repo" | zero folders |
| unknown repo | one line, exit code `rule: logic (S13)` | `list <repo>` with no such folder |
| stale index | rebuilt silently before printing; whether a pull happens too is `rule: logic (S5)` | index stamp ≠ clone HEAD |
