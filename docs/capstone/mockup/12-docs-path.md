---
screen: 12-docs-path
serves_journeys: [J3, J4]
scenarios: [S7]
assumed: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry docs path <repo-a> <repo-b>` - the chain between two repos

## Layout

```
$ quarry docs path ingest-api report-builder
ingest-api -[sqs file-ingest]-> record-store -[http GET /records]-> report-builder
2 hops
```

Element tree: one line, edges inline → hop count.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| chain | shortest path over declared edges, direction producer → consumer | `11-docs-deps.md` |
| `--json` | array of `{from, kind, name, to}` | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| path | above | reachable |
| none | "no path from ingest-api to report-builder" | unreachable |
| several shortest | which is printed is `rule: logic (S7)` | ties |
| direction | whether edges are followed both ways is `rule: logic (S7)` | - |
