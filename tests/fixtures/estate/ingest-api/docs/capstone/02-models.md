---
generated_date: 2026-09-07
---

# Models

## Entities

| Name | Definition site | Storage | Purpose |
|---|---|---|---|
| UserRef | src/models.rs | in memory | the UserRef payload |
| FileIngestMessage | src/models.rs | in memory | the FileIngestMessage payload |

## Fields and types

### UserRef

| Field | Type | Required |
|---|---|---|
| id | string | yes |
| tier | enum | yes |

### FileIngestMessage

| Field | Type | Required |
|---|---|---|
| file_id | string | yes |
| content_type | enum | yes |

## Relationships

None found: each payload stands alone.

## Boundaries

One representation per entity; no conversion sites.

## Validation

None found: the fixtures declare shape only.

## Schema

None found: nothing is persisted.
