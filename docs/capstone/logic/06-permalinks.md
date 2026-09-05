---
scenario: permalinks
id: S4
surfaces_on: [mockup/04-update.md, mockup/15-quarry-layout.md]
depends_on: [S1, S9]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S4 - permalink rewrite

During an import (S1 step 6), `file:line` pointers in the copied pages become links pinned to the imported commit, so a page in the docs repo points at the code it describes even after that code moves.

## Trigger & preconditions

- Trigger: S1 step 6, on the temp-dir copy, before the stamp is written.
- Inputs: the imported commit sha, the normalized origin (S9), the working tree at that commit.

## Steps

1. Scan every `.md` in the copy, body only (frontmatter untouched).
2. Match backtick-wrapped pointers of the form `` `path:NN` `` and `` `path:NN-MM` ``, where `path` contains at least one `/` or a file extension and `NN`, `MM` are integers.
3. Skip matches inside fenced code blocks.
4. `path` exists as a file in the source tree at the imported commit → replace with `[path:NN](<url>)`, the backticked text kept as the link text. Otherwise leave the match unchanged.
5. `<url>` from the template chosen by the host of `origin`:
   - `github.com` → `https://github.com/{owner}/{repo}/blob/{sha}/{path}#L{line}`; ranges `#L{a}-L{b}`.
   - `gitlab.com` → `https://gitlab.com/{owner}/{repo}/-/blob/{sha}/{path}#L{line}`; ranges `#L{a}-{b}`.
   - any other host → the GitHub form with that host, unless `.quarry/.config` carries `permalink_template` with placeholders `{owner}`, `{repo}`, `{sha}`, `{path}`, `{line}` (and `{line_end}` for ranges), which then wins.
6. `{sha}` is always the full imported commit, never a branch name.

## Branches

| Point | Rule |
| --- | --- |
| pointer path absent at the commit | left as-is |
| pointer inside a fenced block or frontmatter | left as-is |
| already a markdown link | left as-is (no double wrapping) |
| host unknown, no template | GitHub form |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| template missing a placeholder | used as given; a template with no `{sha}` produces unpinned links and is the user's choice |
| path with spaces | percent-encoded in the URL, verbatim in the link text |

## State transitions

None.

## Invariants

- Frontmatter, including `paths_covered` and every `site:` field, is byte-identical before and after the rewrite; the index reads raw paths from it (S6).
- The rewrite is deterministic for `(origin, sha, content)`: two importers of the same commit produce identical bytes (S1's redo relies on it).
- A rewritten link never targets a moving ref.

## Outcomes & side effects

- Pages in the docs repo carry pinned links; readers in a terminal see `[path:NN](url)` (S8 returns bodies as stored).

## Dimensions not in play

- D1, D2 (internal step), D4 (no arithmetic beyond line numbers copied), D5, D6, D7, D8 (runs inside S1's atomic build), D10 (pure local), D11, D13, D14, D15: none.
- D12: nothing hidden.
