# Contributing

Spirolo is one Rust binary (see [ADR 0001](docs/adr/0001-single-rust-binary.md) and
[the architecture](docs/architecture.md)). Bugs, false positives and feature requests go
through the [issue forms](.github/ISSUE_TEMPLATE/); security reports go through
[SECURITY.md](SECURITY.md).

For usage questions, see [SUPPORT.md](SUPPORT.md). For a small starting task, look
for open issues labelled `good first issue` in this repository. Participation follows
the [code of conduct](CODE_OF_CONDUCT.md).

Before a large change (a new rule, a new output format, a change to how a rule is measured),
open an issue to propose it first, so the approach is agreed before the code is written.

## The dev loop

You need the stable Rust toolchain (`rust-toolchain.toml` adds rustfmt and clippy), Node 20 or
newer with npm, [`just`](https://github.com/casey/just) and
[`cargo-nextest`](https://nexte.st). Updating snapshots also needs
[`cargo-insta`](https://insta.rs).

```sh
just --list                                            # every recipe
just ci                                                # what CI runs; run it before pushing
cargo fmt --all                                        # format
cargo clippy --all-targets --all-features -- -D warnings
cargo insta test --accept --all-features               # accept changed snapshots
```

`just ci` builds the workspace from the committed lockfile, runs every Rust test with nextest,
then the doctests, then every committed Node suite (the editor views, the npm launcher and
release metadata, and the workflow checks). Clippy warnings are errors.

Output is pinned with [insta](https://insta.rs) snapshots (`snapshots/*.snap` next to the
tests). `cargo insta test --accept` accepts every changed snapshot, so read each snapshot diff in `git diff`
before committing it: a snapshot that changed for a reason you cannot name is a bug, not an
update. Test fixtures come from real output, never shaped by hand to fit the code.

## Commits

One concern per commit. Subjects follow [Conventional Commits](https://www.conventionalcommits.org)
with an optional issue scope, for example `fix(#542): …`, `feat(website): …`,
`docs(#550): …`. The body says why, and what was measured: counts, timings, the command that
showed the change. Pull requests are merged with merge commits.

## How a rule earns trust

A rule is not trusted because it exists. Each one is measured on an open-source corpus: its
findings are sampled and each is decided true or false by a checker that executes the claim
where it can, and the counts land in [`corpus/rule-status.json`](corpus/rule-status.json), which
the binary embeds. A rule is shown by default (the advisory tier) only above a precision floor,
and it is proven only when it clears a pre-registered bar (a Wilson 95% lower bound of at least
0.9 on the pooled sample, plus a fresh held-out check). Changing a rule means rerunning its
measurement: the method is in [docs/rule-visits/README.md](docs/rule-visits/README.md), the
tiers in [docs/tiers.md](docs/tiers.md), and each rule's record in `docs/rules/<rule>.md`.

## Triage

Every new issue gets a first answer within 7 days: a label from
[`.github/labels.yml`](.github/labels.yml), plus a reply or a request for a reproduction. Fixes
have no deadline. An issue waiting on its reporter for 30 days is closed with a note.

`false-positive` reports come first, because the advisory model depends on them: a rule is
shown by default only while its measured precision clears the floor in
[docs/tiers.md](docs/tiers.md). Such a report also gets its rule label, for example
`rule:weak-assertion`.

## Licence

Contributions are accepted under the [Apache 2.0 licence](LICENSE). Dependency versions are
recorded in [Cargo.lock](Cargo.lock). Third-party dependencies retain their own
licences. Each crate declares its licence or licence-file path in its package
manifest; see the [Cargo manifest reference](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields).
If a pull request changes `Cargo.lock`, say so in its description: a maintainer regenerates
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) before it merges.
