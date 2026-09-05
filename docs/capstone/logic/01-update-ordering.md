---
scenario: update-ordering
id: S1
surfaces_on: [mockup/03-add.md, mockup/04-update.md, mockup/05-ci-workflow.md, mockup/14-remove.md, mockup/15-quarry-layout.md]
depends_on: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---

# S1 - update ordering and concurrency

`quarry update` (and the import half of `quarry add`) copies one source repo's docs folder into the docs repo. Two writers exist per repo: an engineer running it by hand, and the source repo's CI on merge to the default branch. The docs repo has no CI and no lock; ordering is enforced by the rules below.

## Trigger & preconditions

- Trigger: `quarry update` run in the source repo's root, by a person or by CI (`mockup/05-ci-workflow.md`).
- `.quarry/.config` exists or the equivalent flags/env vars are set (S12).
- `<docs repo>/<repo>/` exists (S2 settles `update` before `add`).
- The docs folder named in `.config` exists in the working tree.
- Authority: anyone whose git credentials can push to the docs repo. Quarry has no roles, grants, or allow-lists of its own.
- HEAD must be reachable from the source repo's default branch (step 2).

## Steps

1. Resolve the source commit: `HEAD` of the working tree. Resolve the repo name (S9).
2. Fetch `origin/<default>` of the source repo, `<default>` read from `.quarry/.config` (recorded from `origin/HEAD` at `init`; `main`, `master`, `trunk` are all valid). If HEAD is not an ancestor of `origin/<default>`, refuse: `commit <sha> not on origin/<default>; merge first`, exit non-zero, nothing written.
3. Fetch the docs repo clone's `origin/main` and reset the clone to it. The clone is never edited by hand and never has local commits worth keeping.
4. Read `<docs repo>/<repo>/.quarry-stamp` (the previously imported source commit).
   - Absent (folder hand-copied, or written by an older quarry): treat as no stamp; continue to step 6 unconditionally.
   - Equal to HEAD: print `current`, exit 0, write nothing.
5. Ancestry check between the stamp and HEAD, in the source repo:
   - If the stamp commit is not in local history (shallow CI checkout), run `git fetch origin <stamp>` first.
   - Still unresolvable → refuse: `stamp <sha> not reachable from <HEAD>; history rewritten?`, exit non-zero, nothing written.
   - Stamp is a descendant of HEAD → the docs repo already holds newer docs; print one line, skip, exit 0.
   - Stamp and HEAD diverged (neither is an ancestor of the other) → refuse with the same message as unresolvable. `quarry update --force` is the only way past: it imports HEAD and overwrites the stamp.
   - HEAD is a descendant of the stamp → forward; continue.
6. Build the import in a temp dir: copy the docs folder at HEAD, rewrite pointers to permalinks (S4), write `.quarry-stamp` = HEAD.
7. Swap the temp dir into `<docs repo>/<repo>/` (delete old folder, move new one in).
8. Regenerate the docs repo's root `00-index.md` (S3).
9. Commit inside the clone: message `update <repo> @<sha>` (`add <repo> @<sha>` when invoked by `add`), author = the runner's git identity. One commit per run.
10. Push to the docs repo's `main`. Rejected (remote moved) → discard the local commit, fetch, reset to `origin/main`, redo from step 4 on the fresh tree, push again. At most three attempts in total.
11. Rebuild the local index (S5). Print the summary line.

The copy is a pure function of `(repo, commit)`: two machines importing the same commit produce identical bytes, so a redo never conflicts on content, only on the push.

## Branches

| Point | Rule |
| --- | --- |
| HEAD not on default branch | refuse (step 2); no `--branch` escape hatch exists |
| no stamp | import unconditionally |
| same commit | `current`, exit 0 |
| older commit | skip, exit 0 |
| diverged or unresolvable stamp | refuse; `--force` overrides and rewrites the stamp |
| forward | import |
| push rejected | redo from step 4, three attempts |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| three rejected pushes | exit non-zero `docs repo busy, retry`; clone reset to `origin/main`; nothing half-committed |
| killed mid-run | the temp dir is left behind and discarded at the next run's start; the previous import and stamp stand |
| two engineers, same commit, same instant | one push lands; the other is rejected, redoes, finds `current`, exits 0 |
| CI and an engineer, different commits, same instant | both forward; the second pusher redoes on the fresh tree, where step 5 either skips (its commit is older) or imports (newer); the final state is the newer commit either way |
| fetch of source default branch, docs clone fetch, or push fails for network/auth reasons | S11 |
| docs folder missing at HEAD | refuse: nothing to import; points at Capstone `map` (`mockup/03-add.md`) |
| run twice in sequence | second run is `current` |

## State transitions

Entity: the per-repo stamp in the docs repo.

- `absent → <sha>`: first import, or import over a stampless folder.
- `<sha> → <descendant sha>`: forward import.
- `<sha> → <older sha>`: forbidden (skipped).
- `<sha> → <diverged sha>`: forbidden without `--force`.
- No terminal state; `remove` (S2) deletes the folder and the stamp together.

## Invariants

- After any path, `<docs repo>/<repo>/` is either the previous import, byte-for-byte intact, or exactly the docs folder at the stamped commit with permalinks rewritten. Never a mixture.
- The stamp only ever names a commit reachable from the source repo's default branch and descending from the previous stamp, except under `--force`.
- The docs repo's `main` only ever fast-forwards; quarry never rebases, amends, or force-pushes it.
- One commit per successful run; a run that writes nothing commits nothing.

## Outcomes & side effects

- Success: one commit on the docs repo's `main`; root index regenerated (S3); local index rebuilt (S5); summary printed (`mockup/04-update.md`).
- Changed `produces`/`consumes` frontmatter alters other repos' `deps` and `path` answers on their next `sync` (S6).
- Nobody is notified; the docs repo's git log is the only record. Commit message and stamp are the audit; history is never rewritten, so any import is reconstructible from the log.
- Failure endings print one line, exit non-zero, and leave the docs repo untouched.

## Dimensions not in play

- D4 computation: no arithmetic beyond counting changed files for the summary line.
- D5 money: nothing is charged or credited.
- D6 limits: only the three push attempts; no size or rate limits on imports.
- D7 time: no expiry, deadline, or clock comparison; ordering is by git ancestry, never by timestamp.
- D12 visibility: everything the run knows is printed; nothing is deliberately hidden.
- D13 notification: deliberately silent.
