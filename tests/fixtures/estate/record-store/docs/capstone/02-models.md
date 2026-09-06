---
generated_date: 2026-09-07
---

# Models

## Entities

| Name | Definition site | Storage | Purpose |
|---|---|---|---|
| IngestedFile | src/models.rs | in memory | the IngestedFile payload |
| Record | src/models.rs | in memory | the Record payload |

## Fields and types

### IngestedFile

| Field | Type | Required |
|---|---|---|
| file_id | string | yes |
| content_type | enum | yes |

### Record

| Field | Type | Required |
|---|---|---|
| id | string | yes |
| created_at | string | yes |
| content_type | enum | yes |

## Relationships

None found: each payload stands alone.

## Boundaries

One representation per entity; no conversion sites.

## Validation

None found: the fixtures declare shape only.

## Schema

None found: nothing is persisted.
