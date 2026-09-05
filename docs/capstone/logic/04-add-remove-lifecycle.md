---
scenario: add-remove-lifecycle
id: S2
surfaces_on: [mockup/03-add.md, mockup/04-update.md, mockup/14-remove.md]
depends_on: [S1, S9, S3, S10, S13]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S2 - add / remove lifecycle

`add` registers a repo in the docs repo by creating its folder and importing; `remove` deletes the folder. Both are idempotent; `update` requires a prior `add`.

## Trigger & preconditions

- Trigger: `quarry add` or `quarry remove` in a source repo root; `quarry update` for the not-registered branch.
- `.quarry/` initialised (S12); name resolvable (S9).
- `add` requires `<docs_dir>/00-index.md` in the working tree at HEAD.
- Authority: docs-repo push credentials (S1).

## Steps

`add`:
1. Reset the clone to `origin/main` (S1 step 3).
2. `<docs repo>/<name>/` present → print `already in <docs repo> at <stamp sha>`, then continue exactly as `update` (S1 from step 4).
3. Absent → run S1 from step 5 with no stamp (imports unconditionally), commit message `add <name> @<sha>`.

`update`:
1. Folder absent → refuse: `not in <docs repo>; run quarry add` (exit 1). No auto-add.

`remove`:
1. Reset the clone to `origin/main`.
2. Folder absent → print `not in <docs repo>`, exit 0.
3. Present → delete the folder, report dangling edges (S10), regenerate the root index (S3), commit `remove <name>`, push with S1's retry loop, reindex (S5).
4. `.quarry/` in the source repo is left untouched.

## Branches

| Point | Rule |
| --- | --- |
| `add`, docs dir or `00-index.md` missing | refuse: `no 00-index.md in <docs_dir>; run Capstone map first` (exit 1) |
| `add`, HEAD not on default branch | S1 step 2 refusal applies |
| `add`, folder present with a different origin | S9 refusal |
| `remove`, folder present | delete + commit |
| `remove`, folder absent | no-op, exit 0 |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| `add` twice concurrently | second push rejected; redo finds the folder present at the same commit → `current`; exit 0 |
| `remove` and `update` concurrently | whichever pushes second redoes on the fresh tree: `update` after `remove` refuses (not registered); `remove` after `update` deletes the fresh import |
| `remove` push rejected three times | S1: `docs repo busy`, exit 2, folder still present remotely |
| `add` on an empty docs repo (no commits) | first commit creates the root index and the folder together |

## State transitions

Per-repo folder in the docs repo: `absent → present(stamp)` by `add`; `present → present(newer stamp)` by `update` (S1); `present → absent` by `remove`. `absent → present` again by a later `add` is allowed and starts a fresh stamp.

## Invariants

- `update` never creates a folder.
- A folder exists in the docs repo only if an `add` was run for that origin.
- `remove` never touches any other repo's folder; dangling edges are reported, not edited (S10).

## Outcomes & side effects

- `add`: one commit, root index row added, local index rebuilt, transcript per `mockup/03-add.md`.
- `remove`: one commit, root index row dropped, local index rebuilt, dangling-edge report; other repos' `deps` answers change on their next `sync`.
- Silent otherwise; git log is the record.

## Dimensions not in play

- D4 computation, D5 money, D6 limits, D7 time: none.
- D12 visibility: nothing hidden.
- D13 notification: silent.
