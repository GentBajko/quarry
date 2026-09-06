---
generated_date: 2026-09-07
---

# Models

## Entities

| Name | Definition site | Storage | Purpose |
|---|---|---|---|
| IngestEvent | src/models.rs | in memory | the IngestEvent payload |
| RecordView | src/models.rs | in memory | the RecordView payload |

## Fields and types

### IngestEvent

| Field | Type | Required |
|---|---|---|
| file_id | string | yes |

### RecordView

| Field | Type | Required |
|---|---|---|
| id | string | yes |
| created_at | string | yes |

## Relationships

None found: each payload stands alone.

## Boundaries

One representation per entity; no conversion sites.

## Validation

None found: the fixtures declare shape only.

## Schema

None found: nothing is persisted.
