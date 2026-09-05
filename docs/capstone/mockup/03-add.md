---
screen: 03-add
serves_journeys: [J1]
scenarios: [S2, S1, S3]
assumed:
  - stamp file name .quarry-stamp inside the docs repo folder
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry add` - register this repo and import its docs

Second step of J1. Creates `<docs repo>/<repo>/` if absent, then does
the same copy-commit-push as `04-update.md`.

## Layout

```
$ quarry add
ingest-api not in docs-quarry yet
importing docs/capstone (23 files) at 4f1c9a2 -> docs-quarry/ingest-api/
rewriting file:line pointers to permalinks
regenerating docs-quarry/00-index.md
commit "add ingest-api @4f1c9a2"; push origin main ... ok
reindexed. `quarry docs show ingest-api`
```

Element tree, in the docs repo after the run:

```
docs-quarry/
  00-index.md            root index: one row per repo (15-quarry-layout.md)
  ingest-api/
    .quarry-stamp        imported commit (assumed name)
    00-index.md          copied as-is from the source repo
    01-architecture.md … 08-glossary.md, logic/, changelog.md
```

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| presence check | looks for `<docs repo>/<repo>/` | import, or the "already present" state |
| import | copy, permalink rewrite, root index regeneration, commit, push, reindex; identical to `04-update.md` | `04-update.md` for the steps |
| closing line | next command to try | `08-docs-show.md` |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| first add | transcript above | folder absent |
| already present | one line: "already in docs-quarry at <commit>"; whether it then behaves as `update` or stops is `rule: logic (S2)` | folder present |
| no docs folder | one line: `docs/capstone` (or configured dir) missing, nothing to import; points at Capstone `map` | docs dir absent in the source repo |
| no `00-index.md` | the docs dir exists without an index; whether add refuses or imports anyway is `rule: logic (S2)` | index absent |
| not initialised | points at `02-init.md` | no `.quarry/.config`, no env vars |
| push rejected | see `04-update.md` states; `rule: logic (S1)` | remote moved |
