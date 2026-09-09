# Configuration and monorepo targets

Quarry keeps its configuration in **`.quarry/.config` at the source Git worktree root**. Despite the filename, it is a JSON document, not an environment file or TOML. Commit it with `.quarry/.gitignore` so other developers and CI share the same link and target definitions.

## Complete configuration shape

Illustrative configuration:

```json
{
  "default_branch": "main",
  "docs_dir": "docs/capstone",
  "permalink_template": null,
  "targets": [
    {"name": "billing", "docs_dir": "services/billing/docs/capstone"},
    {"name": "orders", "docs_dir": "services/orders/docs/capstone"}
  ],
  "sync_on_read": true,
  "url": "git@github.com:acme/docs-quarry.git"
}
```

| Key | Type and default | Meaning |
|---|---|---|
| `url` | Required string | Shared docs repository clone/push URL. Prefer the `.git` suffix with the bundled GitHub workflow. |
| `docs_dir` | Required string in stored JSON; `init` defaults to `docs/capstone` | Source docs directory relative to the source root. With targets, also the optional umbrella docs directory. |
| `default_branch` | Required string in stored JSON; derived by `init` | Source remote branch whose reachable commits may be imported. It is not the branch of the shared docs clone. |
| `permalink_template` | Optional string or `null`; defaults to `null` | Override source-pointer URLs; configured by editing JSON. |
| `targets` | Optional array, defaults to empty | Independently imported docs directories with their own shared-repo folder names. |
| `sync_on_read` | Optional Boolean, defaults to `false` | Attempt a docs-clone refresh before queries and `check`. Edit JSON to enable it. |

`init` pretty-prints JSON and appends a newline. It always emits `permalink_template`, including `null`; empty `targets` and false `sync_on_read` are omitted. The current struct serialization order is `default_branch`, `docs_dir`, `permalink_template`, optional `targets`, optional `sync_on_read`, `url`.

Unknown keys are ignored on load and are lost when `init` rewrites the config. Missing required fields or incorrect JSON types fail loading. A malformed config can prevent any non-help command from running; correct the JSON directly before retrying `init`.

## Flags and environment precedence

The `init` options are:

```text
quarry init [--url URL] [--docs-dir DIR] [--name NAME]
            [--default-branch BRANCH] [--force]
```

| Option | Environment equivalent | Effect |
|---|---|---|
| `--url` | `QUARRY_DOCS_REPO` | Choose or relink the docs repository. |
| `--docs-dir` | `QUARRY_DOCS_DIR` | Set the root docs directory, or the named target's directory when `--name` is present. |
| `--default-branch` | `QUARRY_DEFAULT_BRANCH` | Set source ancestry eligibility branch. |
| `--name` | None | Add a target or update the existing target with that name; requires a docs-dir input. |
| `--force` | None | Permit changing to a different docs URL. |

For these `init` fields, precedence is **explicit flag → environment → existing configuration → derived/default value**. Environment variables can override an existing config on a later `init`; they are not limited to first-time setup. Ordinary `add`, `update`, `check`, and `docs` commands do not read these environment variables as replacement config values.

Input strings pass through trimming; empty values become absent. The default-branch derivation first checks `refs/remotes/origin/HEAD`, then asks `git ls-remote --symref origin HEAD`, then tries existing `origin/main`, `origin/master`, `origin/trunk`, and `origin/develop`, finally falling back to `main`. A saved branch survives later initialization unless overridden.

If neither a URL input nor an existing config is available, terminal stdin triggers `Docs repo URL: `. With nonterminal stdin the command refuses and explains `--url`/`QUARRY_DOCS_REPO`. Provide the URL or commit config for automation; `--json` does not itself disable the terminal prompt.

## Global read policy

```sh
quarry --sync docs list
quarry --offline docs list
```

`--sync` forces a refresh attempt before reads. `--offline` suppresses that attempt even with `sync_on_read: true`. They conflict with each other. Without either, `sync_on_read` decides.

This policy applies to `check` and docs queries **except `docs index`**. `docs index` always stays local. The flags are accepted globally, but `--offline` does not stop the explicit `sync` command or the network operations performed by `init`, `add`, `update`, and `remove`.

## Identity and ownership

A source repository's default folder name is the final path component of its `origin` URL, with `.git` removed; its case is preserved. It is not configurable as a single-repository display name. `--name` defines a monorepo target instead.

For `git@github.com:Acme/Record-Store.git`, the folder is `Record-Store` and the normalized ownership origin is `github.com/acme/record-store`. The owner is the penultimate URL path component. For nested GitLab groups, this implementation therefore keeps only the final group as `owner`, a limitation that matters for ownership and default source URLs. See [Implementation details](12-parser-details-and-implementation-limits.md).

The shared clone's folder name comes from the last component of the **docs URL**. Linking to `git@github.com:acme/docs-quarry.git` creates `.quarry/docs-quarry/`.

## Register targets

```sh
quarry init --url git@github.com:acme/docs-quarry.git \
  --name billing --docs-dir services/billing/docs/capstone
quarry init --name orders --docs-dir services/orders/docs/capstone
```

Each call adds or replaces one target entry. `init` sorts targets by name. Calling it again without `--name` preserves targets; `--docs-dir` without `--name` changes the root docs directory.

After committing the config/docs and merging them to the source default branch:

```sh
quarry add --strict
quarry docs show billing
quarry docs deps orders --upstream
```

`add`, `update`, `remove`, and `check` operate on **all configured targets**. There is no per-command `--target` selector and no `init --remove-target` option. One target without an umbrella still changes `add`, `update`, and `remove` JSON `result` into an array. `check` keeps an object and includes `target` on contract and break rows.

## Valid names and paths

A target name must be a nonempty single path segment. It cannot be `.`, `..`, contain `/` or `\`, equal the source repo's own name, or equal root-owned `00-index.md` or `observed-edges.json`.

Docs paths must be nonempty relative paths inside the source repository. Absolute paths, leading separators, Windows drive/UNC forms, and any `..` path component using either separator are refused. The normalizer trims surrounding whitespace and trailing `/`, then removes one leading `./`.

Two targets cannot share a name or normalized docs path. A target docs path cannot equal the root docs path. Targets cannot nest inside one another. A target **may** sit inside the root docs directory; the umbrella copy then omits that subtree to prevent duplication.

Use conventional forward-slash paths and ordinary directory names. Validation is not a complete cross-platform filesystem-name validator. Hand-edited configs are validated by `init` and target-consuming operations; config loading itself only deserializes JSON.

## Optional umbrella folder

When targets exist and the root `docs_dir/00-index.md` exists in the working tree, Quarry also imports the root docs folder under the source repo's own name. It imports all its files except target subtrees; it is not restricted to copying only `00-index.md`.

In the umbrella's `00-index.md`, supported inline links into configured target docs are rewritten to sibling folders:

```markdown
[Billing](../../services/billing/docs/capstone/00-index.md#overview)
```

becomes, illustratively:

```markdown
[Billing](../billing/00-index.md#overview)
```

The resolver tries a path relative to the root docs directory, then the path as written from the source root. It preserves fragments and supported quoted link titles, leaves reference-style link definitions alone, and skips fenced code/frontmatter.

Adding a new target can cause an existing umbrella to be rebuilt even at the same source commit: its links depend on the configured/imported target set. A newly registered target needs `quarry add`; `update` refuses any configured folder that is not yet registered.

With no root index, the umbrella is omitted from imports. Removing the root index does not automatically delete a previously imported umbrella folder. `check` always skips the umbrella.

## Moving, renaming, and retiring targets

A stamp claims a folder for a normalized source origin **and a docs directory**. Changing `billing` from `services/billing/docs/capstone` to another directory in config alone does not transfer ownership; the next import refuses the existing stamp. `update --force` does not bypass this check.

Retirement is explicit. `quarry remove` removes every currently configured target, so it is not a single-target retirement command. For a planned whole-repo relayout, remove the old configured set first, change and commit the config, then add the new set. For a selective target retirement, maintainers can make a reviewed change in a separate shared-repository clone, deleting the obsolete folder and updating the root catalog, then remove the target from source config. Coordinate its remaining edges and CI before doing so.

Deleting an entry from `targets` does not garbage-collect that target's old shared folder. Renaming a target can leave both old and new folders until the old one is explicitly removed.

## Relink a repository

```sh
quarry init --url git@github.com:acme/new-docs.git --force
```

URL comparison is literal: changing an SSH spelling to HTTPS can require `--force` even when they address the same remote. Relinking deletes the previous managed clone, rewrites config/ignore files, clones the new URL, and rebuilds. It does not migrate old imported folders into the new remote. If cloning fails, config may already point to the new URL; resolve credentials or URL, then rerun `init`.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [src/config.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/config.rs) [src/cli.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/cli.rs) [src/context.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/context.rs) [src/identity.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/identity.rs) [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [src/importer.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/importer.rs) [tests/s12_init.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s12_init.rs) [tests/s17_targets.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/tests/s17_targets.rs)
