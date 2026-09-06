---
generated_date: 2026-09-07
known_as: []
edges:
  produces:
    - { kind: http, name: "GET /records", site: src/routes.rs, schema: Record }
  consumes:
    - { kind: sqs, name: file-ingest, site: src/consumer.rs, schema: IngestedFile }
---

# Interfaces

## Produces

| Kind | Name | To | Site |
|---|---|---|---|
| http | GET /records | - | `src/routes.rs` |

### GET /records

Model: Record

## Consumes

| Kind | Name | From | Site |
|---|---|---|---|
| sqs | file-ingest | - | `src/consumer.rs` |

### file-ingest

Model: IngestedFile
