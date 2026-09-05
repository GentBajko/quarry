---
screen: 02-init
serves_journeys: [J1, J2]
scenarios: [S12, S9, S11]
assumed:
  - env var names QUARRY_DOCS_REPO and QUARRY_DOCS_DIR
  - .quarry/.config is committable
  - config file format (key=value shown; format is architecture's)
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry init` - link this repo to a docs repo

Run once in a source repo's root. Creates `.quarry/`, clones the docs
repo into it, writes config. Journey J1's first step; J2's CI step
re-runs it headlessly with flags.

## Layout

```
$ quarry init
Docs repo URL: git@github.com:acme/docs-quarry.git
Docs folder in this repo [docs/capstone]:
cloning git@github.com:acme/docs-quarry.git -> .quarry/docs-quarry/ ... done
wrote .quarry/.config
wrote .quarry/.gitignore   (ignores .quarry/docs-quarry/)
this repo is `ingest-api` (from origin). Next: quarry add
```

```
$ quarry init --url git@github.com:acme/docs-quarry.git --docs-dir docs/capstone
cloning ... done
wrote .quarry/.config
wrote .quarry/.gitignore
```

Element tree, on disk after the run:

```
<source repo>/
  .quarry/
    .config          docs repo URL, docs folder, anything later stages add
    .gitignore       one line: the clone folder name
    docs-quarry/     clone of the docs repo (name = last path segment of the URL)
```

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| `Docs repo URL:` prompt | asks for the docs repo URL; skipped when `--url` or `QUARRY_DOCS_REPO` (assumed name) is set | clone |
| `Docs folder in this repo` prompt | the folder `update` copies from; default `docs/capstone`; skipped when `--docs-dir` or `QUARRY_DOCS_DIR` (assumed) is set | config |
| `--url`, `--docs-dir` | non-interactive equivalents of the two prompts, for CI (`05-ci-workflow.md`) | same |
| `--force` | relink to a different URL, replacing `.config` and the clone | `rule: logic (S12)` |
| `.quarry/.config` | holds the two values; committable (assumed), so a repo initialised once needs no flags in CI | read by every other command |
| `.quarry/.gitignore` | ignores the clone folder only; the clone is never committed to the source repo, commits land inside the clone by quarry (`04-update.md`) | - |
| clone `.quarry/<name>/` | the local docs repo; folder name = last segment of the URL minus `.git` | `03-add.md`, `06-sync.md`, every `docs *` command |
| closing line | prints the repo name derived from `origin` (`09-repo-identity` in README) and the next command | `03-add.md` |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| first run | transcript above | no `.quarry/` present |
| already initialised | one line naming the existing `.config` and its URL; whether it re-clones, overwrites, or refuses is `rule: logic (S12)` | `.quarry/.config` exists |
| precedence | which wins when flag, env var, and `.config` disagree is `rule: logic (S12)` | more than one source set |
| clone failed | git's error verbatim, nothing written; retry behaviour `rule: logic (S11)` | bad URL, no network, no auth |
| no `origin` | cannot derive the repo name; what is asked or refused is `rule: logic (S9)` | source repo has no `origin` remote |
| not a git repo | one line refusing: quarry runs in a git repo's root | cwd not a repo root |
