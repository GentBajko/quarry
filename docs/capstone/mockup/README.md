---
stage: mockup
surfaces: [cli]
generated_date: 2026-09-05
readback: a463e52ce97d
capstone_version: 5.2.1
mode: prescriptive
---
# quarry - product brief and mockup index

## Purpose

`quarry` is a CLI that copies each code repo's Capstone-generated
`docs/capstone/` into one shared git repo (the docs repo, "the
quarry"), indexes the copied pages' frontmatter into a local SQLite
file, and answers cross-repo questions: which repos consume what this
repo produces, what this repo relies on, the chain between two repos,
one section of one page. Callers are coding agents (Capstone's `groom`
and `plan`, and any agent session that can run a CLI) and engineers in
a terminal.

It exists because nothing cross-repo exists today at the company: 200+
repos, ~50 engineers, and no structured answer to "if I change X in
repo Y, what breaks in repo Z". Quarry is the pilot; Capstone rolls out
across the repos only if quarry proves itself for agentic use and for
people.

## Audience

One company's ~50 engineers across 200+ repos, and the agents they
run. First-wave repo count: not decided.

## Positioning

Open source, published for use beyond the company. Reads whatever
`docs/capstone/` holds today; every generator-side change (the
`09-interfaces.md` chapter, `non_interactive`/`extract` config keys,
the `groom`/`plan` integration, content hashes, `changelog/` as a
directory, feature numbering) is a separate project in Capstone. That
project's `groom`/`plan` integration is written into the skill
protocol text, never a harness hook, because Capstone also runs on
GitHub Copilot, which has no hooks.

## Success measures

A year after launch, at least one organisation other than the company uses
quarry. No other numeric target.

## Interaction surfaces

`surfaces: [cli]`. No visual surface of its own; the docs repo's
markdown may be opened in any viewer but nothing is designed for one.
`uiux` records itself skipped.

## Commercial model

Apache-2.0 (matching Capstone). No paid tier, no metering, no ledger,
no commercial scenario for `logic`. Voluntary donations via Patreon and
Buy Me a Coffee, linked from the repository README and the `quarry`
help footer (assumed placement).

## Constraints

- Runs in a git repo's root; the repo name is the last path segment of
  the `origin` URL, not configurable (§ Repo identity).
- Two writers per repo: an engineer running `quarry update` by hand,
  and the source repo's CI on merge to main. The docs repo has no CI.
- Every `docs *` result carries `generated_date`; agents receive
  sections, never whole files.
- `.quarry/` in the source repo holds `.config`, a `.gitignore` for the
  clone, and the docs repo clone; the clone is never committed to the
  source repo.

## Non-goals

- Quarry never generates docs and never runs Capstone (no `--map`).
- No `update --all`, no manifest sweep, no cron, no docs-repo CI.
- No Obsidian, Quartz, `.base` files, wikilinks, or Sync.
- The SQLite index is never committed.
- Merge resolution of doc files inside a source repo (chapters, changelog,
  `logic/`) is Capstone's, not quarry's.

## Repo identity

`acme/ingest-api.git` → `ingest-api`. Used for the folder
`<docs repo>/ingest-api/`, `docs list` rows, and every `<repo>`
argument. No `origin`, and two owners sharing a repo name, are S9.

## Edge contract

Edges are read from `produces`/`consumes` frontmatter on any page in a repo's folder. Field set, required fields, direction, one-sided declarations and dangling targets: `../logic/09-edge-contract.md`. Repos with no such keys produce "no edges" in `deps`/`path` and stay fully searchable. Who writes the keys is not quarry's concern; Capstone's `09-interfaces.md` is the intended writer.

## Rationale (confirmed)

- The vault copies and never generates: two people running `update` at
  once produce identical bytes, so concurrency reduces to a git push
  race (S1).
- A local clone is kept because `update` must push a commit and the
  query commands must work offline and on CI runners.
- `sync` and `docs index` are separate because one touches the network
  and the other does not; `--force` has no meaning for a pull.
- No `--all`: each repo pushes its own docs, so the docs repo needs no
  manifest and no scheduler.

## Open questions and deferrals

- First-wave repo count: not decided.
- Whether ad-hoc agent sessions, beyond Capstone's `groom`/`plan`, are a
  target caller: assumed yes.
- Everything under `rule: logic` in the screen files: see § Scenarios.
- Index schema, config file format, credential supply for CI runners,
  permalink host support: `architecture`.

## Screens

| Screen | Journeys | Scenarios |
| --- | --- | --- |
| [01-help.md](01-help.md) | J1, J4 | S13 |
| [02-init.md](02-init.md) | J1, J2 | S12, S9, S11 |
| [03-add.md](03-add.md) | J1 | S2, S1, S3 |
| [04-update.md](04-update.md) | J2 | S1, S3, S4, S11 |
| [05-ci-workflow.md](05-ci-workflow.md) | J2 | S12, S1 |
| [06-sync.md](06-sync.md) | J2, J4 | S11, S5 |
| [07-docs-list.md](07-docs-list.md) | J1, J4 | S5, S13 |
| [08-docs-show.md](08-docs-show.md) | J1, J4 | S13, S5 |
| [09-docs-section.md](09-docs-section.md) | J3, J4 | S8, S13 |
| [10-docs-search.md](10-docs-search.md) | J3, J4 | S8, S13 |
| [11-docs-deps.md](11-docs-deps.md) | J3 | S6, S7 |
| [12-docs-path.md](12-docs-path.md) | J3, J4 | S7 |
| [13-docs-index.md](13-docs-index.md) | J2, J4 | S5 |
| [14-remove.md](14-remove.md) | J5 | S2, S10, S3 |
| [15-quarry-layout.md](15-quarry-layout.md) | J1, J2, J4 | S3, S4, S6 |

## Journeys

| Journey | Path |
| --- | --- |
| J1 first run in a repo | 01 → 02 → 03 → 07 → 08 → 15 |
| J2 docs stay current | 05 (CI) or 04 (manual) → 15 → 06 on another machine → 13 |
| J3 agent checks before it guesses | 11 → 09 (or 10 → 09 when no edge) → 12 |
| J4 engineer browses | 01 → 07 → 08 → 10 → 09 → 12 |
| J5 leaving | 14 → 15 |

## Scenarios for `logic`

| Id | Behavior | Surfaces on | Must settle |
| --- | --- | --- | --- |
| S1 | update ordering and concurrency | 03, 04, 05, 14, 15 | idempotence on `(repo, commit)`; never-go-backwards; compare-and-push retry count and final failure; manual vs CI writer; run from a non-main branch or stale clone; interrupted run and temp dir; folder without stamp |
| S2 | add / remove lifecycle | 03, 04, 14 | `add` re-run; `update` before `add` (auto-add or refuse); `add` without `00-index.md`; `remove` when absent; exit codes |
| S3 | root index regeneration | 03, 04, 14, 15 | when and from what the docs repo's `00-index.md` is rebuilt; columns; ordering |
| S4 | permalink rewrite | 04, 15 | which pointers are rewritten, host URL derivation from `origin`, unsupported hosts, pointers to files absent at the stamped commit |
| S5 | index freshness | 06, 07, 08, 13 | rebuild on `built_at_commit` mismatch; whether `docs *` pulls or only `sync` does; corrupt index detection; `--force` |
| S6 | edge contract | 08, 11, 15 | accepted `kind` values, required fields, dangling targets, duplicate declarations, producer/consumer disagreement |
| S7 | graph traversal | 11, 12 | `--depth` default and cap; cycle printing and cut; `path` tie-break; direction of traversal |
| S8 | section and search matching | 09, 10 | heading match rule (exact, case, prefix); duplicates; no match; FTS ranking and result cap |
| S9 | repo identity | 02 | no `origin`; same-name repos under two owners; non-GitHub hosts; URL forms (ssh, https) |
| S10 | removal with dangling edges | 14 | refuse, warn, or silently remove when other repos still name this one |
| S11 | network and auth failure | 02, 04, 06 | retry, partial state, exit codes for clone, fetch, pull, push failures; diverged clone |
| S12 | init and configuration | 02, 05 | re-run on an initialised repo; precedence of flag, env var, `.config`; missing docs dir |
| S13 | output envelope and exit codes | 01, 07, 08, 09, 10, 11 | `--json` shape per command, exit codes for unknown command / unknown repo / no match, `generated_date` placement |

Every `rule: logic (Sn)` marker in the screen files is owned by row Sn.

## Assumed, and where each was settled

| Item | Screen | Settled by |
| --- | --- | --- |
| donation links in the help footer | 01 | this file § Commercial model (stands as assumed) |
| env var names `QUARRY_DOCS_REPO`, `QUARRY_DOCS_DIR` | 02, 05 | `../logic/02-init-and-config.md` |
| `.quarry/.config` is committable | 02, 05 | `../logic/02-init-and-config.md` |
| config file format | 02 | `../02-models.md` (JSON) |
| stamp file name `.quarry-stamp` | 03, 04, 15 | `../logic/01-update-ordering.md` |
| commit message shape | 04 | `../logic/01-update-ordering.md` |
| GitHub Actions as the example runner; Capstone `map check` as an optional prior gate | 05 | stands as an example |
| `list <repo>` prints that repo's `00-index.md` rows; repo table columns | 07 | stands (mockup owns layout) |
| `show` overview = first section of `00-index.md` | 08 | stands (mockup owns layout) |
| index file `.quarry/.docs-index.sqlite` | 13 | `../02-models.md` |
| root index columns | 15 | `../logic/05-root-index.md` |
