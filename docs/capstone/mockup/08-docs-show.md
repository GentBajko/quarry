---
screen: 08-docs-show
serves_journeys: [J1, J4]
scenarios: [S13, S5]
assumed:
  - overview = the first section of the repo's 00-index.md
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry docs show <repo>` - one repo's frontmatter and overview

## Layout

```
$ quarry docs show ingest-api
ingest-api @ 7be2d10 (generated 2026-09-04, capstone 5.2.1)
produces: sqs file-ingest -> record-store
consumes: http GET /customers/{id} <- identity-api

# ingest-api
<overview section of 00-index.md, verbatim>
```

Element tree: header (stamps) → edges → overview.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| header | repo name, imported commit, newest `generated_date`, `capstone_version` | - |
| edges | `produces`/`consumes` from frontmatter (`README.md` § Edge contract), one line each | `11-docs-deps.md` |
| overview | first section of the repo's `00-index.md` (assumed), never the whole file | `09-docs-section.md` |
| `--json` | `{repo, commit, generated_date, capstone_version, produces, consumes, overview}` | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| found | above | repo folder present |
| no edges | `produces: none` / `consumes: none` | no frontmatter keys anywhere in the repo |
| unknown repo | one line; exit code `rule: logic (S13)` | no such folder |
