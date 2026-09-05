---
scenario: root-index
id: S3
surfaces_on: [mockup/03-add.md, mockup/04-update.md, mockup/14-remove.md, mockup/15-quarry-layout.md]
depends_on: [S1, S6]
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
---
# S3 - root index regeneration

The docs repo's root `00-index.md` is a pure function of the folders present; every write command regenerates it whole.

## Trigger & preconditions

- Trigger: step 8 of S1 (`add`, `update`) and step 3 of `remove` (S2), inside the temp-build before the commit.
- No actor input; no authority beyond the write itself.

## Steps

1. Enumerate top-level folders in the clone that contain `00-index.md`. Any other folder or file at the root is ignored and left alone.
2. Per folder compute: Imported = first 7 characters of the commit in `.quarry-stamp` (`-` when absent); Newest doc = maximum `generated_date` across the folder's `.md` frontmatter (`-` when none); Pages = count of `.md` files recursively; Produces / Consumes = counts of valid edge entries per S6.
3. Sort rows by folder name, byte order.
4. Write the file: a fixed title (`# <docs repo name>`), one table with columns Repo (relative link `[name](name/00-index.md)`), Imported, Newest doc, Pages, Produces, Consumes. No timestamps, no run metadata, no trailing whitespace.
5. Empty docs repo → title plus the header row only.

## Branches

| Point | Rule |
| --- | --- |
| folder without `00-index.md` | not listed |
| frontmatter unparsable in a page | page counted in Pages, contributes no date or edges |

## Unhappy paths

| Case | Behaviour |
| --- | --- |
| two writers regenerate at once | identical bytes for identical folder state; the push race is S1's |
| root file hand-edited | overwritten on the next write without warning |

## State transitions

None; the file has no state of its own.

## Invariants

- Given the same set of folders and their contents, the regenerated file is byte-identical on any machine.
- The file never references a folder that does not exist at commit time.

## Outcomes & side effects

- Included in the same commit as the import or removal; never committed alone.

## Dimensions not in play

- D1, D2, D3: internal step, no actor, no input.
- D5 money, D6 limits, D7 time (dates are copied, never compared to now), D10 failure (pure local computation), D11, D13, D15: none.
- D12: nothing hidden; the file shows every listed folder.
