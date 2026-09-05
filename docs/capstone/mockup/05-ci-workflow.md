---
screen: 05-ci-workflow
serves_journeys: [J2]
scenarios: [S12, S1]
assumed:
  - GitHub Actions as the example runner
  - the Capstone map check gate shown as an optional prior step owned by the separate Capstone project
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# Merge-to-main workflow in a source repo

Not a command: the transcript a source repo's CI runs so the docs repo
is updated without anyone remembering. The docs repo has no CI of its
own (README § Non-goals).

## Layout

```yaml
# .github/workflows/quarry.yml   (GitHub Actions shown; any runner works)
on:
  push:
    branches: [main]
jobs:
  quarry:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: curl -LsSf https://github.com/<owner>/quarry/releases/latest/download/quarry-installer.sh | sh
      - run: quarry init --url ${{ vars.QUARRY_DOCS_REPO }}     # no-op if .quarry/.config is committed
      - run: quarry update
```

Optional prior gate, owned by the separate Capstone project: a
`pull_request` job running Capstone's `map check` as a required status,
so what `update` copies is never stale.

Element tree: checkout → install the precompiled binary → init (flags or env) → update.

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| installer line | downloads the release binary for the runner's platform; no cargo, no runtime | `07-operations.md` |
| `quarry init --url …` | headless init (`02-init.md`); `QUARRY_DOCS_REPO` env var (assumed name) works the same | `.quarry/` on the runner |
| `quarry update` | `04-update.md` with CI as the writer | docs repo commit |
| credentials | the runner needs push access to the docs repo; how it is supplied is `architecture`'s | - |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| green | update's forward or current transcript in the job log | ordinary merge |
| race with a manual run | update's push-rejected retry; outcome `rule: logic (S1)` | an engineer ran `update` at the same time |
| init on an initialised repo | init's "already initialised" state; `rule: logic (S12)` | `.quarry/.config` committed |
