---
screen: 11-docs-deps
serves_journeys: [J3]
scenarios: [S6, S7]
assumed: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry docs deps <repo> --downstream|--upstream [--depth N]`

J3's first call: who breaks if this repo changes (`--downstream`), and
what this repo relies on (`--upstream`). Edges come from `produces` /
`consumes` frontmatter (`README.md` § Edge contract).

## Layout

```
$ quarry docs deps ingest-api --downstream
ingest-api
└─ sqs file-ingest -> record-store        (record-store/09-interfaces.md)
   └─ http GET /records -> report-builder          (report-builder/09-interfaces.md)
2 repos, depth 2
```

```
$ quarry docs deps ingest-api --downstream --json
[{"repo":"record-store","kind":"sqs","name":"file-ingest","depth":1,
  "via":"record-store/docs/capstone/09-interfaces.md"},
 {"repo":"report-builder","kind":"http","name":"GET /records","depth":2,
  "via":"report-builder/docs/capstone/09-interfaces.md"}]
```

Element tree: root → edge lines (kind, name, target, via file) → count.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| edge line | kind, name, the other repo, the page declaring the edge | `09-docs-section.md` on the via file |
| `--depth N` | how far to walk; default `rule: logic (S7)` | - |
| `--upstream` | same layout, arrows reversed | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| edges | above | frontmatter edges exist |
| no edges | "no edges declared for ingest-api; try `quarry docs search`" | no `produces`/`consumes` keys anywhere name this repo |
| dangling edge | an edge names a repo not in the docs repo; shown with a marker; `rule: logic (S6)` | target folder absent |
| cycle | a repo reachable from itself; how it is printed and cut is `rule: logic (S7)` | cyclic graph |
| unknown repo | one line; exit code `rule: logic (S13)` | - |
