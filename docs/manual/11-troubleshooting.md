# Troubleshooting and recovery

Start from the failed command's exit status and whether the output is an error, a successful empty result, or a warning attached to an answer. Many “nothing to compare” and zero-hit cases intentionally exit 0.

For Git failures, repeat the command with `--verbose` to see which Git operation fails. For document/index diagnostics, run `quarry docs index --force`; the cached `index current` response does not retain every warning.

## Setup and configuration

| Symptom | Cause and next action |
|---|---|
| `git not found on PATH` | Install/configure the Git executable available to this process. Quarry shells out to Git using argument vectors. |
| `not a git repository` | Run inside a source worktree, or run `git init` for a new read-only project. A raw docs directory outside Git is not command context. |
| `not initialised here; run quarry init` | No `.quarry/.config` at this worktree's Git root. Run init with the docs URL. |
| `docs repo clone missing; run quarry init` | Config exists but its URL-derived clone is absent/not a Git clone. Run init to recreate it. |
| `.quarry/.config is not readable: ...` | Fix malformed JSON, required keys, or wrong value types. Do not use TOML or `.env` syntax. |
| `no docs repo URL ... (no terminal to ask on)` | Set `--url`, `QUARRY_DOCS_REPO`, or commit a usable existing config. |
| `already linked to ...; use --force to relink` | URL strings differ, possibly only SSH/HTTPS spelling. Reuse the committed URL or intentionally relink with `init --force`. |
| `no origin remote` on a contribution | Add a valid source `origin`; read-only init/queries do not need one. Unsupported origin URL shapes can produce the same context-level refusal. |
| Unexpected repo name or ownership | Repo identity comes from origin URL, not the local directory name. Inspect `git remote get-url origin`. |
| Reinit silently changes config | Check `QUARRY_DOCS_REPO`, `QUARRY_DOCS_DIR`, and `QUARRY_DEFAULT_BRANCH`; environment overrides saved config during init. |

The source default branch and docs-clone branch are independent. `init --default-branch trunk` changes source import eligibility; it does not switch the branch checked out in the shared docs clone.

## Import refusal or unexpected no-op

| Symptom | Cause and next action |
|---|---|
| `no 00-index.md in ...` | A newly registered docs directory needs an index page. Generate/write it, commit it, and merge it to the source default branch. |
| `no <docs_dir> at <sha>` | The working directory may exist locally but is absent at committed HEAD. Commit/merge the docs or correct the path. |
| `commit ... not on origin/<branch>; merge first` | HEAD is not reachable from the fetched configured default branch. Merge/push normally, check out the intended reachable commit, or correct `default_branch`. `update --force` does not bypass this. |
| `<repo> is not in the docs repo; run quarry add` | Use add for first registration or newly configured targets. The template's update does not auto-register them. |
| `current` although local docs changed | Imports read Git HEAD, and the stamp is already that commit. Check/commit/merge the edits before updating. |
| `current` after changing permalink template | Same-source-SHA imports are skipped. Make a new source commit for a fresh import; `--force` does not force same-stamp rebuilding. |
| `docs repo already holds a newer commit, skipped` | Your HEAD is an ancestor of the imported stamp. Update your source checkout or leave the newer shared docs in place. |
| `stamp ... not reachable ... history rewritten?` | Quarry could not find the old source SHA, including after trying to fetch it. Restore history/access or, after verifying the intended replacement snapshot, use `update --force`. |
| `stamp ... diverged ...` | Old and current source histories are not ancestors of each other. Diagnose the rewrite; `--force` permits that specific overwrite. |
| `name ... already used by ... at ...` | Folder ownership stamp differs in origin or docs directory. Correct config/identity or explicitly retire/re-register the old folder; force does not transfer ownership. |
| Several targets fail together before publication | All pending units are built/gated before folder replacement. Fix the failing target and retry the whole command. |

Changing only source code can still produce a new docs import because the source stamp changes. Conversely, changing a working file without making a new eligible commit does not.

## Strict findings

A site note means only that the referenced source path is absent from the imported commit. Check workspace-relative bases, spelling/case, and whether the file was actually committed. A valid `:10-20` suffix is removed before checking the path; Quarry does not validate line numbers.

For `mode: prescriptive`, planned sites are not verified. Do not set that flag on descriptive pages merely to disguise missing evidence. For intentionally illustrative source-pointer text in Markdown, fences prevent permalink rewriting; declaration site verification is driven by parsed edge metadata/tables.

A secret-shaped finding reports a file and pattern label without printing the value. Remove or replace the matching value in source docs, commit/merge, then retry strict import. Lenient imports copy matching content unchanged. Strict is not retroactive on a `current` stamp.

## Empty or confusing query results

| Symptom | Explanation and next action |
|---|---|
| A newly added other repo is absent | Your clone may predate its import. Run `quarry sync`, then list. |
| Alias used as command repo says unknown | CLI arguments require the exact canonical folder. Read `docs list`; aliases only resolve edge endpoints. |
| Imported folder absent from list | Registration scanning requires immediate `<folder>/00-index.md` in the docs clone. Check that it was committed in source before import. |
| A copied image is absent from files list | `docs list REPO` lists indexed Markdown, not every imported file. |
| A visible phrase does not search | Frontmatter and text before the first heading are not FTS content; pages over 5 MiB have no indexed sections; a multiword input is one phrase. |
| Search matches but JSON snippet lacks the term | Heading matches are eligible, but snippets come from the section body. |
| Section returns only a table or `Model:` line | Section boundaries stop at every heading and do not expand linked/model docs. Retrieve the child/model heading separately. |
| Section is ambiguous | Exact normalized duplicate headings or several prefix matches exist. Use a more specific unique heading; there is no file selector. |
| “Nearest” does not suggest a one-letter typo | Suggestions score common leading characters, not edit distance. |
| No dependency edge from a visible interface table | Check declaration precedence, required headers, and exact `09-interfaces.md` basename; force the index to see parse warnings. |
| Explicit edge marked missing | Fix the endpoint canonical name/alias or register the absent repo. |
| Two open producers have the same contract key | Name join intentionally leaves the consumer ambiguous; add an explicit endpoint when known. |
| `to: unknown` does not join | It is an explicit publication sentinel. Omit the endpoint to request name joining. |
| More printed repo names than `deps` repo count | By-name leads are excluded from counts and traversal; repeated names count once. |
| A reverse path is displayed | No forward path exists, so Quarry searched in the opposite directed order. |
| Observed-only duplicate of a declared route | Match exact normalized endpoints, lowercased kind, and name spelling; observed names are not route-normalized. |

A `site unverified` mark can originate at either endpoint and is keyed by contract kind/name, not by a particular far-end repo. Inspect both source stamps/docs when diagnosing it.

## Contract checks that pass unexpectedly

Read the complete check output, not just `no breaks`. Notes about unregistered producers, no consumers, missing sections/models, or no fields indicate absent comparison data. Warnings about missing payloads can also mean a comparison was skipped.

Check requires producer working-tree `09-interfaces.md`, uses the indexed consumer graph, and reads consumers from the imported clone. A frontend/client not registered in Quarry is invisible. Observed-only edges and by-name leads do not create field checks. A schema-only consumer needs its imported `02-models.md` and a valid heading/table.

Field names compare exactly. Type comparisons happen only when both sides provide types. Required flips only warn. Enum values in notes, nested schema semantics, and runtime serialization are outside this implementation.

Deleting a producer contract known only through joined or consumer-only edges can yield notes instead of breaks. Changing route parameter names can join graph ends while inline payload headings still fail their simpler exact/parenthesized matching. Standardize contract headings or use valid schema links, and treat missing-input notes as review work rather than compatibility proof.

## Cache and clone recovery

```sh
quarry sync
quarry docs index --force
```

This refreshes the remote snapshot, then rebuilds every index table and diagnostic report. A corrupt/missing/old-schema database is normally rebuilt automatically. Explicit sync can fail if the remote is inaccessible; ordinary reads remain available from an existing usable clone.

Do not maintain authored changes inside `.quarry/<docs-repo>/`. Refresh may discard local commits, modified files, and untracked files. If something there matters, recover it using Git/your backup before running a refresh, and move legitimate shared-file work to a separate docs-repository checkout.

If you forced an index of manually changed clone content, a later sync to the same HEAD may not invalidate that cache. Run `docs index --force` after restoring the clean clone.

## Git and CI failures

| Symptom | Cause and next action |
|---|---|
| Fetch/push authentication failure | Inspect Git credentials for the relevant remote. Quarry uses Git's authentication; it has no separate login command. |
| Commit identity unknown | Configure Git name/email, or the CI author/committer environment. Quarry does not invent local commit identity. |
| CI installer returns 404 | As of 9 September 2026 no GitHub release assets are published. Use the pinned source-build adaptation in the CI chapter; later, choose a tag that actually carries an installer. |
| Source fetch works, docs push fails in template | Check App scope, Contents write permission, docs repo access, and `.git` URL suffix matching. |
| `docs repo busy, retry` | Three push attempts exhausted. Retry from a separate/serialized worktree after contention eases. |
| Git lock/SQLite temp/rename failures during simultaneous commands | Commands shared one managed clone/cache. Serialize or use independent worktrees. |
| `--offline update` still contacts remotes | Offline is a read-policy override, not a global network kill switch. |
| `--sync check` succeeds after remote failure | Designed fallback; inspect top-level notes or run explicit sync first for a network-required CI gate. |
| JSON consumer cannot parse a usage error | Clap errors precede the JSON renderer. Validate command syntax and capture stderr/exit code separately. |

For a bug report, include Quarry version, OS, command, exit status, relevant redacted config/remote shape, and whether it reproduces in a separate clean worktree. Identify source-stamp commit separately from docs-repo `built_at_commit`; conflating them makes ancestry/cache problems hard to diagnose.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/config.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/config.rs) [src/context.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/context.rs) [src/gitcmd.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/gitcmd.rs) [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [src/importer.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/importer.rs) [src/check.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/check.rs) [src/query.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/query.rs) [src/index.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/index.rs) [src/main.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/main.rs) [templates/quarry-update.yml](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/templates/quarry-update.yml)
