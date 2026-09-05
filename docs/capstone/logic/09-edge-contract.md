---
scenario: edge-contract
id: S6
surfaces_on: [mockup/08-docs-show.md, mockup/11-docs-deps.md, mockup/15-quarry-layout.md]
depends_on: [S9, S5]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S6 - edge contract

Cross-repo edges are read from two frontmatter keys. Quarry publishes the shape; any generator (Capstone's `09-interfaces.md` is the intended one) may write it.

## Trigger & preconditions

- Trigger: index rebuild (S5 step 3) reads frontmatter from every page under `<repo>/`.
- No actor; the contract is checked at index time only.

## Steps

1. Two accepted forms per page, checked in order:
   - **Frontmatter keys** `produces` and `consumes`, each a list, on any `.md` under a repo folder.
   - **Tables**, when the page carries neither key and its filename is `09-interfaces.md`: the rows under a `## Produces` or `## Consumes` heading, columns matched by name (`Kind`, `Name`, `To`/`From`/`Repo`, optional `Site`; `Endpoint` is accepted for `Name`). Cells are read after stripping backticks and markdown link syntax, so `[record-store](../record-store/09-interfaces.md)` yields `record-store`. `\|` inside a cell is a literal pipe. Tables inside fenced blocks are never read, and neither is any table on another page: a chapter documenting the format declares nothing.
   - A page with both keys and tables uses the keys, so a generator that emits only one form never has a mirror to drift.
2. Entry fields (frontmatter form; the table columns carry the same set):

   | Field | Required | Meaning |
   | --- | --- | --- |
   | `kind` | yes | free-form string, lowercased at index time (`sqs`, `http`, `grpc`, `kafka`, …) |
   | `name` | yes | the queue, topic, endpoint, or contract name, verbatim |
   | `to` (produces) / `from` (consumes) | yes | the other repo's name per S9 |
   | `site` | no | `path:line` in the declaring repo, raw (S4 leaves frontmatter untouched) |
   | anything else | no | kept in the page's frontmatter JSON, ignored by the edge table |

3. An entry missing a required field, or with a non-list key, is skipped. `docs index` prints one warning per skipped entry (`<file>: produces[2] missing 'to'`); exit stays 0.
4. Edge identity: `(from, to, kind, name)`, direction producer → consumer. A `produces` entry in repo A with `to: B` and a `consumes` entry in B with `from: A` and equal `kind`/`name` are one edge with `declared_by: both`; either alone gives `declared_by: producer` or `consumer`.
5. The same edge declared in several pages of one repo collapses to one; `via` lists every declaring page (repo-relative path).
6. `to`/`from` naming a repo with no folder in the docs repo → the edge is kept with `missing: true`.

## Branches

| Point | Rule |
| --- | --- |
| valid entry | edge |
| missing required field, or a renamed column | skipped + warning naming the field |
| table on a page other than `09-interfaces.md` | ignored, no warning: it is prose |
| table inside a fenced block | ignored |
| page with both frontmatter and tables | frontmatter wins |
| one-sided declaration | edge, `declared_by` set; `deps` human output appends `(declared by <side> only)` |
| other repo missing | edge, `missing: true`; human output `(not in quarry)` |
| `kind` differs in case between sides | same edge (lowercased) |
| `name` differs between sides | two edges; the disagreement is visible in `deps` |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| a repo declares an edge to itself | kept; `deps` prints it at depth 1 with `(self)` |
| frontmatter is not valid YAML | page indexed without frontmatter; warning in `docs index` |

## State transitions

None; edges are recomputed on every rebuild.

## Invariants

- The edge table contains no entry lacking `from`, `to`, `kind`, `name`.
- Edge direction is never inferred from prose: it comes from the frontmatter key, or from the heading the table sits under.

## Outcomes & side effects

- Feeds `deps`, `path` (S7), `show` (edge lines), the root index counts (S3), and `docs list` counts.

## Dimensions not in play

- D1, D2, D5, D6, D7, D8, D9, D10, D11, D13, D14, D15: none; pure parsing inside S5's rebuild.
- D12: every entry, valid or skipped, is visible (skipped ones via `docs index` warnings).
