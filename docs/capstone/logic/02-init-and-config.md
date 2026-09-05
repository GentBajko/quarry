---
scenario: init-and-config
id: S12
surfaces_on: [mockup/02-init.md, mockup/05-ci-workflow.md]
depends_on: [S9, S11]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S12 - init and configuration

`quarry init` links a source repo to a docs repo: writes `.quarry/.config`, `.quarry/.gitignore`, and clones the docs repo into `.quarry/<name>/`. Idempotent; CI re-runs it on every job.

## Trigger & preconditions

- Trigger: `quarry init [--url U] [--docs-dir D] [--force]` in a source repo's root, by a person or CI.
- cwd is the root of a git repository with an `origin` remote (S9); otherwise refuse (exit 1).
- Authority: git credentials that can clone the docs repo. Nothing else.

## Steps

1. Resolve the docs repo URL and docs dir. Precedence: flag > env var (`QUARRY_DOCS_REPO`, `QUARRY_DOCS_DIR`) > existing `.quarry/.config`. Nothing set and stdin is a TTY → prompt (`mockup/02-init.md`). Nothing set and no TTY → refuse, naming the two flags (exit 1).
2. Resolve `default_branch` from `origin/HEAD` (`git symbolic-ref refs/remotes/origin/HEAD`). Unset → `main`, with a warning line.
3. If `.quarry/.config` exists and its `url` differs from the resolved one and `--force` is absent → refuse: `already linked to <stored url>; use --force to relink` (exit 1). With `--force`: delete the clone folder, continue.
4. Write `.quarry/.config` with `url`, `docs_dir`, `default_branch`. Optional `permalink_template` (S4) is preserved when present.
5. Write `.quarry/.gitignore` containing exactly the clone folder name (last URL path segment minus `.git`). Rewritten every run.
6. Clone folder absent → `git clone <url> .quarry/<name>/`. Present → `git fetch origin` and reset to `origin/main` (S11 diverged-clone rule).
7. Docs dir absent in the working tree → warning `no <docs_dir> yet; run Capstone map before quarry add`; config is still written.
8. Print the repo name (S9) and the next command.

## Branches

| Point | Rule |
| --- | --- |
| config present, same URL | keep; ensure clone; exit 0 |
| config present, different URL | refuse unless `--force` |
| config absent, values from flags/env | write, clone |
| config absent, interactive | prompt |
| config absent, no TTY | refuse |
| docs repo has no commits yet | clone succeeds; first `add` creates the root index (S3) |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| clone fails (URL, network, auth) | S11: git's error, exit 2, `.config` already written stays |
| not a git repo / no `origin` | exit 1, nothing written |
| source repo's own `.gitignore` ignores `.quarry/` wholesale | allowed; `.config` is then not committed and CI must pass flags or env |
| `--docs-dir` outside the repo (absolute or `..`) | refuse, exit 1 |

## State transitions

`.quarry/`: `absent → linked(url)`; `linked(url) → linked(url')` only via `--force`. No terminal state; `remove` (S2) does not touch it.

## Invariants

- `init` writes nothing outside `.quarry/`.
- After a successful run, `.quarry/.config` names exactly one docs repo and the clone folder matches it.
- The clone folder is ignored by the source repo's git at all times after the first run.

## Outcomes & side effects

- Success: config, ignore file, clone on disk; transcript printed; exit 0.
- Nobody notified; nothing recorded beyond the files themselves.

## Dimensions not in play

- D4 computation, D5 money, D6 limits, D7 time: none.
- D12 visibility: the transcript shows every path written.
- D13 notification: silent.
- D14 effects on others: none; the docs repo is only read.
- D15 record: the files are the record; no log.
