# Parser details and implementation limits

This appendix records behavior visible in source revision `78c71fa`, including behaviors that may change in a future release. It is meant to explain surprising results, not to encourage relying on parser accidents or weak validation as a document format.

## CLI and context details

- Global flags are parsed for all verbs, but only their receiving command logic gives them meaning. `--sync docs index` never pulls; `--offline sync` still refreshes; `--offline update` still fetches/pushes.
- Most commands build context by parsing `.quarry/.config` before running. Invalid config can therefore block a command such as `init --force`; force is not a malformed-config recovery parser.
- Bare root/docs help is returned before context is built. Clap help/version also short-circuit context, but follow a different output path.
- Origin parsing failures are discarded while building context. A later writer can say `no origin remote` even when an origin exists but its URL could not be parsed into the expected identity.
- `init` derives a default branch before resolving the saved config/flag; branch discovery may contact source origin even when a branch value is already available.
- JSON output can be preceded by an interactive URL prompt if terminal stdin triggers it. Supply config/URL for automation and do not assume the integration test's nonterminal behavior describes terminals.
- Failure to write/flush stdout exits 2; the program cannot guarantee a second JSON error document on an already-broken stdout stream.

## Identity and path edge cases

The source identity parser handles SSH/scp, `ssh://`, HTTP(S), `git://`, `file://`, and absolute/dot-prefixed local paths. It converts backslashes to forward slashes first, strips a trailing `.git`, takes the last two nonempty path components as owner/repo, and lowercases host/owner/repo for the ownership key. The display repo name preserves case.

Authority parsing drops a port in scheme URLs and any user info before `@`. Nested namespace components beyond the final owner/repo are not kept. Distinct GitLab subgroup repos with the same final two components can therefore normalize to the same origin, and built-in permalinks can omit their outer groups. Local file origins use `localhost` and the final parent/name; the whole local absolute path is not an ownership identity. This parser is a naming convention, not a universal canonical Git URL implementation.

Clone naming is a separate “last URL component” operation. There is no configured clone path or branch flag, no provider API lookup, and no authentication based on the stamp identity. Use ordinary remote URLs without query strings for predictable path derivation.

Docs path validation refuses traversal/absolute forms but is not full canonicalization. It strips one leading `./` and trailing forward slashes; it does not collapse every embedded `.` component, resolve symlinks, normalize Unicode, or apply every platform's reserved-name restrictions. The string `.` can still pass its nonempty relative-path validation. Target-name validation does not exhaustively reject every hidden/platform-special directory name. Prefer simple target names and canonical forward-slash docs paths.

For manually edited configs, `init` tidies/validates more than basic `config::load`, which only deserializes. The no-target unit path skips the target-validation function in import/remove/check. Do not treat comments claiming universal per-load validation as a stronger guarantee than the actual call path.

Git file enumeration uses line-based `ls-tree --name-only` rather than NUL-delimited raw filenames. Unusual filenames containing newlines or Git-quoted characters can fail round-tripping. Imported regular files do not preserve executable permission bits, symlink mode, or Git object mode as special filesystem objects; the importer reads blob content and writes files. A source symlink's blob is copied as link text rather than recreated as a symlink.

## YAML, table, and heading boundaries

| Detail | Consequence |
|---|---|
| Frontmatter must start at byte zero with `---` and a newline. | A BOM or leading blank line prevents frontmatter recognition. |
| YAML mapping/null is accepted; other top-level types warn. | Bad metadata does not necessarily remove searchable body content. |
| YAML names/keys are case-sensitive. | `generated_date` is recognized; `Generated_Date` is not. |
| `string_field` accepts JSON/YAML numbers as strings. | Numeric kind/name/site/scalar endpoint inputs can parse; arrays are stricter. |
| Valid `edges: {}` suppresses every other form. | Visible Markdown tables may intentionally contribute no graph edges. |
| Legacy key presence suppresses tables even if its value is invalid. | Read forced rebuild warnings when declarations vanish. |
| Table selection is the first unfenced line starting with `|`, followed by another pipe line containing `-`. | It is a heuristic, not complete Markdown table validation. |
| Table reading stops at the first nonpipe row. | Blank lines terminate the table; a second table is not merged. |
| Link-cell cleanup extracts text before the first `](`. | The link label supplies the endpoint; reference links and complex nested Markdown are not fully parsed. |
| A lone dash is empty in interface cells, not necessarily in payload Field cells. | A payload row naming `-` can be treated as a literal field. |
| Indentation is trimmed before heading recognition. | An indented `# Heading` may become a section unless fenced. |
| Setext headings (`Title` followed by `===`) are not recognized. | Use ATX `# Title` headings for searchable sections. |
| Closing heading hashes are not stripped. | `## Storage ##` has literal trailing `##` in its normalized heading. |
| Trailing `{#anchor}` is removed from the normalized key, but original heading is kept. | Retrieval can match `Storage` while output retains its explicit anchor. |
| Every recognized heading closes the prior section regardless of level. | Parent sections never contain descendants' content. |

Section and table fence detection recognizes lines starting with triple backticks or triple tildes. It is deliberately simpler than all CommonMark fence-length/indentation rules. If docs contain complex nested fences, verify retrieval with a recorded query rather than assuming a Markdown renderer and Quarry split it identically.

## Graph details that affect interpretation

Explicit edge merging is keyed by resolved from/to and exact lowercased kind plus name spelling. Name joining normalizes route-like names, but only for open declarations. Observed rows use the exact name. These three mechanisms do not apply identical normalization.

The producer's spelling names a joined edge. Explicit producers are not candidate open producers, and explicit consumers do not participate in the open join. A no-match open consumer is silently unjoined; it is not the same diagnostic category as an explicitly named missing repo. An explicit consumes `unknown` can remain a missing edge while `unknown` is suppressed from unresolved reporting.

`via` lists declaration pages, not individual source rows or line numbers. A site failure stamp stores only `kind name`, so it can mark all same-contract edges for that stamped endpoint, regardless of which specific declaration had the missing site.

Declared `known_as` metadata includes original-case claims even if the registry rejected them as ambiguous. Read the aliases table/forced warnings to distinguish declared aliases from accepted aliases. Exact folder names always remain authoritative.

By-name leads use FTS restricted to exact root path `09-interfaces.md`; nested files with that basename can declare edges but do not contribute this particular lead search. Leads can be mentions anywhere in that page's searchable sections, not only a Consumes table.

Dependency `cycle` flags track the traversal's recorded parent chain. They are useful loop annotations, not an enumeration of every possible graph cycle. Missing endpoints are not expanded but may appear in the distinct counterpart count. Path traversal contains no by-name leads and never alternates directions.

## Contract parser asymmetries

- Producer current rows match indexed contracts by exact kind/name, even when their graph origins involved aliases or route normalization.
- Inline consumer subsection matching does not apply route-parameter normalization. A graph can join `GET /users/{id}` with `get /users/:userId` while the inline payload section still fails to match the producer-spelled heading. The consumer schema-row lookup uses a more forgiving route-normalized comparison.
- Consumer schema lookup matches by normalized name without filtering by kind. Reusing the same name across kinds on one consumer page can select the first schema row.
- Contract section headings also do not include kind in their lookup key. Keep headings unambiguous when multiple protocols share names.
- The first matching contract heading/table wins; duplicate field names are not schema errors. Producer field lookup uses the first exact matching name; duplicate consumer fields can create repeated findings.
- `Model:` scanning is a line-based recognizer and is not itself fence-aware. A model-looking line inside a section's example fence can still supply a model name when there is no overriding inline field table/schema.
- A missing entire producer contract creates removal breaks only when an old explicit producer/both declaration backs that indexed contract. Joined-only deletion and consumer-only deletion become notes, as explained in the checks chapter.
- `check` considers root `09-interfaces.md`/`02-models.md` in each imported consumer folder. A nested interfaces page may contribute graph edges but its payload is not automatically retrieved by the check.

## Exact secret detector patterns

The following are **pattern definitions**, not example credentials. The scanner searches the entire copied text, including frontmatter and examples; each pattern is reported at most once per file regardless of match count.

```text
aws-access-key  AKIA[0-9A-Z]{16}
github-token    gh[pousr]_[A-Za-z0-9]{36,}
slack-token     xox[abprs]-[A-Za-z0-9-]{10,}
stripe-key      sk_(live|test)_[A-Za-z0-9]{16,}
google-api-key  AIza[0-9A-Za-z_-]{35}
private-key     -----BEGIN [A-Z ]*PRIVATE KEY-----
```

Patterns are unanchored regular expressions and do not verify whether a matching value is active. They omit many secret families and can match realistic-looking examples. Non-Markdown bytes are scanned through a lossy UTF-8 conversion, then copied as original bytes. Markdown content is rendered from that lossy conversion, so invalid UTF-8 in a `.md` blob can be replaced in the imported output.

## Refresh, retry, and reporting gaps

The shared clone is initially shallow (`--depth 1`). Its refresh fetches the currently checked-out docs branch, hard-resets to `FETCH_HEAD`, and cleans untracked files. When local-commit counting in a shallow clone suggests divergence, it may deepen history by 50 commits to avoid miscounting behind-remote history as local work.

If fetch stderr contains `couldn't find remote ref`, refresh treats that as the empty/missing-branch case and returns without a reset. The caller may still record a sync timestamp. Docs branch selection is the current symbolic branch, falling back to `main` for detached HEAD; there is no separate configured docs branch.

Push conflict classification is a stderr substring check for `rejected` or `fetch first`; it can treat a hook/permission refusal using that wording as a retryable conflict. It attempts no randomized backoff. The loop refreshes and rebuilds even after the third rejected push before returning `docs repo busy, retry`, so failure can leave a rebuilt dirty managed clone ready to be discarded/refreshed on the next run. Nonconflict push errors attempt cleanup refresh; cleanup failure can replace the original error message.

A successful push precedes the local index rebuild and advisory contract reads. A later filesystem/SQLite error can make the command exit 2 even though its remote docs commit was already published. Inspect the remote stamp before treating every failed command as an unpublished change.

Filesystem folder replacement removes the old directory before renaming the build into place. A failure between those steps can leave an incomplete local clone. There is no cross-folder rollback, cross-process lock, or durable journal covering all operations. Git's pushed commit remains a separate boundary from these local filesystem steps.

A rebuild scans working files but keys freshness only by Git HEAD/schema. Ordinary diagnostics are not cached, and an unchanged cache report can show zero rebuild counters despite populated tables. Manually forcing dirty clone content into an index can leave it associated with the same HEAD even after sync restores clean files; force another rebuild.

Dates are unvalidated metadata strings. Newest-doc and last-seen choices compare lexically; inconsistent timestamp offsets/precision can make that differ from chronological order. The human age note is based only on parseable local `synced_at`, and warnings do not change query exit status.

The repository includes an **ignored performance test** that builds an index page plus 5,000 generated chapter pages and asserts a forced rebuild completes in under 10 seconds. That is a test definition, not a published benchmark or a guaranteed production estate limit. It was not run for the recorded guide fixture. Normal local verification here covered the four-repo example and a build using the locked dependencies.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/cli.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/cli.rs) [src/context.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/context.rs) [src/config.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/config.rs) [src/identity.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/identity.rs) [src/frontmatter.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/frontmatter.rs) [src/importer.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/importer.rs) [src/query.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/query.rs) [src/index.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/index.rs) [src/check.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/check.rs) [src/docsrepo.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/docsrepo.rs) [src/secrets.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/secrets.rs) [tests/perf.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/perf.rs)
