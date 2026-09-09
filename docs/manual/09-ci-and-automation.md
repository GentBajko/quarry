# CI, publishing workflows, and repository audits

Quarry ships two GitHub Actions templates: a reusable import workflow and a shared-repository audit. They are files to copy and configure; `quarry init` does not install workflows, provision a GitHub App, configure secrets, or create branch protections.

**Release status, verified 9 September 2026:** no Quarry GitHub release assets are published. The stock reusable template installs `v0.2.0` from a release URL and will currently fail at that step. Adapt it to the pinned source build below before enabling callers, or wait for an actual published release. A version string in Cargo.toml is not a release artifact.

Use a distinct phase for each job: check working-tree contract docs before merge; publish a committed source snapshot after merge; audit coverage/ownership in the shared docs repository.

## Before enabling publishing

Initialize each source repository locally, commit `.quarry/.config` and `.quarry/.gitignore`, and register its current docs with `quarry add`. The import template runs `update`, which refuses an unregistered folder.

Choose a source default branch and make it consistent in three places: your actual Git branch, Quarry `default_branch`, and the workflow's push trigger. The source checkout should have full history; stamps depend on source commit ancestry.

For GitHub App publishing, give the App access to the docs repository with repository Contents write permission. Store `QUARRY_APP_ID` and `QUARRY_APP_PRIVATE_KEY` as organization secrets visible to the source repositories that call the reusable workflow. Configure the docs repository to allow those repositories to call its reusable workflow.

These are GitHub administration tasks, separate from the CLI's runtime configuration. Quarry relies on Git credentials already arranged for each source fetch and docs push.

## Reusable import workflow

Copy the pinned source `templates/quarry-update.yml` to the **docs repository** as `.github/workflows/quarry-update.yml`. Its inputs are:

| Input | Required/default | Meaning |
|---|---|---|
| `docs-repo` | Required string | GitHub `owner/name` of the shared docs repo. |
| `quarry-version` | Default `v0.2.0` | Published release tag that provides `quarry-installer.sh`. |
| `strict` | Default `true` | Pass `--strict` on import. |

Required reusable-workflow secrets are `app-id` and `app-private-key`.

Illustrative caller in a source repo's `.github/workflows/quarry.yml`:

```yaml
name: Publish Quarry docs
on:
  push:
    branches: [main]
permissions:
  contents: read
jobs:
  quarry:
    uses: acme/docs-quarry/.github/workflows/quarry-update.yml@main
    with:
      docs-repo: acme/docs-quarry
      quarry-version: v0.2.0
      strict: true
    secrets:
      app-id: ${{ secrets.QUARRY_APP_ID }}
      app-private-key: ${{ secrets.QUARRY_APP_PRIVATE_KEY }}
```

Replace `acme/docs-quarry` and the ref with your organization/repository and reviewed workflow ref. A pinned commit ref gives a reproducible workflow implementation; the separate `quarry-version` pins the binary installer.

The supplied job:

1. Checks out the calling source repo with `actions/checkout@v4` and `fetch-depth: 0`.
2. Derives the docs repo's bare name from `owner/name`.
3. Uses `actions/create-github-app-token@v1` to mint a token scoped to that docs repository.
4. Configures Git URL rewrites for the docs URL alone, leaving source-origin fetch credentials separate.
5. Attempts to install the requested Quarry release; this stock step needs replacing while no release exists.
6. Runs `quarry init` followed by `quarry update --strict` by default.

For a working source-build adaptation, replace the template's **Install quarry** step with this illustrative step on a runner with a Rust toolchain:

```yaml
      - name: Build pinned Quarry source
        run: |
          git clone https://github.com/GentBajko/quarry.git "$RUNNER_TEMP/quarry-src"
          git -C "$RUNNER_TEMP/quarry-src" checkout 78c71fab5f5818662dc02d5867246cec963567a9
          cargo install --path "$RUNNER_TEMP/quarry-src" --locked
```

The adapted installer is pinned by source SHA, so the template's `quarry-version` input no longer controls installation until you restore the release-based step. Ensure Cargo's bin directory is on the runner's PATH. The source build was verified with Rust 1.91.1; provision a tested compiler version in your runner rather than assuming a manifest minimum was tested.

It sets Git author/committer name to the source repository name, with `<repo>@quarry.invalid` addresses. That naming convention feeds the audit described below.

## URL spelling matters

The template routes these exact prefixes through its App token:

```text
https://github.com/<owner>/<docs-repo>.git
git@github.com:<owner>/<docs-repo>.git
```

The `.git` suffix is intentional. A committed Quarry URL without it does not match the template rewrite and can produce an unauthenticated push failure. The template avoids broad suffixless prefix matching that could also redirect another repo with a similar name.

The token is placed into the runner's Git URL-rewrite configuration; it is not stored in committed `.quarry/.config`. Do not print the expanded credential URL in your own workflow logging. `init` takes no explicit `--url` because the committed config is the authoritative spelling and a different spelling could trigger a relink refusal.

The template assumes the App owner is `github.repository_owner`. Cross-organization docs repositories need an adapted token setup. Ensure the selected release tag actually contains the installer asset; the source template's default alone does not create that release.

## Pull-request contract gate

Do not call the publishing job on an unmerged feature commit: imports require default-branch ancestry. Instead, install Quarry and read the shared docs with `check`.

Illustrative job fragment, for a runner whose Git credentials can already read the configured shared docs remote:

```yaml
name: Check Quarry contracts
on:
  pull_request:
permissions:
  contents: read
jobs:
  contracts:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Build pinned Quarry source
        run: |
          git clone https://github.com/GentBajko/quarry.git "$RUNNER_TEMP/quarry-src"
          git -C "$RUNNER_TEMP/quarry-src" checkout 78c71fab5f5818662dc02d5867246cec963567a9
          cargo install --path "$RUNNER_TEMP/quarry-src" --locked
      - name: Read current consumer documentation
        run: quarry init
      - name: Check documented fields
        run: quarry check
```

This is a starting fragment for a runner with Rust/Cargo and its bin directory on PATH, not a complete private-repository authentication setup. A private docs repo requires read credentials scoped for that repo before `init`, for example an appropriately scoped App token and the same narrowly targeted URL rewrites. Arrange how secrets are available for your PR model; Quarry does not solve that policy.

If a job reuses a cache/clone without calling `init`, use this sequence when freshness is mandatory:

```sh
quarry sync
quarry check
```

A failing explicit sync exits 2 and can fail the job. `quarry --sync check` has different semantics: it attempts a refresh but can return a successful locally computed answer with a top-level note if the remote is unreachable.

For saved machine output while preserving the real exit code:

```sh
status=0
quarry --json check > quarry-check.json || status=$?
cat quarry-check.json
exit "$status"
```

This valid shell sequence does not treat `ok: true` as synonymous with exit 0. A breaking completed check has `ok: true` and exit 1. Run Capstone's independent docs/code freshness check beside it when using Capstone; Quarry does not compare coverage fingerprints.

## Strict import policy

The CLI's default is lenient; the reusable import template's default is strict. A caller can set `strict: false` to turn supported site/secret findings into notes, but the imported files are then published unchanged. `mode: prescriptive` only exempts site verification; it does not exempt secret scanning.

Strict mode applies to newly built imports. It does not re-audit an unchanged stamp. If a previous lenient import has findings, fix the source docs, commit/merge a new source snapshot, and import that snapshot under strict mode.

## Nightly audit workflow

Copy `templates/quarry-audit.yml` to the **docs repository's** `.github/workflows/`. The template has a nightly schedule of `17 3 * * *` and a manual dispatch input `since`, default `24 hours ago`.

The `committer-folders` job checks commits in that window. A changed file is allowed when it is:

- Root `00-index.md` or `observed-edges.json`.
- Below `.github/`.
- In the folder named after that commit's committer, case-insensitively.
- In a target folder whose stamp origin's final repo component matches the committer, case-insensitively.

For removed files/folders it checks the current commit's stamp, then the parent commit's stamp. Violations are printed, written to the GitHub job summary, and cause exit 1. Other root files, such as a manually added README, are not automatically exempted by this template.

This is a convention audit over committer text and stamp data. It is not Git server authorization, cryptographic proof of origin, or a replacement for repository permissions and review.

The `unregistered-repos` job mints an organization-read token, lists up to **1000 nonarchived visible repos** with `gh repo list`, and compares their names with immediate shared-repo folders containing `00-index.md`. It excludes the docs repo itself. Missing names are printed and summarized, but do not intentionally fail the job.

Coverage is limited by token visibility and the 1000-result limit. The check compares folder names, not stamp origins. A monorepo with only target folders and no umbrella named after its source repo can be reported as unregistered even though its targets contribute docs. Account for that when adapting the coverage job.

## CI concurrency and cache use

Independent runners with separate source worktrees can publish concurrently through Git's rejected-push/rebuild retry flow. There are only three push attempts; a busy remote can still need a rerun. Do not share one writable `.quarry/` folder between concurrent jobs.

Caching the binary is straightforward. Caching a docs clone/index trades freshness for speed and needs an explicit refresh policy. The index is reconstructible; do not treat a restored `.docs-index.sqlite` as proof that its corresponding clone or remote is current.

After a publishing failure, inspect the exit status and error, then retry after addressing permissions/branch/strict findings. Do not add `--force` to every CI import: it is specifically for a diagnosed diverged or unreachable stamp, not a generic retry or authorization override.

## Source basis

Verified against Quarry **0.2.0**, source revision [`78c71fa`](https://github.com/GentBajko/quarry/tree/78c71fab5f5818662dc02d5867246cec963567a9/). [templates/quarry-update.yml](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/templates/quarry-update.yml) [templates/quarry-audit.yml](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/templates/quarry-audit.yml) [src/commands.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/commands.rs) [src/docsrepo.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/docsrepo.rs) [src/main.rs](https://github.com/GentBajko/quarry/blob/78c71fab5f5818662dc02d5867246cec963567a9/src/main.rs)
