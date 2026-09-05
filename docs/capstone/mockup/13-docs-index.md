---
screen: 13-docs-index
serves_journeys: [J2, J4]
scenarios: [S5]
assumed:
  - index file name .docs-index.sqlite beside the clone
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry docs index [--force]` - rebuild the local index

Never needs the network; the command CI and a `post-merge` hook call.
Every `docs *` command runs it implicitly when the index stamp lags the
clone.

## Layout

```
$ quarry docs index
index current (built at 9c04e1f)
```

```
$ quarry docs index --force
rebuilding .quarry/.docs-index.sqlite from docs-quarry @ 9c04e1f
212 repos, 3,904 pages, 1,120 edges in 1.9s
```

Element tree: stamp check → rebuild → counts.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| stamp check | compares `built_at_commit` in the index with the clone's HEAD | rebuild or "current" |
| `--force` | rebuild regardless of the stamp; the way out of a corrupt or schema-changed index after a quarry upgrade | - |
| counts | repos, pages, edges | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| current | one line | stamps equal |
| rebuilt | counts | stamps differ, or `--force` |
| corrupt index | detection and recovery without `--force` is `rule: logic (S5)` | unreadable file |
| index location and schema | `.quarry/.docs-index.sqlite` (assumed); tables are `architecture`'s | - |
