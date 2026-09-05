---
scenario: output-and-exit-codes
id: S13
surfaces_on: [mockup/01-help.md, mockup/07-docs-list.md, mockup/08-docs-show.md, mockup/09-docs-section.md, mockup/10-docs-search.md, mockup/11-docs-deps.md]
depends_on: [S5]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S13 - output envelope and exit codes

One convention for every command, so an agent can call any of them without special cases.

## Trigger & preconditions

- Trigger: every `quarry` command's exit; `--json` accepted by all of them, including write commands.

## Steps

1. Exit codes:

   | Code | Meaning | Examples |
   | --- | --- | --- |
   | 0 | answered, or nothing to do | `current`, older-commit skip, 0 search hits, `no path`, `remove` of an absent repo |
   | 1 | refusal or unanswerable | unknown command, not initialised, not registered, HEAD not on default branch, diverged or unresolvable stamp, origin mismatch, unknown repo, ambiguous or missing section, bad flags |
   | 2 | external failure | network, auth, `docs repo busy` after three attempts, disk full during rebuild |

2. `--json`: exactly one JSON document on stdout, nothing on stderr. Envelope `{"ok": true, "built_at_commit": "<sha>", "synced_at": "<iso8601|null>", "result": <command-specific>}`. On failure `{"ok": false, "code": 1|2, "error": "<one line>", "result": <candidates or null>}`; `result` carries `candidates` for ambiguous/missing sections and `dangling` for `remove`.
3. Without `--json`: the human layout from the mockup screens on stdout; every error as one line on stderr; the S5 staleness line on stdout first when due.
4. Per-item fields in query results: every page or section carries `file` (repo-relative) and `generated_date`; every repo-level result carries `commit` (the stamp) and `newest_generated_date`.
5. Write-command JSON results: `update`/`add` → `{repo, from, to, files_changed, result: "imported"|"current"|"skipped"}`; `remove` → `{repo, removed: bool, dangling: [...]}`; `sync` → `{pulled_commits, repos_changed, index: "rebuilt"|"current"}`; `index` → `{repos, pages, edges, warnings: [...]}`.
6. Timestamps are UTC ISO-8601 with `Z`; shas are full 40 characters in JSON, 7 in human output.

## Branches

| Point | Rule |
| --- | --- |
| `--json` plus an interactive prompt would be needed | refuse, exit 1: prompts never happen under `--json` |
| `--help` | help text, exit 0, both modes |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| stdout closed mid-write | exit 2 |
| unknown flag | exit 1, usage line |

## State transitions

None.

## Invariants

- Under `--json`, stdout is always parseable as one JSON value, on every exit code.
- An agent can distinguish "no dependencies" (exit 0, empty array) from "no such repo" (exit 1) on every command.

## Outcomes & side effects

- None beyond the printed output.

## Dimensions not in play

- D1, D2, D3, D5, D6, D7, D8, D9, D10, D11, D13, D14, D15: none; this scenario shapes output only.
- D12: the envelope always exposes `built_at_commit` and `synced_at`; nothing hidden.
