---
scenario: network-failure
id: S11
surfaces_on: [mockup/02-init.md, mockup/04-update.md, mockup/06-sync.md]
depends_on: [S1, S13]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S11 - network and auth failure

Every git operation quarry performs against a remote (clone, fetch, pull, push) can fail for reasons quarry does not control. The rules here decide what is retried, what is discarded, and what the user sees.

## Trigger & preconditions

- Trigger: a git remote operation inside `init`, `add`, `update`, `sync`, `remove` returns non-zero for a network, DNS, TLS, or authentication reason.
- Credentials are git's own: ssh agent, credential helper, `GIT_ASKPASS`. Quarry stores and prompts for none.

## Steps

1. Clone (`init`), fetch (`init`, `add`, `update`, `remove`), pull (`sync`): no quarry-level retry. Git's stderr is printed verbatim, one quarry line names the operation (`fetching origin/<default> failed`), exit 2. Nothing is written to the docs repo; `.quarry/.config` written earlier in the same `init` stays.
2. Push: a rejection because the remote moved is S1's redo loop (three attempts). Any other push failure exits 2 at once; the local commit is discarded and the clone reset to `origin/main`.
3. Diverged clone (local commits or working-tree edits inside `.quarry/<name>/`, from an interrupted push or hand edits): `sync`, `add`, `update`, `remove` reset hard to `origin/main` before doing anything and print one line: `discarded <n> local commits / local changes in <clone>`. The clone is disposable.

## Branches

| Point | Rule |
| --- | --- |
| remote moved on push | S1 retry |
| network/auth on push | exit 2, no retry |
| network/auth on clone/fetch/pull | exit 2, no retry |
| clone diverged | reset, continue |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| offline `docs *` query | unaffected; queries never touch the network (S5) |
| offline `sync` | exit 2; index untouched; next query still answers from the old clone with the staleness line (S5) |
| auth valid for read, not write | `update` reaches the push and exits 2 with git's message; import discarded |

## State transitions

None beyond the clone being reset to the remote state.

## Invariants

- No remote failure leaves a partial commit in the clone or a half-written folder (S1's temp-dir build).
- Quarry never caches, logs, or prints a credential.

## Outcomes & side effects

- Exit 2 with git's error on stderr (or the JSON error envelope, S13). Nothing else changes.

## Dimensions not in play

- D1 (git's authority, not quarry's), D3, D4, D5, D6 (no retry counts beyond S1's), D7 (no timeouts of quarry's own; git's apply), D9, D12, D13, D14, D15: none.
