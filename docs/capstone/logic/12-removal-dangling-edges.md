---
scenario: removal-dangling-edges
id: S10
surfaces_on: [mockup/14-remove.md]
depends_on: [S2, S6]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S10 - removal with dangling edges

When a repo is removed, other repos may still declare edges to it. Removal proceeds and reports them.

## Trigger & preconditions

- Trigger: S2's `remove` step 3, after the folder is deleted in the temp build and before the commit.
- Index current for the pre-removal state (S5).

## Steps

1. Query the edge table for every edge whose `from` or `to` is the removed repo, declared by another repo.
2. Print `<n> repos still declare edges to <repo>: <names, byte order>`; JSON `dangling: [{repo, kind, name, via}]`.
3. Continue with S2's commit and push. Exit 0.
4. After the next rebuild, those edges carry `missing: true` (S6) and appear in `deps` as `(not in quarry)` until the declaring repos' docs change.

## Branches

| Point | Rule |
| --- | --- |
| no dangling edges | no report line |
| dangling edges | report, proceed |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| the removed repo is re-added later | edges resolve again on the next rebuild; nothing to repair |

## State transitions

None of its own; S2 owns the folder transition.

## Invariants

- `remove` never edits another repo's folder.
- Every dangling edge remains visible in `deps` until its declaring page changes.

## Outcomes & side effects

- The report line; the commit is S2's.

## Dimensions not in play

- D1, D2 (S2's), D3, D4 (a count), D5, D6, D7, D8 (S1's), D9, D10, D11, D13 (the report is the only signal; nobody in the other repos is notified), D15 (git log): none.
- D12: nothing hidden.
