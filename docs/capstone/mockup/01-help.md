---
screen: 01-help
serves_journeys: [J1, J4]
scenarios: [S13]
assumed:
  - donation links in the help footer
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# `quarry` and `quarry docs` - help

Landing screen for both journeys: bare `quarry` and bare `quarry docs`
print help and exit 0. No command runs implicitly.

## Layout

```
$ quarry
quarry - one docs repo for many code repos, queryable by agents and people

Usage: quarry <command> [options]

  init      link this repo to a docs repo (.quarry/)
  add       register this repo in the docs repo and import its docs
  update    copy this repo's docs into the docs repo, commit, push
  sync      pull the docs repo clone and rebuild the index
  remove    drop this repo from the docs repo
  docs      query the docs repo (run `quarry docs` for its commands)

Options: --json   machine-readable output on every command
         --help
Support quarry: <patreon-url> | <buy-me-a-coffee-url>
```

```
$ quarry docs
Usage: quarry docs <command> [options]

  list [<repo>]                         repos in the docs repo, or one repo's files
  show <repo>                           frontmatter and overview of one repo
  section <repo> "<heading>"            one section by heading, never a whole file
  search "<term>" [--repo <repo>]       full-text search: repo, file, heading hits
  deps <repo> --downstream|--upstream [--depth N]
  path <repo-a> <repo-b>                shortest chain of edges between two repos
  index [--force]                       rebuild the local index
```

Element tree: header line → command table → options → footer (donation links).

## Elements

| Element | Does | Leads to |
| --- | --- | --- |
| command rows | name one subcommand each; copy is working text, `uiux` is skipped (no visual surface) so this text is final | the screen of that command (`02`-`14`) |
| `--json` | global flag; every command emits one JSON document instead of the human layout | same screen, JSON variant |
| donation footer | two links, Patreon and Buy Me a Coffee (`README.md` § Commercial model) | external |

## States

| State | What the user sees | Trigger |
| --- | --- | --- |
| help | the block above, exit 0 | no subcommand, or `--help` |
| unknown command | one line naming the unknown word and pointing at `quarry --help`; exit code `rule: logic (S13)` | a word not in the table |
