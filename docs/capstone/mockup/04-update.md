---
screen: 04-update
serves_journeys: [J2]
scenarios: [S1, S3, S4, S11]
assumed:
  - stamp file name .quarry-stamp
  - commit message shape
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry update` - copy this repo's docs into the docs repo

Run in the source repo's root by an engineer, or by CI on merge to main
(`05-ci-workflow.md`). Never runs Capstone; copies what `docs/capstone/`
holds at the current commit.

## Layout

```
$ quarry update
ingest-api @ 7be2d10 (main)
docs-quarry: fetching origin/main ... ok
stamp 4f1c9a2 -> 7be2d10: 3 files changed
rewriting file:line pointers to permalinks
regenerating docs-quarry/00-index.md
commit "update ingest-api @7be2d10"; push origin main ... ok
reindexed
```

```
$ quarry update
ingest-api @ 7be2d10: current, nothing to do
```

Element tree of one run: read stamp → fetch docs clone → build new
folder in a temp dir → swap into `<docs repo>/<repo>/` → regenerate
root index → commit → push → reindex.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| header line | repo name, current commit, branch | - |
| stamp comparison | reads `<docs repo>/<repo>/.quarry-stamp` (assumed name), decides current / forward / backward | states below |
| permalink rewrite | `path:line` pointers in copied files become links at the imported commit on the source repo's host, derived from `origin` | `rule: logic (S4)` for the rewrite rule and unknown hosts |
| root index regeneration | `<docs repo>/00-index.md` rebuilt from the folders present (`15-quarry-layout.md`) | `rule: logic (S3)` |
| commit + push | one commit per run inside the clone; pushes to the docs repo's `main` | states below |
| reindex | rebuilds the local SQLite index (`13-docs-index.md`) | - |
| `--force` | imports HEAD over a diverged or unresolvable stamp and rewrites the stamp | `rule: logic (S1)` |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| forward | full transcript | incoming commit descends from the stamp |
| current | one line, exit 0, nothing written | same commit as the stamp |
| backward | one line: docs repo already has a newer commit, skipped | stamp descends from the incoming commit; the rule and its exit code are `rule: logic (S1)` |
| diverged | refused with the stamp sha; `--force` is the way past | stamp neither ancestor nor descendant; `rule: logic (S1)` |
| run from a branch | what is imported, refused, or warned when the current commit is not on `main` is `rule: logic (S1)` | branch other than main |
| push rejected | "remote moved, redoing copy (2/3)"; retry count, and what happens after the last try, `rule: logic (S1)` | concurrent writer (manual vs CI, or two engineers) |
| fetch failed | git's error; nothing written; `rule: logic (S11)` | no network / no auth |
| not registered | points at `03-add.md`; whether update auto-adds is `rule: logic (S2)` | `<docs repo>/<repo>/` absent |
| interrupted | previous import intact (temp dir + swap); the partial temp dir's fate is `rule: logic (S1)` | killed mid-run |
| `--json` | one document: repo, from, to, files_changed, result | any of the above |
