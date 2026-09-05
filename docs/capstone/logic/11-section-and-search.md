---
scenario: section-and-search
id: S8
surfaces_on: [mockup/09-docs-section.md, mockup/10-docs-search.md]
depends_on: [S5, S13]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S8 - section and search matching

`section` returns one heading's body; `search` returns section-level hits. Neither ever returns a whole file.

## Trigger & preconditions

- Trigger: `quarry docs section <repo> "<heading>"`, `quarry docs search "<term>" [--repo <repo>] [--limit N]`.
- Index current (S5). `<repo>` must exist (exit 1 otherwise).

## Steps

`section`:
1. Candidates: every markdown heading (`#` … `######`) in every page under `<repo>/`, frontmatter excluded, fenced code excluded.
2. Normalize both sides: trim, collapse internal whitespace, case-fold, strip a trailing `{#anchor}`.
3. Exact match on the normalized text. None → prefix match. 
4. One candidate → body: from the heading line to the line before the next heading of equal or higher level, or end of file. Returned as stored (permalinks per S4).
5. Several candidates → list `file § heading` for each, no body, exit 1, JSON `ambiguous: true, candidates: [...]`.
6. None → `no section "<heading>" in <repo>`, up to 5 nearest headings ranked by longest common normalized prefix, exit 1, JSON `candidates` likewise.

`search`:
1. FTS5 table: one row per section (repo, file, heading, text); text is the section body with markdown syntax kept.
2. Query passed to FTS5 as a phrase when it contains whitespace, as a term otherwise; FTS5 syntax characters are escaped, never interpreted.
3. Ranking: bm25. Default limit 20; `--limit N`; `--repo` filters before ranking.
4. Each hit: repo, file, heading, `generated_date`, one-line FTS5 snippet.
5. Scope: every `.md` in the clone except the root `00-index.md`; `changelog.md` included.

## Branches

| Point | Rule |
| --- | --- |
| exact match | body |
| prefix match, unique | body |
| prefix match, several | ambiguous |
| none | not found + nearest |
| search 0 hits | `0 hits`, exit 0, empty array |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| heading argument empty | exit 1 usage |
| same heading in two files, one exact and one prefix | exact wins; no ambiguity |
| section body over 64 KiB | returned whole; no truncation |

## State transitions

None.

## Invariants

- `section` never returns more than one body.
- A hit row is always a valid `section` call (`repo`, `file`, `heading`).

## Outcomes & side effects

- Read-only. Every result carries the page's `generated_date` and `file`.

## Dimensions not in play

- D1, D2, D5, D7, D8, D9, D10, D11, D13, D14, D15: none; read-only local queries.
- D4: bm25 is FTS5's; nothing computed by quarry.
- D6: `--limit` default 20; not a cap.
- D12: nothing hidden.
