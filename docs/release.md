# Build and stage a release

This guide is for maintainers preparing release artifacts. For installation and
first use, start with the [README](../README.md).

`workspace.package.version` in `Cargo.toml` is the editable version source.
`npm/release.mjs` synchronizes package and extension metadata from it. Run the
commands below from the repository root. Building and staging create local files;
publishing is a separate step in the [publication checklist](release/alpha-checklist.md).

## Prepare the toolchain

Use the Rust toolchain in `rust-toolchain.toml` and the cargo-dist version in
`dist-workspace.toml`. You also need Node and npm for packaging. Install the editor's
locked dependencies before building the VSIX:

```sh
node npm/release.mjs sync
node npm/release.mjs check
npm ci --prefix editors/vscode --no-audit --no-fund
mkdir -p target/rc
npm run package --prefix editors/vscode
```

Review generated changes before committing. VSIX packaging includes runtime
dependencies and excludes development tools and tests. It does not publish to the
extension marketplace.

## Build native artifacts

The configured targets are Linux and macOS on x64 and ARM64. The target list in
`dist-workspace.toml` is the source of truth. Build serially and set
`CARGO_BUILD_JOBS` to a limit suitable for the machine.

For Linux artifacts:

```sh
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
nice dist build --artifacts=local \
  --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu
```

Linux cross-compilation uses cargo-zigbuild and Zig. Install those tools before
building a target that needs them. Archives and checksums go in `target/distrib/`.

Build macOS artifacts on a Mac:

```sh
rustup target add x86_64-apple-darwin aarch64-apple-darwin
nice dist build --artifacts=local \
  --target x86_64-apple-darwin --target aarch64-apple-darwin
```

Copy the resulting archives into the staging checkout's `target/distrib/`.
Staging can also read real Darwin binaries from `cargo zigbuild` output, but a
successful cross-build is not runtime validation. Smoke-test each platform on an
appropriate host. Windows is not in the configured release target list.

## Stage and verify packages

Use a clean staging directory so artifacts from an older version are not included.

```sh
node npm/stage.mjs target/rc
just rc-test
(cd target/rc && sha256sum -c SHA256SUMS)
```

Staging extracts the built native binaries, packs the launcher and platform
packages, and writes `artifacts.json` and `SHA256SUMS`. The manifest lists the
platforms actually staged. The test installs real tarballs offline, checks native
architecture and package resolution, and runs the host launcher.

For a Linux-only rehearsal, use `node npm/stage.mjs target/rc --linux-only`.
This does not validate the omitted macOS payloads. Repeat staging and verification
with every release target before treating the set as complete.

An offline consumer needs both the launcher tarball and the matching platform
tarball. Installing only the launcher can try to resolve its optional dependencies
from a registry. Use the names written by staging; they follow the package metadata.

## Rehearse registry installation locally

Before a version is published, rehearse the install against a local
[Verdaccio](https://verdaccio.org) registry configured with `npm/verdaccio.yaml`: no
upstream proxy, port bound to loopback, fresh storage for each repeated version. Publish
the staged platform tarballs before the launcher, under the dist-tag from
`npm/dist-tag.mjs`, then install them in disposable Linux containers. Docker is
required; ARM64 container checks on another architecture also need emulation. A Linux
container rehearsal does not validate installation on macOS or publication to another
registry.

## Publication

Follow the [publication checklist](release/alpha-checklist.md) using the verified
artifacts and intended repository and registry. This guide makes no claim that a
particular version has been published. Check the destination's release records.
