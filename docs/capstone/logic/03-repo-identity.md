---
scenario: repo-identity
id: S9
surfaces_on: [mockup/02-init.md]
depends_on: []
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S9 - repo identity

Every command that names a repo derives the name from the source repo's `origin` remote. Nothing is configurable.

## Trigger & preconditions

- Trigger: any of `init`, `add`, `update`, `remove` in a source repo root.
- `git remote get-url origin` succeeds; otherwise refuse: `no origin remote; quarry derives the repo name from it` (exit 1).

## Steps

1. Read the `origin` URL. Accepted forms: `git@host:owner/repo`, `ssh://git@host/owner/repo`, `https://host/owner/repo`, each with or without `.git`.
2. Name = last path segment, `.git` stripped, case preserved.
3. Normalized origin = `host/owner/repo`, lowercased, scheme, user, port and `.git` stripped. Stored in `.quarry-stamp` beside the commit by `add`/`update` (S1 step 6).
4. On `add`/`update`, when `<docs repo>/<name>/.quarry-stamp` exists and its normalized origin differs from this repo's → refuse: `name <name> already used by <stored origin>` (exit 1). `--force` does not override this.

## Branches

| Point | Rule |
| --- | --- |
| no `origin` | refuse |
| URL not parseable into `owner/repo` (fewer than two path segments) | refuse: `cannot derive owner/repo from <url>` |
| stamp origin matches | continue |
| stamp origin differs | refuse |
| stamp absent or from an older quarry without an origin field | continue; the next import writes it |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| two repos differ only by case (`Foo`, `foo`) | two distinct names; on a case-insensitive filesystem the second `add` fails at the filesystem level with git's error, exit 2 |
| origin renamed (repo moved to another owner) | stamp origin differs → refuse; the human path is `remove` from a clone with the old origin, then `add` |

## State transitions

None; the name is derived per run and never stored in the source repo.

## Invariants

- One folder in the docs repo maps to exactly one normalized origin, once stamped.
- The name never depends on cwd, branch, or config.

## Outcomes & side effects

- The name is printed by `init` and used as the folder and the `<repo>` argument everywhere (`mockup/README.md` § Repo identity).

## Dimensions not in play

- D1 authority, D4, D5, D6, D7, D8 concurrency, D10, D11, D13, D14, D15: none; this is a pure derivation with one persisted field (the stamp origin, owned by S1's write).
- D12 visibility: nothing hidden.
