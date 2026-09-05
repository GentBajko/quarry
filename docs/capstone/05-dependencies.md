---
generated_date: 2026-09-05
capstone_version: 5.2.1
mode: prescriptive
paths_covered:
  - ":(top)Cargo.toml"
  - ":(top)Cargo.lock"
  - ":(top)dist-workspace.toml"
---
> Prescriptive design intent; code does not exist yet.

# Dependencies

Rust binary crate; every crate below is a recorded decision. Concrete versions, licences, and the researched alternatives are in the capability matrix.

## Dev and tooling

| Package | Role |
| --- | --- |
| Rust stable, edition 2024 (`rust-version` = stable current at each release) | toolchain; `std::process::Command`, `std::fs`, `std::io::IsTerminal` from std |
| `cargo` | build, test, clippy, fmt |
| `cargo-dist` (`dist`) | release workflow generation, cross-platform archives, installers |
| GitHub Actions | CI (fmt, clippy, test on Linux, macOS, Windows), release on tag |

Posture: permissive licences only (MIT, Apache-2.0, BSD, Unlicense); Dependabot monthly for `Cargo.lock`; each new crate is a `changelog.md` entry naming the ladder rung.

## External services

| Service | Role | Connection | Outage behaviour |
| --- | --- | --- | --- |
| `git` ≥ 2.30 on PATH | all repo operations | `gitcmd::Git::run` subprocess | absent → `Refusal` at startup naming the binary |
| the user's git host (GitHub, GitLab, other) | docs repo remote and source `origin` | git over ssh/https with the user's credentials | S11: exit 2; queries keep working from the clone |
| GitHub Releases | distribution of precompiled binaries | installer script / `cargo binstall` / archive download | not a runtime dependency |

No database server, cache, broker, IdP, or monitoring service exists. SQLite is compiled into the binary. Lock-in: none priced above zero; the release workflow and installer URLs are the only host-specific files.

## Stack picks

Capability matrix, `stack` stage 2026-09-05 (re-run after the Rust decision). Version floors are the minimum; `Cargo.lock` pins exact versions.

| Capability | Decision | Version floor | Licence | Pricing | Why |
| --- | --- | --- | --- | --- | --- |
| language / toolchain | Rust stable, edition 2024 | stable current at release | MIT / Apache-2.0 | free | user decision (architecture Q22): precompiled releases, ~ms startup for agent loops, SQLite bundled. Rejected Python (distribution, startup), Go (maintainer preference; cgo for SQLite) |
| CLI parsing | `clap` with `derive`, `env` features | 4.5 | MIT / Apache-2.0 | free | the standard; `--help`, env fallbacks, `ArgGroup`. Rejected `argh`, `lexopt` (no env/help ergonomics), hand-rolled (13 subcommands) |
| database + full text | `rusqlite` with `bundled` (SQLite ≥ 3.4x compiled in, FTS5 on) | 0.38 | MIT | free | retires the FTS5-availability risk; no system SQLite. Rejected `sqlx` (async, heavier), system-linked `rusqlite` (reintroduces the risk) |
| YAML frontmatter | `serde-saphyr` (serde deserializer over the saphyr/granit parser; no tag-driven construction) | 0.0.10 | MIT / Apache-2.0 (verify at build) | free | `serde_yaml` deprecated (2026-01); `serde_yml` deprecated (final release is a shim); `serde-saphyr` passes the full yaml-test-suite, panic-free on malformed input. Rejected `yaml-rust2` (no serde layer; would need hand conversion), `noyalib` (younger) |
| JSON (config, stamp, envelope) | `serde` + `serde_json` | 1.0 | MIT / Apache-2.0 | free | the standard; deterministic key ordering via `BTreeMap`/sorted structs for byte-identical stamps |
| errors | `thiserror` | 2.0 | MIT / Apache-2.0 | free | one enum, `From` impls; rejected `anyhow` (standards: typed variants map to exit codes) |
| temp dirs | `tempfile` | 3.10 | MIT / Apache-2.0 | free | unique dirs, cleanup on drop; std has no `mkdtemp` |
| pointer matching | `regex` | 1.10 | MIT / Apache-2.0 | free | the `path:NN[-MM]` scan (S4); std has no regex |
| timestamps | `jiff` | 0.2 | MIT / Unlicense | free | `Timestamp` RFC 3339 for `synced_at`; rejected `chrono` (jiff is the maintained, panic-averse choice), hand-rolled civil-date code |
| git | system `git` via `std::process::Command` | 2.30 | GPL-2.0 (tool, not linked) | – | ladder rung 4; rejected `gix`, `git2` (credentials and config must be the user's own git's; eight subcommands) |
| distribution | `cargo-dist` → GitHub Releases: archives, `quarry-installer.sh`/`.ps1`, `cargo binstall` metadata | 0.32 | MIT / Apache-2.0 | free | precompiled binaries, no cargo on user machines (user decision). Rejected crates.io-only (needs cargo), Homebrew tap (deferred; trigger: macOS users ask) |
| targets | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc` | – | – | free | the company laptops and runners; Windows best-effort per architecture Q1 |
| tests | `cargo test`; integration via `CARGO_BIN_EXE_quarry` | – | – | free | no test crates; rejected `assert_cmd`, `insta` (std covers it) |
| lint / format | `clippy` (`-D warnings`, `unwrap_used`, `expect_used`), `rustfmt` | toolchain | – | free | `standards.md` |
| CI | GitHub Actions | – | – | free for public repos | the repo lives on GitHub |
| docs host | none | – | – | – | README only; a site deferred (trigger: adoption outside the company) |
| paid services | none | – | – | – | donations are links |

Runtime crate count: 9 (`clap`, `rusqlite`, `serde`, `serde_json`, `serde-saphyr`, `thiserror`, `tempfile`, `regex`, `jiff`). Open: none. Deferred: as listed with triggers.
