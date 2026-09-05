---
screen: 09-docs-section
serves_journeys: [J3, J4]
scenarios: [S8, S13]
assumed: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry docs section <repo> "<heading>"` - one section, never a file

The agent-side read primitive (J3 step 2). Returns the text under one
heading, with the page's `generated_date`.

## Layout

```
$ quarry docs section record-store "file-ingest (v2)"
record-store/docs/capstone/09-interfaces.md § file-ingest (v2)   generated 2026-09-03

| Field | Type | Required | Notes |
|---|---|---|---|
| file_id | string (uuid) | yes | |
| content_type | enum | yes | accepted: application/json, application/xml (src/consumers/file_ingest.py:48) |
…
```

Element tree: locator line → section body verbatim.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| locator | file, heading, `generated_date` so the caller knows how far to trust it | - |
| body | the section's markdown to the next heading of the same or higher level; permalinks as rewritten by `04-update.md` | - |
| `--json` | `{repo, file, heading, generated_date, body}` | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| one match | above | heading unique in the repo |
| several matches | list of `file § heading` candidates, no body; whether the first is returned instead is `rule: logic (S8)` | same heading in more than one file |
| no match | one line; nearest headings offered or not is `rule: logic (S8)` | heading absent |
| matching rule | exact, case-insensitive, or prefix is `rule: logic (S8)` | - |
