---
screen: 06-sync
serves_journeys: [J2, J4]
scenarios: [S11, S5]
assumed: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry sync` - pull the docs clone and rebuild the index

Read-side counterpart of `04-update.md`. Touches the network;
`13-docs-index.md` does not.

## Layout

```
$ quarry sync
docs-quarry: pulling origin/main ... 4 commits, 2 repos changed
index: rebuilt (212 repos, 3,904 pages, 1,120 edges) in 1.8s
```

```
$ quarry sync
docs-quarry: up to date
index: current
```

Element tree: pull → compare index stamp → reindex if HEAD moved.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| pull | fast-forwards the clone's `main`; the clone is never edited by hand, so nothing conflicts | reindex |
| reindex | `13-docs-index.md` without `--force` | every `docs *` command |
| summary line | commits pulled, repos changed, index counts | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| moved | first transcript | remote ahead |
| up to date | second transcript | remote equal |
| pull failed | git's error; index untouched; `rule: logic (S11)` | no network / no auth |
| not initialised | points at `02-init.md` | no `.quarry/` |
| diverged clone | someone edited the clone by hand; what sync does is `rule: logic (S11)` | local commits not on remote |
