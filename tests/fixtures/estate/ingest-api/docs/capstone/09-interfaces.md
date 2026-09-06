---
generated_date: 2026-09-07
known_as: []
edges:
  produces:
    - { kind: sqs, name: file-ingest, site: src/publish.rs, schema: FileIngestMessage }
  consumes:
    - { kind: http, name: "GET /users/:id", site: src/auth.rs, schema: UserRef, from: identity.internal }
---

# Interfaces

## Produces

| Kind | Name | To | Site |
|---|---|---|---|
| sqs | file-ingest | - | `src/publish.rs` |

### file-ingest

Model: FileIngestMessage

## Consumes

| Kind | Name | From | Site |
|---|---|---|---|
| http | GET /users/:id | identity.internal | `src/auth.rs` |

### GET /users/:id

Model: UserRef
