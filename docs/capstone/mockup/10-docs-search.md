---
screen: 10-docs-search
serves_journeys: [J3, J4]
scenarios: [S8, S13]
assumed: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry docs search "<term>" [--repo <repo>]` - full-text hits

J3's fallback when no edge exists ("`file-ingest` finds the consumer by
name, one extra hop").

## Layout

```
$ quarry docs search "file-ingest"
record-store  09-interfaces.md   § file-ingest (v2)          2026-09-03
record-store  logic/01-file-ingest.md  § Trigger            2026-09-03
ingest-api      09-interfaces.md   § Produces                  2026-09-04
3 hits
```

Element tree: one row per hit (repo, file, heading, `generated_date`) → count.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| hit row | locates a section; each row is a valid `section` call | `09-docs-section.md` |
| `--repo` | restricts to one repo | - |
| `--json` | array of `{repo, file, heading, generated_date, snippet}` | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| hits | above | FTS match |
| none | "0 hits" | no match |
| ranking and cap | order of rows and any result limit are `rule: logic (S8)` | many hits |
