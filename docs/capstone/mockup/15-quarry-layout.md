---
screen: 15-quarry-layout
serves_journeys: [J1, J2, J4]
scenarios: [S3, S4, S6]
assumed:
  - stamp file name .quarry-stamp
  - root index columns
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# The docs repo as a reader sees it

Not a command: the layout of the docs repo (the quarry) that every
command writes or reads. A git repo anyone clones; nobody edits it by
hand.

## Layout

```
docs-quarry/
  00-index.md                 root index, regenerated on every write
  ingest-api/
    .quarry-stamp             imported source commit (assumed name)
    00-index.md               Capstone's index, copied verbatim
    01-architecture.md
    …
    08-glossary.md
    09-interfaces.md          present only where the source repo's generator wrote it
    logic/
    changelog.md
  identity-api/
    …
```

`00-index.md` at the root:

```
# docs-quarry

| Repo | Imported | Newest doc | Pages | Produces | Consumes |
|---|---|---|---|---|---|
| [ingest-api](ingest-api/00-index.md) | 7be2d10 | 2026-09-04 | 23 | 1 | 2 |
| [identity-api](identity-api/00-index.md) | 1a9e77c | 2026-09-01 | 19 | 3 | 0 |
```

Element tree: root index → one folder per repo → the repo's own
`docs/capstone/` tree plus a stamp.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| root index | one row per folder present, relative markdown links; columns assumed | each repo's `00-index.md` |
| repo folder | byte-for-byte copy of the source repo's docs dir at the stamped commit, except `path:line` pointers rewritten to permalinks (`04-update.md`) | `rule: logic (S4)` |
| `.quarry-stamp` | the imported commit; `update`'s idempotence key | `04-update.md` |
| cross-repo links | a page naming another repo writes a relative markdown link (`../identity-api/09-interfaces.md`); the frontmatter mirror is what the index reads | `README.md` § Edge contract, `rule: logic (S6)` |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| empty | root index with no rows | no repo added yet |
| populated | above | ≥1 folder |
| folder without stamp | treated how by `update` is `rule: logic (S1)` | hand-copied or interrupted import |
