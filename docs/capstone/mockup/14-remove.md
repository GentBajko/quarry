---
screen: 14-remove
serves_journeys: [J5]
scenarios: [S2, S10, S3]
assumed: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry remove` - drop this repo from the docs repo

Run in the source repo's root. Inverse of `03-add.md`.

## Layout

```
$ quarry remove
removing docs-quarry/ingest-api/ (23 files)
2 repos still declare edges to ingest-api: record-store, report-builder
regenerating docs-quarry/00-index.md
commit "remove ingest-api"; push origin main ... ok
reindexed
```

Element tree: delete folder → report dangling edges → regenerate root
index → commit → push → reindex.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| folder removal | deletes `<docs repo>/<repo>/`; `.quarry/` in the source repo is left in place | - |
| dangling-edge report | lists repos whose `consumes`/`produces` still name this one | `11-docs-deps.md` |
| root index regeneration, commit, push, reindex | as in `04-update.md` | `rule: logic (S3)` |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| removed | above | folder present |
| not present | one line, exit 0 or not is `rule: logic (S2)` | folder absent |
| dangling edges | listed; whether removal is refused, warned, or silent is `rule: logic (S10)` | other repos reference this one |
| push rejected | as `04-update.md`; `rule: logic (S1)` | concurrent writer |
