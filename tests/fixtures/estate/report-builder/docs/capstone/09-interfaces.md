---
generated_date: 2026-09-07
known_as: []
edges:
  consumes:
    - { kind: sqs, name: file-ingest, site: src/events.rs, schema: IngestEvent }
    - { kind: http, name: "GET /records", site: src/client.rs, schema: RecordView }
---

# Interfaces

## Produces

None found: this repo publishes nothing.

## Consumes

| Kind | Name | From | Site |
|---|---|---|---|
| sqs | file-ingest | - | `src/events.rs` |
| http | GET /records | - | `src/client.rs` |

### file-ingest

Model: IngestEvent

### GET /records

Model: RecordView
