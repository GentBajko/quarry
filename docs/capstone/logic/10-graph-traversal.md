---
scenario: graph-traversal
id: S7
surfaces_on: [mockup/11-docs-deps.md, mockup/12-docs-path.md]
depends_on: [S6, S13]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S7 - graph traversal

`deps` walks the edge table from one repo; `path` finds the chain between two.

## Trigger & preconditions

- Trigger: `quarry docs deps <repo> --downstream|--upstream [--depth N]`, `quarry docs path <a> <b>`.
- Index current (S5). `<repo>`, `<a>`, `<b>` must be folders in the docs repo; otherwise exit 1 `unknown repo <name>` (S13).
- Exactly one of `--downstream` / `--upstream` is required for `deps`; neither or both → exit 1 usage.

## Steps

`deps`:
1. `--downstream` follows edges producer → consumer from `<repo>`; `--upstream` follows consumer → producer.
2. `--depth N` default 1; `0` means unlimited.
3. Breadth-first. Each repo is emitted once, at its shallowest depth, with the edge that reached it (`kind`, `name`, `via`, `declared_by`, `missing`). Repos reached by several edges at the same depth are emitted once per edge.
4. An edge to an already-emitted repo (or back to `<repo>`) is emitted once with `cycle: true` / `(cycle)` and not expanded.
5. Output ends with `<n> repos, depth <d>` (human) or the array (JSON).

`path`:
1. `a == b` → `same repo`, exit 0, `path: []`.
2. BFS producer → consumer from `a` to `b`. Neighbours expanded in byte order of repo name, so the first path found is deterministic; one path returned.
3. None → BFS from `b` to `a`; found → returned with `direction: reverse` and the human line `reverse path (<b> produces for <a>)`.
4. Neither → `no path from <a> to <b>`, exit 0, `path: null`.

## Branches

| Point | Rule |
| --- | --- |
| no edges from `<repo>` | `no edges declared for <repo>; try quarry docs search`, exit 0, empty array |
| depth exhausted with more beyond | last line says `more beyond depth <N>` (human) / `truncated: true` (JSON) |
| missing repo on an edge | listed, `missing: true`, never expanded |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| cyclic graph with `--depth 0` | terminates: visited set |
| `--depth` negative or non-integer | exit 1 usage |

## State transitions

None.

## Invariants

- Output for the same index and arguments is identical across runs and machines.
- A repo never appears twice as an expanded node.
- `path` never crosses a `missing: true` repo.

## Outcomes & side effects

- Read-only. Each JSON edge carries the declaring page and its `generated_date` so the caller can weigh trust.

## Dimensions not in play

- D1, D2 (index present is the only precondition), D5, D7, D8 (read-only), D9, D10 (local), D11, D13, D14, D15: none.
- D4: depth counting only.
- D6: `--depth` default 1 is a default, not a cap; `0` lifts it.
- D12: nothing hidden; one-sided and missing edges are shown, not filtered.
